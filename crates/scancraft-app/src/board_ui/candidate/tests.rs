use super::*;
fn finish(ui: &mut CandidateUi) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while ui.is_busy() {
        assert!(std::time::Instant::now() < end);
        ui.tick();
        thread::yield_now();
    }
}
#[test]
fn failure_and_panic_preserve_last_published_candidate() {
    let mut ui = CandidateUi::default();
    ui.spawn(|_| {
        Ok(Completed {
            directory: "saved".into(),
            manifest: Value::Null,
        })
    });
    finish(&mut ui);
    assert!(ui.message.contains("has not been selected"));
    for panic in [false, true] {
        ui.spawn(move |_| {
            if panic {
                panic!("test candidate panic")
            } else {
                Err("test failed fit".into())
            }
        });
        finish(&mut ui);
        assert_eq!(
            ui.completed.as_ref().unwrap().directory,
            PathBuf::from("saved")
        );
        assert!(ui.message.contains("Previous candidate retained"));
    }
}
#[test]
fn cancellation_joins_the_job_and_duplicate_start_cannot_replace_it() {
    let mut ui = CandidateUi::default();
    ui.spawn(|work| {
        while work.check().is_ok() {
            thread::yield_now();
        }
        Err("test fit cancelled".into())
    });
    ui.start("must-not-read".into(), "must-not-publish".into());
    assert!(ui.request.is_empty());
    ui.cancel();
    finish(&mut ui);
    assert!(ui.message.contains("test fit cancelled"));
    assert!(ui.message.contains("No previous candidate is available"));
    assert!(ui.completed.is_none());
}
