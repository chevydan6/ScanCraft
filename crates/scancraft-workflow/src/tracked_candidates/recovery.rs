//! Recorded recovery adapter. Optical hypotheses are computed once; all selection
//! trials and tracking results remain in bounded, checksummed diagnostic artifacts.
use super::{Error, Request, line, markers};
use scancraft_geometry::{
    Cloud,
    session::{ScanSession, SessionFrame},
    tracking::TrackingUpdate,
    work::WorkControl,
};
use serde_json::{Value, json};
use std::io::{self, Write};

pub(super) const MARKER_BYTES: u64 = 1024 * 1024 * 1024;
pub(super) const JOURNAL_LINE_BYTES: usize = 1024 * 1024;
pub(super) const JOURNAL_BYTES: u64 = 256 * JOURNAL_LINE_BYTES as u64;

/// Refuse oversized diagnostics before writing beyond the reader's quota.
pub(super) struct LimitedWriter<W> {
    inner: W,
    remaining: u64,
}
impl<W> LimitedWriter<W> {
    pub fn new(inner: W, remaining: u64) -> Self {
        Self { inner, remaining }
    }
}
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::other("recorded diagnostic byte budget exceeded"));
        }
        let n = self.inner.write(bytes)?;
        self.remaining -= n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub(super) fn tracking(t: &TrackingUpdate) -> Value {
    json!({"accepted":t.accepted,"reason":t.reason,
        "pose":t.world_from_frame.as_ref().map(|p|json!({
            "rotation":(0..3).map(|i|(0..3).map(|j|p.rotation[(i,j)]).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "translation_model_units":p.translation.as_slice()})),
        "rms_model_units":t.rms_mm,"source_overlap":t.source_overlap,"target_overlap":t.target_overlap})
}

pub(super) struct Processed {
    pub step: Option<SessionFrame>,
    pub points: usize,
    pub error: Option<String>,
    pub summary: Value,
}

pub(super) fn process(
    session: &mut ScanSession,
    frame_id: u64,
    surface: &Cloud,
    prepared: &markers::Prepared,
    request: &Request,
    outputs: (&mut impl Write, &mut impl Write),
    work: &WorkControl,
) -> Result<Processed, Error> {
    let (markers, journal) = outputs;
    let mut index = 0;
    let outcome = session.process_with_reference_recovery(
        frame_id,
        surface,
        |reference, work| {
            line(
                markers,
                &json!({"kind":"recovery_attempt_start","attempt":index,
            "reference_frame":reference.map(|r|r.frame_id)}),
            )
            .map_err(|e| e.to_string())?;
            index += 1;
            prepared
                .select(
                    reference.map(|r| (r, request.tracking.max_correspondence_model_units)),
                    markers,
                    work,
                )
                .map_err(|e| e.to_string())
        },
        work,
    )?;
    let selected = &outcome.attempts[outcome.selected_attempt];
    let summary = json!({"attempts":outcome.attempts.len(),"selected_attempt":outcome.selected_attempt,
        "recovered_from":outcome.recovered_from,"conflicting_references":outcome.conflicting_references,
        "maximum_pose_disagreement_model_units":outcome.maximum_pose_disagreement});
    let attempts: Vec<_> = outcome.attempts.iter().map(|a| json!({
        "reference_frame":a.reference_frame,"retained":a.retained,
        "positions_model_units":a.observations,
        "tracking":match &a.tracking { Ok(t)=>json!({"Ok":tracking(t)}),Err(e)=>json!({"Err":e}) }
    })).collect();
    let row = json!({"schema_version":1,"frame_id":frame_id,"summary":summary,
        "attempts":attempts,"retained_references":session.retained_reference_ids()});
    let bytes = serde_json::to_vec(&row)?;
    if bytes.len() >= JOURNAL_LINE_BYTES {
        return Err("recovery journal row byte budget exceeded".into());
    }
    work.check()?;
    journal.write_all(&bytes)?;
    journal.write_all(b"\n")?;
    line(
        markers,
        &json!({"kind":"tracking_observations","positions_model_units":selected.observations,
        "stereo_attempts":prepared.attempts,"marker_identity_verified":false}),
    )?;
    Ok(Processed {
        points: selected.observations.len(),
        error: selected.tracking.as_ref().err().cloned(),
        step: outcome.frame,
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_quota_retains_partial_bytes_and_refuses_overflow() {
        let mut bytes = Vec::new();
        let mut writer = LimitedWriter::new(&mut bytes, 4);
        writer.write_all(b"abc").unwrap();
        assert!(writer.write_all(b"de").is_err());
        writer.write_all(b"d").unwrap();
        assert!(writer.write_all(b"e").is_err());
        assert_eq!(bytes, b"abcd");
    }
}
