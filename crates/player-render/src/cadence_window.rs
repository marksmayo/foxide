//! Measurement clock for the compositor regression test.
use std::time::Duration;

#[derive(Default)]
pub(crate) struct CadenceWindow {
    first_video: Option<Duration>,
}

impl CadenceWindow {
    const STARTUP_DEADLINE: Duration = Duration::from_secs(15);
    const SAMPLE_DURATION: Duration = Duration::from_secs(5);

    pub(crate) fn observe(&mut self, elapsed: Duration, video_present: bool) {
        if self.first_video.is_none() && video_present && elapsed < Self::STARTUP_DEADLINE {
            self.first_video = Some(elapsed);
        }
    }

    pub(crate) fn complete(&self, elapsed: Duration) -> bool {
        self.first_video
            .is_some_and(|first| elapsed >= first + Self::SAMPLE_DURATION)
    }

    pub(crate) fn finished(&self, elapsed: Duration) -> bool {
        self.complete(elapsed) || (self.first_video.is_none() && elapsed >= Self::STARTUP_DEADLINE)
    }
}

#[test]
fn cadence_window_starts_at_presented_video_and_retains_a_startup_deadline() {
    let mut timing = CadenceWindow::default();
    timing.observe(Duration::from_secs(4), false);
    assert!(!timing.finished(Duration::from_secs(6)));
    timing.observe(Duration::from_secs(4), true);
    assert!(!timing.finished(Duration::from_secs(6)));
    assert!(timing.finished(Duration::from_secs(9)));
    assert!(timing.complete(Duration::from_secs(9)));

    let mut black = CadenceWindow::default();
    black.observe(Duration::from_secs(14), false);
    assert!(!black.finished(Duration::from_secs(14)));
    assert!(black.finished(Duration::from_secs(15)));
    black.observe(Duration::from_secs(16), true);
    assert!(
        !black.complete(Duration::from_secs(21)),
        "Late video must not erase a startup timeout"
    );
}
