use super::{PENDING_PER_CAMERA, Result};
use scancraft_core::{Frame, StreamProfile, uvc_pts};
use serde::Serialize;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy)]
pub enum Camera {
    First,
    Second,
}
impl Camera {
    pub fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }
}
pub struct Sample {
    pub index: u64,
    pub frame: Frame,
}
pub struct Pair {
    pub samples: [Sample; 2],
    pub raw_pts: u32,
}
#[derive(Default, Debug, Serialize)]
pub struct AssociationCounts {
    pub delivered: [u64; 2],
    pub evicted_pending: [u64; 2],
    pub rejected_host_window_pairs: u64,
    pub paired: u64,
}
struct Pending {
    sample: Sample,
    ticks: i128,
}
#[derive(Clone, Copy)]
struct Previous {
    index: u64,
    host: u64,
    sequence: u32,
    raw_pts: u32,
    ticks: i128,
}
/// Constant-size association state; no session-wide PTS set or frame history.
/// Forward modular increments must be strictly less than half the u32 range.
/// Initial cameras are aligned to the nearest observed peer counter; an exact
/// half-range offset is ambiguous and rejected. Counter unwrapping is a policy,
/// not proof of clock frequency, reset behavior or simultaneous exposure.
pub struct Association {
    profiles: [StreamProfile; 2],
    maximum: u64,
    host_limit: u64,
    previous: [Option<Previous>; 2],
    pending: [VecDeque<Pending>; 2],
    last_pair: Option<[u64; 2]>,
    pub counts: AssociationCounts,
}
impl Association {
    pub fn new(profiles: [StreamProfile; 2], maximum: u64, host_limit: u64) -> Result<Self> {
        if maximum == 0
            || host_limit > 1_000_000
            || !matches!(
                [profiles[0].interface, profiles[1].interface],
                [0, 2] | [2, 0]
            )
            || profiles.iter().any(|p| {
                p.fourcc != *b"GREY"
                    || p.fps == 0
                    || p.width == 0
                    || p.height == 0
                    || u64::from(p.width) * u64::from(p.height) > scancraft_image::MAX_PIXELS as u64
            })
        {
            return Err("invalid bounded laser association configuration".into());
        }
        Ok(Self {
            profiles,
            maximum,
            host_limit,
            previous: [None; 2],
            pending: Default::default(),
            last_pair: None,
            counts: Default::default(),
        })
    }
    pub fn remaining(&self) -> [usize; 2] {
        self.pending.each_ref().map(VecDeque::len)
    }
    pub fn push(&mut self, camera: Camera, sample: Sample) -> Result<Option<Pair>> {
        let side = camera.index();
        let other = 1 - side;
        super::validate_sample(&self.profiles[side], &sample.frame)?;
        let pts = uvc_pts(&sample.frame.metadata)?.ok_or("missing laser frame PTS")?;
        if sample.index >= self.maximum {
            return Err("laser source index exceeds configured limit".into());
        }
        let ticks = if let Some(old) = self.previous[side] {
            let delta = pts.wrapping_sub(old.raw_pts);
            let seq = sample.frame.sequence.wrapping_sub(old.sequence);
            if sample.index <= old.index
                || sample.frame.host_timestamp_us <= old.host
                || delta == 0
                || delta >= 1 << 31
                || seq == 0
                || seq >= 1 << 31
            {
                return Err("laser source index/host/sequence/PTS progression is ambiguous".into());
            }
            old.ticks
                .checked_add(i128::from(delta))
                .ok_or("laser counter extension overflow")?
        } else if let Some(peer) = self.previous[other] {
            let delta = pts.wrapping_sub(peer.raw_pts);
            if delta == 1 << 31 {
                return Err("initial laser counter offset is half-range ambiguous".into());
            }
            peer.ticks
                .checked_add(if delta < 1 << 31 {
                    i128::from(delta)
                } else {
                    i128::from(delta) - (1i128 << 32)
                })
                .ok_or("laser counter extension overflow")?
        } else {
            0
        };
        self.previous[side] = Some(Previous {
            index: sample.index,
            host: sample.frame.host_timestamp_us,
            sequence: sample.frame.sequence,
            raw_pts: pts,
            ticks,
        });
        self.counts.delivered[side] += 1;
        if let Some(position) = self.pending[other].iter().position(|p| p.ticks == ticks) {
            let peer = self.pending[other].remove(position).unwrap().sample;
            if sample
                .frame
                .host_timestamp_us
                .abs_diff(peer.frame.host_timestamp_us)
                > self.host_limit
            {
                self.counts.rejected_host_window_pairs += 1;
                return Ok(None);
            }
            let samples = if side == 0 {
                [sample, peer]
            } else {
                [peer, sample]
            };
            let indices = samples.each_ref().map(|s| s.index);
            if self
                .last_pair
                .is_some_and(|old| indices[0] <= old[0] || indices[1] <= old[1])
            {
                return Err("laser candidate pair order regressed".into());
            }
            self.last_pair = Some(indices);
            self.counts.paired += 1;
            return Ok(Some(Pair {
                samples,
                raw_pts: pts,
            }));
        }
        if self.pending[side].len() == PENDING_PER_CAMERA {
            self.pending[side].pop_front();
            self.counts.evicted_pending[side] += 1;
        }
        self.pending[side].push_back(Pending { sample, ticks });
        Ok(None)
    }
}
