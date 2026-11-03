use super::*;
use crate::laser_display::tests::raw_pair;
#[test]
fn slow_preview_output_does_not_hold_producer_and_final_latest_input_is_drained() {
    let mailbox = Arc::new(Mutex::new(None));
    let omitted = Arc::new(AtomicU64::new(0));
    let done = Arc::new(AtomicBool::new(false));
    let work = Arc::new(WorkControl::default());
    let (pair, profiles, outcome, progress) = raw_pair(0);
    assert!(matches!(
        offer(
            PreviewInput {
                pair: &pair,
                profiles: &profiles,
                outcome: &outcome,
                progress
            },
            Origin::RecordingReplay,
            &work,
            &mailbox,
            &omitted
        ),
        PreviewStatus::Accepted
    ));
    let (ready, started) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let (m, o, d, w) = (mailbox.clone(), omitted.clone(), done.clone(), work.clone());
    let consumer = std::thread::spawn(move || {
        let mut received = Vec::new();
        publish(&m, &d, &o, &w, |snapshot| {
            received.push(snapshot.indices);
            if received.len() == 1 {
                ready.send(()).unwrap();
                wait.recv().unwrap();
            }
            Ok(())
        })
        .unwrap();
        received
    });
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    let (sent, observed) = std::sync::mpsc::channel();
    let (m, o, d, w) = (mailbox.clone(), omitted.clone(), done.clone(), work.clone());
    let producer = std::thread::spawn(move || {
        for index in [1, 2] {
            let (pair, profiles, outcome, progress) = raw_pair(index);
            assert!(matches!(
                offer(
                    PreviewInput {
                        pair: &pair,
                        profiles: &profiles,
                        outcome: &outcome,
                        progress
                    },
                    Origin::RecordingReplay,
                    &w,
                    &m,
                    &o
                ),
                PreviewStatus::Accepted
            ));
        }
        d.store(true, Ordering::Release);
        sent.send(()).unwrap();
    });
    let result = observed.recv_timeout(Duration::from_secs(5));
    // Always release before asserting or joining to avoid stranding threads.
    release.send(()).unwrap();
    producer.join().unwrap();
    let received = consumer.join().unwrap();
    result.expect("producer was blocked by slow optional output");
    assert_eq!(received, [[0, 0], [2, 2]]);
    assert_eq!(omitted.load(Ordering::Relaxed), 1);
    assert!(mailbox.lock().unwrap().is_none());
    assert!(work.check().is_ok());
}
#[test]
fn worker_preparation_rejection_is_an_omission_and_does_not_cancel_recording() {
    let (pair, profiles, outcome, mut progress) = raw_pair(0);
    progress.journaled_frames = [0; 2];
    let work = WorkControl::default();
    let mailbox = Mutex::new(None);
    let omitted = AtomicU64::new(0);
    assert!(matches!(
        offer(
            PreviewInput {
                pair: &pair,
                profiles: &profiles,
                outcome: &outcome,
                progress
            },
            Origin::Native,
            &work,
            &mailbox,
            &omitted
        ),
        PreviewStatus::Accepted
    ));
    publish(&mailbox, &AtomicBool::new(true), &omitted, &work, |_| {
        panic!("invalid snapshot must not be sent")
    })
    .unwrap();
    assert_eq!(omitted.load(Ordering::Relaxed), 1);
    assert!(work.check().is_ok());
    let (pair, profiles, outcome, progress) = raw_pair(1);
    assert!(matches!(
        offer(
            PreviewInput {
                pair: &pair,
                profiles: &profiles,
                outcome: &outcome,
                progress
            },
            Origin::Native,
            &work,
            &mailbox,
            &omitted
        ),
        PreviewStatus::Accepted
    ));
    let mut sent = false;
    publish(
        &mailbox,
        &AtomicBool::new(true),
        &omitted,
        &work,
        |snapshot| {
            assert_eq!(snapshot.indices, [1; 2]);
            sent = true;
            Ok(())
        },
    )
    .unwrap();
    assert!(sent);
}
