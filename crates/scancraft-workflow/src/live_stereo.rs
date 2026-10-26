//! Bounded candidate association and incremental reconstruction, independent of USB/UI.
//! Equal raw PTS plus a host-window policy never attests simultaneous exposures.
pub mod runner;
use crate::{Error, Request, bounded};
use scancraft_calibration::storage;
use scancraft_core::{Frame, StreamProfile, uvc_pts, validate_frame};
use scancraft_depth::{
    MatchSource,
    pipeline::{StereoProcessor, StereoReport, StereoRequest},
};
use scancraft_geometry::{
    Cloud, session::ScanSession, tracking::TrackingConfig, work::WorkControl,
};
use scancraft_image::GrayImage;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{HashSet, VecDeque},
    path::Path,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub session_id: String,
    pub fps: u32,
    pub maximum_frames_per_camera: usize,
    pub maximum_host_delta_us: u64,
    pub tracking_samples: usize,
    pub tracking: TrackingConfig,
    pub voxel_mm: f64,
    pub maximum_voxels: usize,
}
#[derive(Clone, Copy, Debug)]
pub enum Side {
    Left,
    Right,
}
impl Side {
    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}
pub struct Sample {
    pub index: usize,
    pub frame: Frame,
}
pub struct Pair {
    pub left: Sample,
    pub right: Sample,
    pub pts: u32,
}
#[derive(Default, Serialize)]
pub struct AssociationCounts {
    pub delivered: [usize; 2],
    pub evicted_pending: [usize; 2],
    pub rejected_host_window_pairs: usize,
    pub paired: usize,
}
/// Two pending frames per side. Invalid/order-ambiguous input aborts, not repairs.
pub struct Association {
    profiles: [StreamProfile; 2],
    limit: u64,
    maximum: usize,
    pending: [VecDeque<Sample>; 2],
    previous: [Option<(usize, u64, u32)>; 2],
    last_pair: Option<[usize; 2]>,
    seen: [HashSet<u32>; 2],
    pub counts: AssociationCounts,
}
impl Association {
    pub fn new(profiles: [StreamProfile; 2], maximum: usize, limit: u64) -> Result<Self, String> {
        if !(1..=1000).contains(&maximum)
            || limit > 1_000_000
            || profiles[0].interface != 0
            || profiles[1].interface != 2
            || profiles.iter().any(|p| {
                p.fourcc != *b"GREY"
                    || p.width == 0
                    || p.height == 0
                    || u64::from(p.width) * u64::from(p.height) > scancraft_image::MAX_PIXELS as u64
            })
        {
            return Err(
                "association requires GREY, 1..1000 frames and host limit <=1000000us".into(),
            );
        }
        Ok(Self {
            profiles,
            limit,
            maximum,
            pending: Default::default(),
            previous: [None; 2],
            last_pair: None,
            seen: Default::default(),
            counts: Default::default(),
        })
    }
    /// An association is only a candidate. Source indices refer to full raw recordings.
    pub fn push(&mut self, side: Side, sample: Sample) -> Result<Option<Pair>, String> {
        let i = side.index();
        let other = 1 - i;
        validate_frame(&self.profiles[i], &sample.frame)?;
        let pts = uvc_pts(&sample.frame.metadata)?.ok_or("missing PTS")?;
        if sample.index >= self.maximum
            || self.previous[i].is_some_and(|(index, host, previous_pts)| {
                sample.index <= index
                    || sample.frame.host_timestamp_us <= host
                    || pts.wrapping_sub(previous_pts) == 0
                    || pts.wrapping_sub(previous_pts) >= (1u32 << 31)
            })
            || self.seen[i].contains(&pts)
        {
            return Err(
                "stream source indices/host times/PTS must advance without repeated values".into(),
            );
        }
        self.previous[i] = Some((sample.index, sample.frame.host_timestamp_us, pts));
        self.seen[i].insert(pts);
        self.counts.delivered[i] += 1;
        if let Some(position) = self.pending[other]
            .iter()
            .position(|p| uvc_pts(&p.frame.metadata) == Ok(Some(pts)))
        {
            let peer = self.pending[other].remove(position).unwrap();
            if sample
                .frame
                .host_timestamp_us
                .abs_diff(peer.frame.host_timestamp_us)
                > self.limit
            {
                self.counts.rejected_host_window_pairs += 1;
                return Ok(None);
            }
            let (left, right) = if i == 0 {
                (sample, peer)
            } else {
                (peer, sample)
            };
            let indices = [left.index, right.index];
            if self
                .last_pair
                .is_some_and(|old| indices[0] <= old[0] || indices[1] <= old[1])
            {
                return Err(
                    "candidate pair order regressed; no temporal reordering is inferred".into(),
                );
            }
            self.last_pair = Some(indices);
            self.counts.paired += 1;
            return Ok(Some(Pair { left, right, pts }));
        }
        if self.pending[i].len() == 2 {
            self.pending[i].pop_front();
            self.counts.evicted_pending[i] += 1;
        }
        self.pending[i].push_back(sample);
        Ok(None)
    }
    pub fn remaining(&self) -> [usize; 2] {
        self.pending.each_ref().map(VecDeque::len)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InputMode {
    PhysicalCapture,
    SyntheticFixture,
}

pub struct Processor {
    config: Config,
    profiles: [StreamProfile; 2],
    calibration_bytes: Vec<u8>,
    request_bytes: Vec<u8>,
    config_bytes: Vec<u8>,
    request: Request,
    stereo: StereoProcessor,
    session: ScanSession,
    attempts: usize,
    last_pair: Option<([usize; 2], [u64; 2], u32)>,
}
impl Processor {
    /// Physical entry rejects synthetic records before any caller opens a device.
    /// Observations are declarations only; actual hardware binding remains unverified.
    pub fn open(
        calibration: &Path,
        request: &Path,
        config: &Path,
        mode: InputMode,
        work: &WorkControl,
    ) -> Result<Self, Error> {
        let calibration_bytes = bounded(calibration, 1024 * 1024 + 16, work)?;
        let calibration = storage::read(calibration_bytes.as_slice())?;
        let request_bytes = bounded(request, 1024 * 1024, work)?;
        let request: Request = serde_json::from_slice(&request_bytes)?;
        let config_bytes = bounded(config, 1024 * 1024, work)?;
        let config: Config = serde_json::from_slice(&config_bytes)?;
        if request.schema_version != 1
            || config.schema_version != 1
            || request.meshing.is_some()
            || request.surface_normals.is_some()
            || !(1..=1000).contains(&config.maximum_frames_per_camera)
            || config.maximum_host_delta_us > 1_000_000
            || !(3..=512).contains(&config.tracking_samples)
            || ![5, 10, 15, 20, 25, 30, 40, 50].contains(&config.fps)
        {
            return Err("live request requires schema1, bounded GREY frame policy, 3..512 tracking samples, enumerated IR FPS and no meshing".into());
        }
        match (&request.source, mode) {
            (MatchSource::Observations(binding), InputMode::PhysicalCapture) => calibration.validate_for_capture(binding)?,
            (MatchSource::SyntheticFixture, InputMode::SyntheticFixture) if calibration.provenance == storage::Provenance::SyntheticFixture => {},
            _ => return Err("physical capture requires declared observations and a nonsynthetic matching calibration; fixtures require explicit synthetic mode".into()),
        }
        let profiles = [
            (&calibration.geometry.left, 0),
            (&calibration.geometry.right, 2),
        ]
        .map(|(camera, interface)| StreamProfile {
            interface,
            width: camera.width,
            height: camera.height,
            fourcc: *b"GREY",
            fps: config.fps,
        });
        let mut session = ScanSession::new(
            config.session_id.clone(),
            config.tracking,
            config.voxel_mm,
            config.maximum_voxels,
            config.maximum_frames_per_camera,
        )?;
        let stereo = StereoProcessor::build(calibration, request.output_grid.clone(), work)?;
        // Validate the complete existing matcher policy before acquisition. Blank
        // test pixels are discarded; they are not observations or published results.
        let left = vec![0; profiles[0].width as usize * profiles[0].height as usize];
        let right = vec![0; profiles[1].width as usize * profiles[1].height as usize];
        Self::reconstruct(&stereo, &request, &profiles, &left, &right, work)?;
        session.start()?;
        Ok(Self {
            config,
            profiles,
            calibration_bytes,
            request_bytes,
            config_bytes,
            request,
            stereo,
            session,
            attempts: 0,
            last_pair: None,
        })
    }
    pub fn source(&self) -> &MatchSource {
        &self.request.source
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn profiles(&self) -> &[StreamProfile; 2] {
        &self.profiles
    }
    pub fn input_bytes(&self) -> [(&str, &[u8]); 3] {
        [
            ("calibration.rapcal", &self.calibration_bytes),
            ("request.json", &self.request_bytes),
            ("scan-request.json", &self.config_bytes),
        ]
    }
    fn reconstruct(
        stereo: &StereoProcessor,
        request: &Request,
        profiles: &[StreamProfile; 2],
        left: &[u8],
        right: &[u8],
        work: &WorkControl,
    ) -> Result<StereoReport, String> {
        let input = StereoRequest {
            left: GrayImage::new(profiles[0].width, profiles[0].height, left)?,
            right: GrayImage::new(profiles[1].width, profiles[1].height, right)?,
            source: request.source.clone(),
            seeds: &request.seeds,
            matching: request.matching,
            triangulation: request.triangulation,
        };
        match request.automatic_sampling {
            Some(s) => stereo.process_automatic(input, s, work),
            None => stereo.process(input, work),
        }
    }
    /// Deterministic evenly spaced source samples feed existing point-to-point ICP.
    /// No-geometry pairs are recorded and never presented as successful tracking.
    pub fn process(&mut self, pair: &Pair, work: &WorkControl) -> Result<Value, Error> {
        work.check()?;
        if self.attempts >= self.config.maximum_frames_per_camera {
            return Err("live pair capacity exceeded".into());
        }
        let indices = [pair.left.index, pair.right.index];
        let hosts = [
            pair.left.frame.host_timestamp_us,
            pair.right.frame.host_timestamp_us,
        ];
        if indices
            .iter()
            .any(|&i| i >= self.config.maximum_frames_per_camera)
            || self.last_pair.is_some_and(|(old, previous_hosts, pts)| {
                indices[0] <= old[0]
                    || indices[1] <= old[1]
                    || hosts[0] <= previous_hosts[0]
                    || hosts[1] <= previous_hosts[1]
                    || pair.pts.wrapping_sub(pts) == 0
                    || pair.pts.wrapping_sub(pts) >= (1u32 << 31)
            })
        {
            return Err("processed candidate pair indices/times/PTS must advance".into());
        }
        validate_frame(&self.profiles[0], &pair.left.frame)?;
        validate_frame(&self.profiles[1], &pair.right.frame)?;
        if uvc_pts(&pair.left.frame.metadata)? != Some(pair.pts)
            || uvc_pts(&pair.right.frame.metadata)? != Some(pair.pts)
            || pair
                .left
                .frame
                .host_timestamp_us
                .abs_diff(pair.right.frame.host_timestamp_us)
                > self.config.maximum_host_delta_us
        {
            return Err("pair does not satisfy explicit raw-PTS/host-window policy".into());
        }
        let report = Self::reconstruct(
            &self.stereo,
            &self.request,
            &self.profiles,
            &pair.left.frame.data,
            &pair.right.frame.data,
            work,
        )?;
        let cloud = report.reconstruction.as_ref().map(|r| r.cloud());
        let mut tracking = json!({"integrated":false,"state":"no_geometry","reason":"fewer than three reconstructed points"});
        if let Some(cloud) = cloud.filter(|c| c.points.len() >= 3) {
            let count = self.config.tracking_samples.min(cloud.points.len());
            let indices: Vec<_> = (0..count)
                .map(|i| i * (cloud.points.len() - 1) / (count - 1))
                .collect();
            let update = self
                .session
                .process(self.attempts as u64, &cloud, &indices, work)?;
            let t = &update.tracking;
            let pose=t.world_from_frame.as_ref().map(|p|json!({"rotation":(0..3).map(|r|(0..3).map(|c|p.rotation[(r,c)]).collect::<Vec<_>>()).collect::<Vec<_>>(),"translation_mm":[p.translation[0],p.translation[1],p.translation[2]]}));
            tracking = json!({"integrated":update.fusion.is_some(),"state":format!("{:?}",t.state),"reason":t.reason,
                "world_from_frame":pose,"rms_mm":t.rms_mm,"source_overlap":t.source_overlap,"target_overlap":t.target_overlap,"tracking_indices":indices});
        }
        let result = json!({"pair_id":self.attempts,"left_index":pair.left.index,"right_index":pair.right.index,
            "raw_pts":pair.pts,"left_host_us":pair.left.frame.host_timestamp_us,"right_host_us":pair.right.frame.host_timestamp_us,
            "synchronization_verified":false,"physical_accuracy_qualified":false,"reconstruction":report,"tracking":tracking});
        self.attempts += 1;
        self.last_pair = Some((indices, hosts, pair.pts));
        Ok(result)
    }
    pub fn finish(&mut self, work: &WorkControl) -> Result<Cloud, Error> {
        self.session.stop()?;
        Ok(self.session.cloud(work)?)
    }
}
