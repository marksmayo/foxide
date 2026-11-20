//! Blocking owned-session operations. Call from a worker; no UI/audio clock runs here.
use crate::{CertificatePin, MediaOrigin, transport::Channel};
use serde_json::{Value, json};
use std::{io, net::SocketAddr, path::Path, time::Duration};
const RECEIVER: &str = "urn:x-cast:com.google.cast.receiver";
const MEDIA: &str = "urn:x-cast:com.google.cast.media";
const APP: &str = "CC1AD845";

#[derive(Clone, Copy, Debug)]
pub struct CastOptions {
    /// Only an explicit user action may replace a receiver's existing application.
    pub allow_replacing_application: bool,
    pub initially_paused: bool,
    /// Total deadline for each operation, including its ownership checks.
    pub timeout: Duration,
}
impl Default for CastOptions {
    fn default() -> Self {
        Self {
            allow_replacing_application: false,
            initially_paused: false,
            timeout: Duration::from_secs(10),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastState {
    Playing,
    Paused,
    Buffering,
    Idle,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CastStatus {
    pub state: CastState,
    pub position: f64,
    pub duration: Option<f64>,
}

/// Owns a receiver application identity, media identity and ephemeral file origin.
/// Drop revokes file serving and closes control transport. stop() explicitly stops
/// only the still-owned media; it never sends a receiver-wide application STOP.
pub struct CastPlayback {
    channel: Channel,
    origin: MediaOrigin,
    app_session: String,
    destination: String,
    media_session: u64,
    timeout: Duration,
    last: CastStatus,
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn lost() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "The receiver is now controlled by another session",
    )
}
fn app(status: &Value) -> io::Result<(&str, &str)> {
    let application = status["applications"]
        .as_array()
        .and_then(|apps| apps.iter().find(|a| a["appId"] == APP))
        .ok_or_else(|| invalid("Receiver did not start the media application"))?;
    let identifier = |key: &str| -> io::Result<&str> {
        application[key]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control))
            .ok_or_else(|| invalid("Invalid receiver application identity"))
    };
    Ok((identifier("sessionId")?, identifier("transportId")?))
}
fn media_status(value: &Value, previous: Option<&CastStatus>) -> io::Result<CastStatus> {
    let state = match value["playerState"].as_str() {
        Some("PLAYING") => CastState::Playing,
        Some("PAUSED") => CastState::Paused,
        Some("BUFFERING") => CastState::Buffering,
        Some("IDLE") if value["idleReason"] != "ERROR" => CastState::Idle,
        _ => {
            return Err(invalid(
                "Receiver reported an invalid or failed playback state",
            ));
        }
    };
    let position = match value.get("currentTime") {
        None => previous.map_or(0.0, |status| status.position),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| invalid("Invalid receiver playback clock"))?,
    };
    let duration = match value["media"].get("duration") {
        None => previous.and_then(|status| status.duration),
        Some(Value::Null) => None,
        Some(value) => Some(
            value
                .as_f64()
                .ok_or_else(|| invalid("Invalid receiver media duration"))?,
        ),
    };
    if !position.is_finite()
        || position < 0.0
        || duration.is_some_and(|d| !d.is_finite() || d < 0.0)
    {
        return Err(invalid("Invalid receiver playback clock"));
    }
    Ok(CastStatus {
        state,
        position,
        duration,
    })
}

fn matches_owned(value: &Value, id: u64, url: &str) -> bool {
    if value["mediaSessionId"] != id {
        return false;
    }
    match value.get("media") {
        None => true,
        Some(Value::Object(media)) => media
            .get("contentId")
            .is_none_or(|value| value.as_str() == Some(url)),
        Some(_) => false,
    }
}
impl CastPlayback {
    pub fn start_file(
        path: &Path,
        address: SocketAddr,
        pin: CertificatePin,
        options: CastOptions,
    ) -> io::Result<Self> {
        // Validate timeout before opening a local port or touching the receiver.
        crate::transport::deadline(address, options.timeout)?;
        let origin = MediaOrigin::new(path, address.ip())?;
        let mut channel = Channel::open(address, pin, options.timeout)?;
        let current = channel.receiver_status()?;
        match current.get("applications") {
            None | Some(Value::Null) => {}
            Some(Value::Array(apps))
                if apps
                    .iter()
                    .all(|app| app.get("isIdleScreen").and_then(Value::as_bool) == Some(true))
                    || options.allow_replacing_application => {}
            Some(Value::Array(_)) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "The receiver already has an application; explicit replacement is required",
                ));
            }
            _ => return Err(invalid("Invalid receiver application list")),
        }
        let launched = channel.request(
            "receiver-0",
            RECEIVER,
            json!({"type":"LAUNCH","appId":APP}),
            "RECEIVER_STATUS",
        )?;
        let (app_session, destination) = app(&launched["status"])?;
        let (app_session, destination) = (app_session.to_string(), destination.to_string());
        channel.connect_destination(&destination)?;
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Foxide media")
            .chars()
            .filter(|c| !c.is_control())
            .take(256)
            .collect::<String>();
        let loaded=channel.request(&destination,MEDIA,json!({"type":"LOAD","autoplay":!options.initially_paused,"media":{"contentId":origin.url(),"contentType":origin.media_type(),"streamType":"BUFFERED","metadata":{"metadataType":0,"title":title}}}),"MEDIA_STATUS")?;
        let entry = loaded["status"]
            .as_array()
            .and_then(|values| {
                values
                    .iter()
                    .find(|v| v["media"]["contentId"] == origin.url())
            })
            .ok_or_else(|| invalid("Receiver did not load the selected media"))?;
        let media_session = entry["mediaSessionId"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or_else(|| invalid("Invalid receiver media identity"))?;
        let request_id = loaded["requestId"]
            .as_u64()
            .ok_or_else(|| invalid("Missing load response identity"))?;
        let mut entry = entry.clone();
        let mut last = media_status(&entry, None)?;
        for _ in 0..64 {
            if matches!(last.state, CastState::Playing | CastState::Paused) {
                break;
            }
            if last.state == CastState::Idle && entry.get("idleReason").is_some() {
                return Err(invalid(
                    "Receiver ended the load before playback became ready",
                ));
            }
            let update = channel.update(&destination, MEDIA, "MEDIA_STATUS", request_id)?;
            entry = update["status"]
                .as_array()
                .and_then(|entries| {
                    entries
                        .iter()
                        .find(|value| matches_owned(value, media_session, origin.url()))
                })
                .cloned()
                .ok_or_else(lost)?;
            last = media_status(&entry, Some(&last))?;
        }
        if !matches!(last.state, CastState::Playing | CastState::Paused) {
            return Err(invalid("Receiver playback readiness limit exceeded"));
        }
        Ok(Self {
            channel,
            origin,
            app_session,
            destination,
            media_session,
            timeout: options.timeout,
            last,
        })
    }
    pub fn transfer_stats(&self) -> crate::TransferStats {
        self.origin.transfer_stats()
    }
    pub fn last_status(&self) -> &CastStatus {
        &self.last
    }
    fn owned(&mut self) -> io::Result<Option<Value>> {
        let current = self.channel.receiver_status()?;
        let (session, destination) = app(&current).map_err(|_| lost())?;
        if session != self.app_session || destination != self.destination {
            return Err(lost());
        }
        let response = self.channel.request(
            &self.destination,
            MEDIA,
            json!({"type":"GET_STATUS"}),
            "MEDIA_STATUS",
        )?;
        let entries = response["status"]
            .as_array()
            .ok_or_else(|| invalid("Invalid receiver media status"))?;
        if entries.is_empty() {
            return Ok(None);
        }
        entries
            .iter()
            .find(|v| matches_owned(v, self.media_session, self.origin.url()))
            .cloned()
            .map(Some)
            .ok_or_else(lost)
    }
    pub fn status(&mut self) -> io::Result<CastStatus> {
        self.channel.begin(self.timeout)?;
        self.last = if let Some(entry) = self.owned()? {
            media_status(&entry, Some(&self.last))?
        } else {
            CastStatus {
                state: CastState::Idle,
                position: self.last.duration.unwrap_or(self.last.position),
                duration: self.last.duration,
            }
        };
        Ok(self.last.clone())
    }
    fn control(&mut self, mut request: Value) -> io::Result<CastStatus> {
        self.channel.begin(self.timeout)?;
        if self.owned()?.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Media playback has ended",
            ));
        }
        request
            .as_object_mut()
            .unwrap()
            .insert("mediaSessionId".into(), self.media_session.into());
        let response = self
            .channel
            .request(&self.destination, MEDIA, request, "MEDIA_STATUS")?;
        let entries = response["status"]
            .as_array()
            .ok_or_else(|| invalid("Invalid receiver media status"))?;
        self.last = if entries.is_empty() {
            CastStatus {
                state: CastState::Idle,
                ..self.last.clone()
            }
        } else {
            let entry = entries
                .iter()
                .find(|v| matches_owned(v, self.media_session, self.origin.url()))
                .ok_or_else(lost)?;
            media_status(entry, Some(&self.last))?
        };
        Ok(self.last.clone())
    }
    pub fn pause(&mut self) -> io::Result<CastStatus> {
        self.control(json!({"type":"PAUSE"}))
    }
    pub fn play(&mut self) -> io::Result<CastStatus> {
        self.control(json!({"type":"PLAY"}))
    }
    pub fn seek(&mut self, seconds: f64) -> io::Result<CastStatus> {
        if !seconds.is_finite() || seconds < 0.0 || self.last.duration.is_some_and(|d| seconds > d)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Choose a seek position inside the media duration",
            ));
        }
        self.control(json!({"type":"SEEK","currentTime":seconds}))
    }
    pub fn stop(mut self) -> io::Result<()> {
        self.channel.begin(self.timeout)?;
        if self.owned()?.is_some() {
            self.channel.request(
                &self.destination,
                MEDIA,
                json!({"type":"STOP","mediaSessionId":self.media_session}),
                "MEDIA_STATUS",
            )?;
        }
        self.origin.shutdown();
        Ok(())
    }
}
