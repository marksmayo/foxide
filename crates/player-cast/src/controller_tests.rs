use super::*;
use crate::CastState;
use std::{collections::VecDeque, sync::Mutex};
fn state(position: f64) -> CastStatus {
    CastStatus {
        state: CastState::Playing,
        position,
        duration: Some(20.0),
    }
}
struct Fake {
    replies: VecDeque<io::Result<CastStatus>>,
    stopped: Arc<AtomicBool>,
    stop_error: bool,
}
impl Session for Fake {
    fn initial(&self) -> CastStatus {
        state(0.0)
    }
    fn status(&mut self) -> io::Result<CastStatus> {
        self.replies.pop_front().expect("Unexpected poll")
    }
    fn apply(&mut self, _: Command) -> io::Result<CastStatus> {
        self.status()
    }
    fn stop(self) -> io::Result<()> {
        self.stopped.store(true, Ordering::Release);
        if self.stop_error {
            Err(io::Error::other("stop failed"))
        } else {
            Ok(())
        }
    }
}
fn fake(replies: Vec<io::Result<CastStatus>>, stop_error: bool) -> (Fake, Arc<AtomicBool>) {
    let stopped = Arc::new(AtomicBool::new(false));
    (
        Fake {
            replies: replies.into(),
            stopped: stopped.clone(),
            stop_error,
        },
        stopped,
    )
}
#[test]
fn queue_is_bounded_and_closed_handles_reject_commands() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let controller = CastController {
        sender,
        stopping: Arc::new(AtomicBool::new(false)),
    };
    controller.play().unwrap();
    assert_eq!(
        controller.pause().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    for value in [f64::NAN, f64::INFINITY, -1.0] {
        assert_eq!(
            controller.seek(value).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    drop(receiver);
    assert_eq!(
        controller.seek(1.0).unwrap_err().kind(),
        io::ErrorKind::NotConnected
    );
    controller.stop();
    assert_eq!(
        controller.pause().unwrap_err().kind(),
        io::ErrorKind::NotConnected
    );
}
#[test]
fn unchanged_status_is_silent_and_rejected_seek_keeps_session_alive() {
    let (sender, receiver) = mpsc::sync_channel(8);
    for command in [
        Command::Play,
        Command::Seek(5.0),
        Command::Seek(99.0),
        Command::Pause,
    ] {
        sender.send(command).unwrap();
    }
    let (session, stopped) = fake(
        vec![
            Ok(state(0.0)),
            Ok(state(5.0)),
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek out of range",
            )),
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "ownership lost",
            )),
        ],
        true,
    );
    let events = Mutex::new(Vec::new());
    run(session, receiver, &AtomicBool::new(false), |e| {
        events.lock().unwrap().push(e)
    });
    let events = events.into_inner().unwrap();
    assert_eq!(events.len(), 4);
    assert_eq!(events[0], CastEvent::Status(state(0.0)));
    assert_eq!(events[1], CastEvent::Status(state(5.0)));
    assert!(matches!(&events[2],CastEvent::Rejected(s) if s=="seek out of range"));
    assert!(matches!(&events[3],CastEvent::Failed(s) if s=="ownership lost"));
    assert!(stopped.load(Ordering::Acquire));
}
#[test]
fn periodic_status_services_connection_and_reports_failure_once() {
    let (_sender, receiver) = mpsc::sync_channel(1);
    let (session, stopped) = fake(
        vec![Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "status deadline",
        ))],
        false,
    );
    let events = Mutex::new(Vec::new());
    run(session, receiver, &AtomicBool::new(false), |e| {
        events.lock().unwrap().push(e)
    });
    assert!(stopped.load(Ordering::Acquire));
    let events = events.into_inner().unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(&events[1],CastEvent::Failed(s) if s=="status deadline"));
}
#[test]
fn disconnected_clients_and_failed_cleanup_have_explicit_outcomes() {
    for stop_error in [false, true] {
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(sender);
        let (session, stopped) = fake(vec![], stop_error);
        let events = Mutex::new(Vec::new());
        run(session, receiver, &AtomicBool::new(false), |e| {
            events.lock().unwrap().push(e)
        });
        assert!(stopped.load(Ordering::Acquire));
        let events = events.into_inner().unwrap();
        assert!(if stop_error {
            matches!(&events[1],CastEvent::Failed(s) if s=="stop failed")
        } else {
            events[1] == CastEvent::Stopped
        });
    }
}
#[test]
fn shutdown_skips_queued_commands_and_cleans_up() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (session, stopped) = fake(vec![], false);
    let shutdown = AtomicBool::new(false);
    run(session, receiver, &shutdown, |event| {
        if matches!(event, CastEvent::Status(_)) {
            shutdown.store(true, Ordering::Release);
            sender.send(Command::Wake).unwrap();
        }
    });
    assert!(stopped.load(Ordering::Acquire));
}
#[test]
fn shutdown_wakes_a_waiting_worker_without_running_queued_commands() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (session, stopped) = fake(vec![], false);
    let shutdown = Arc::new(AtomicBool::new(false));
    let flag = shutdown.clone();
    let (ready_tx, ready_rx) = mpsc::sync_channel(0);
    let (done_tx, done_rx) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        run(session, receiver, &flag, |event| {
            if matches!(event, CastEvent::Status(_)) {
                ready_tx.send(()).unwrap();
            } else {
                done_tx.send(event).unwrap();
            }
        })
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    shutdown.store(true, Ordering::Release);
    sender.send(Command::Wake).unwrap();
    assert_eq!(
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        CastEvent::Stopped
    );
    thread.join().unwrap();
    assert!(stopped.load(Ordering::Acquire));
}
#[test]
fn startup_validation_and_worker_failure_are_reported() {
    let address = "127.0.0.1:8009".parse().unwrap();
    let pin = CertificatePin::of(b"fixture");
    let path = std::env::temp_dir().join("foxide-no-such-cast-file.mp4");
    let invalid = CastOptions {
        timeout: Duration::ZERO,
        ..CastOptions::default()
    };
    assert!(
        CastController::start_file(path.clone(), address, pin, invalid, |_| panic!(
            "invalid setup started worker"
        ))
        .is_err()
    );
    let (tx, rx) = mpsc::channel();
    let controller =
        CastController::start_file(path, address, pin, CastOptions::default(), move |e| {
            tx.send(e).unwrap()
        })
        .unwrap();
    assert!(matches!(
        rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        CastEvent::Failed(_)
    ));
    drop(controller);
}
