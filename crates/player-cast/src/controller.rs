//! One bounded worker owns the receiver; callers enqueue without waiting for network I/O.
use crate::{CastOptions, CastPlayback, CastStatus, CertificatePin};
use std::{
    io,
    net::SocketAddr,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    time::Duration,
};

#[derive(Clone, Debug, PartialEq)]
pub enum CastEvent {
    Status(CastStatus),
    Stopped,
    Failed(String),
    Rejected(String),
}
#[derive(Debug)]
enum Command {
    Pause,
    Play,
    Seek(f64),
    Wake,
}
/// Session-scoped worker handle. Drop requests owned stop without blocking a UI thread.
/// In-flight operations and cleanup retain the configured network deadlines.
/// Notifications run on the worker: dispatch them to the UI event loop before drawing.
pub struct CastController {
    sender: SyncSender<Command>,
    stopping: Arc<AtomicBool>,
}
impl CastController {
    pub fn start_file(
        path: PathBuf,
        address: SocketAddr,
        pin: CertificatePin,
        options: CastOptions,
        notify: impl Fn(CastEvent) + Send + 'static,
    ) -> io::Result<Self> {
        crate::transport::deadline(address, options.timeout)?;
        let (sender, receiver) = mpsc::sync_channel(8);
        let stopping = Arc::new(AtomicBool::new(false));
        let shutdown = stopping.clone();
        std::thread::Builder::new()
            .name("foxide-cast".into())
            .spawn(move || {
                match CastPlayback::start_file(&path, address, pin, options) {
                    Ok(playback) => run(playback, receiver, &shutdown, notify),
                    Err(error) => notify(CastEvent::Failed(error.to_string())),
                }
                shutdown.store(true, Ordering::Release);
            })?;
        Ok(Self { sender, stopping })
    }
    fn send(&self, command: Command) -> io::Result<()> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Casting session is stopping or closed",
            ));
        }
        self.sender.try_send(command).map_err(|error| match error {
            TrySendError::Full(_) => io::Error::new(
                io::ErrorKind::WouldBlock,
                "A casting operation is already queued; try again",
            ),
            TrySendError::Disconnected(_) => {
                io::Error::new(io::ErrorKind::NotConnected, "Casting worker has closed")
            }
        })
    }
    pub fn pause(&self) -> io::Result<()> {
        self.send(Command::Pause)
    }
    pub fn play(&self) -> io::Result<()> {
        self.send(Command::Play)
    }
    pub fn seek(&self, seconds: f64) -> io::Result<()> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Choose a finite, nonnegative seek position",
            ));
        }
        self.send(Command::Seek(seconds))
    }
    pub fn stop(&self) {
        self.stopping.store(true, Ordering::Release);
        let _ = self.sender.try_send(Command::Wake);
    }
}
impl Drop for CastController {
    fn drop(&mut self) {
        self.stop();
    }
}
trait Session {
    fn initial(&self) -> CastStatus;
    fn status(&mut self) -> io::Result<CastStatus>;
    fn apply(&mut self, command: Command) -> io::Result<CastStatus>;
    fn stop(self) -> io::Result<()>;
}
impl Session for CastPlayback {
    fn initial(&self) -> CastStatus {
        self.last_status().clone()
    }
    fn status(&mut self) -> io::Result<CastStatus> {
        CastPlayback::status(self)
    }
    fn apply(&mut self, command: Command) -> io::Result<CastStatus> {
        match command {
            Command::Pause => self.pause(),
            Command::Play => self.play(),
            Command::Seek(time) => self.seek(time),
            Command::Wake => self.status(),
        }
    }
    fn stop(self) -> io::Result<()> {
        CastPlayback::stop(self)
    }
}
fn run(
    mut session: impl Session,
    receiver: Receiver<Command>,
    stopping: &AtomicBool,
    notify: impl Fn(CastEvent),
) {
    let mut last = session.initial();
    notify(CastEvent::Status(last.clone()));
    let mut failed = false;
    while !stopping.load(Ordering::Acquire) {
        let operation = match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(_) if stopping.load(Ordering::Acquire) => break,
            Ok(command) => session.apply(command),
            Err(RecvTimeoutError::Timeout) => session.status(),
            Err(RecvTimeoutError::Disconnected) => break,
        };
        match operation {
            Ok(status) => {
                if status != last {
                    notify(CastEvent::Status(status.clone()));
                    last = status;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::InvalidInput => {
                notify(CastEvent::Rejected(error.to_string()))
            }
            Err(error) => {
                notify(CastEvent::Failed(error.to_string()));
                failed = true;
                break;
            }
        }
    }
    match session.stop() {
        Ok(()) if !failed => notify(CastEvent::Stopped),
        Err(error) if !failed => notify(CastEvent::Failed(error.to_string())),
        _ => {}
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
