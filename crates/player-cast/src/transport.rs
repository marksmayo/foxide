use crate::CertificatePin;
use player_services::cast_wire::{Message, Payload};
use rustls::{
    ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme, StreamOwned,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use serde_json::{Value, json};
use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const CONNECTION: &str = "urn:x-cast:com.google.cast.tp.connection";
const RECEIVER: &str = "urn:x-cast:com.google.cast.receiver";
const HEARTBEAT: &str = "urn:x-cast:com.google.cast.tp.heartbeat";

/// Query an explicitly paired receiver without launching or stopping any application.
/// The timeout bounds the entire operation, including fragmented TLS/frame reads.
/// Run on a worker thread. Addresses are literal local-network IPs; no DNS lookup occurs.
pub fn receiver_status(
    address: SocketAddr,
    pin: CertificatePin,
    timeout: Duration,
) -> io::Result<Value> {
    Channel::open(address, pin, timeout)?.receiver_status()
}

pub(crate) fn deadline(address: SocketAddr, timeout: Duration) -> io::Result<Instant> {
    let local = match address.ip() {
        std::net::IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        std::net::IpAddr::V6(ip) => {
            ip.is_unique_local() || ip.is_loopback() || ip.is_unicast_link_local()
        }
    };
    if !local || address.port() == 0 || timeout.is_zero() || timeout > Duration::from_secs(30) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Choose a local receiver and a timeout between zero and 30 seconds",
        ));
    }
    Ok(Instant::now() + timeout)
}

fn connect(
    address: SocketAddr,
    deadline: Instant,
    verifier: Arc<dyn ServerCertVerifier>,
) -> io::Result<StreamOwned<ClientConnection, DeadlineTcp>> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(io::Error::other)?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    let socket = TcpStream::connect_timeout(&address, remaining(deadline)?)?;
    let name = ServerName::IpAddress(address.ip().into());
    let mut tls = StreamOwned::new(
        ClientConnection::new(Arc::new(config), name).map_err(io::Error::other)?,
        DeadlineTcp { socket, deadline },
    );
    while tls.conn.is_handshaking() {
        tls.conn.complete_io(&mut tls.sock)?;
    }
    Ok(tls)
}

fn remaining(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "Receiver operation timed out"))
}
struct DeadlineTcp {
    socket: TcpStream,
    deadline: Instant,
}
impl Read for DeadlineTcp {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.socket
            .set_read_timeout(Some(remaining(self.deadline)?))?;
        self.socket.read(output)
    }
}
impl Write for DeadlineTcp {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.socket
            .set_write_timeout(Some(remaining(self.deadline)?))?;
        self.socket.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        remaining(self.deadline)?;
        self.socket.flush()
    }
}
#[derive(Debug)]
enum Trust {
    Pinned(CertificatePin),
    Inspect(Arc<Mutex<Option<CertificatePin>>>),
}
#[derive(Debug)]
struct PinnedCertificate {
    trust: Trust,
    provider: Arc<CryptoProvider>,
}
impl ServerCertVerifier for PinnedCertificate {
    fn verify_server_cert(
        &self,
        cert: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let presented = CertificatePin::of(cert.as_ref());
        match &self.trust {
            Trust::Pinned(pin) if *pin == presented => Ok(ServerCertVerified::assertion()),
            Trust::Pinned(_) => Err(rustls::Error::General(
                "Receiver certificate changed or is not trusted".into(),
            )),
            Trust::Inspect(observed) => {
                *observed.lock().map_err(|_| {
                    rustls::Error::General("Certificate inspection unavailable".into())
                })? = Some(presented);
                Err(rustls::Error::General(
                    "Inspection stops before establishing trust".into(),
                ))
            }
        }
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Return the presented leaf-certificate fingerprint, then abort the handshake.
/// No application command is sent. This does not authenticate the device or save trust.
pub fn inspect_receiver_certificate(
    address: SocketAddr,
    timeout: Duration,
) -> io::Result<CertificatePin> {
    let deadline = deadline(address, timeout)?;
    let observed = Arc::new(Mutex::new(None));
    let verifier = Arc::new(PinnedCertificate {
        trust: Trust::Inspect(observed.clone()),
        provider: Arc::new(rustls::crypto::ring::default_provider()),
    });
    let result = connect(address, deadline, verifier);
    observed
        .lock()
        .map_err(|_| io::Error::other("Certificate inspection unavailable"))?
        .take()
        .ok_or_else(|| {
            result
                .err()
                .unwrap_or_else(|| io::Error::other("Receiver did not present a certificate"))
        })
}

/// A single pinned control connection. One begin() deadline covers every RPC
/// belonging to a user operation; request IDs distinguish unsolicited updates.
pub(crate) struct Channel {
    tls: StreamOwned<ClientConnection, DeadlineTcp>,
    address: SocketAddr,
    next_request: u64,
}
impl Channel {
    pub(crate) fn open(
        address: SocketAddr,
        pin: CertificatePin,
        timeout: Duration,
    ) -> io::Result<Self> {
        let until = deadline(address, timeout)?;
        let verifier = Arc::new(PinnedCertificate {
            trust: Trust::Pinned(pin),
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        });
        let mut seed = [0; 4];
        getrandom::fill(&mut seed)
            .map_err(|_| io::Error::other("Secure randomness unavailable"))?;
        let mut channel = Self {
            tls: connect(address, until, verifier)?,
            address,
            next_request: u64::from(u32::from_le_bytes(seed) & 0x7fff_ffff) + 1,
        };
        channel.connect_destination("receiver-0")?;
        Ok(channel)
    }
    pub(crate) fn begin(&mut self, timeout: Duration) -> io::Result<()> {
        self.tls.sock.deadline = deadline(self.address, timeout)?;
        Ok(())
    }
    pub(crate) fn connect_destination(&mut self, destination: &str) -> io::Result<()> {
        Message::json(
            "sender-0",
            destination,
            CONNECTION,
            json!({"type":"CONNECT","origin":{}}),
        )?
        .write(&mut self.tls)?;
        self.tls.flush()
    }
    pub(crate) fn request(
        &mut self,
        destination: &str,
        namespace: &str,
        mut payload: Value,
        expected: &str,
    ) -> io::Result<Value> {
        #[cfg(feature = "device-integration")]
        if std::env::var_os("FOXIDE_CAST_TRACE").is_some() {
            eprintln!(
                "[cast-request] type={} expected={}",
                payload["type"].as_str().unwrap_or("other"),
                expected
            );
        }
        let id = self.next_request;
        self.next_request = id
            .checked_add(1)
            .filter(|next| *next <= 9_007_199_254_740_991)
            .ok_or_else(|| io::Error::other("Cast request limit exceeded"))?;
        payload
            .as_object_mut()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Expected a Cast command object",
                )
            })?
            .insert("requestId".into(), id.into());
        Message::json("sender-0", destination, namespace, payload)?.write(&mut self.tls)?;
        self.tls.flush()?;
        self.receive(destination, namespace, expected, id, false)
    }
    pub(crate) fn update(
        &mut self,
        destination: &str,
        namespace: &str,
        expected: &str,
        request_id: u64,
    ) -> io::Result<Value> {
        self.receive(destination, namespace, expected, request_id, true)
    }
    fn receive(
        &mut self,
        destination: &str,
        namespace: &str,
        expected: &str,
        id: u64,
        accept_broadcast: bool,
    ) -> io::Result<Value> {
        for _ in 0..64 {
            let message = Message::read(&mut self.tls)?;
            #[cfg(feature = "device-integration")]
            if std::env::var_os("FOXIDE_CAST_TRACE").is_some() {
                let kind = match message.payload() {
                    Payload::Json(value) => [
                        "RECEIVER_STATUS",
                        "MEDIA_STATUS",
                        "PING",
                        "PONG",
                        "LAUNCH_ERROR",
                        "LOAD_FAILED",
                        "INVALID_REQUEST",
                        "INVALID_PLAYER_STATE",
                        "ERROR",
                    ]
                    .into_iter()
                    .find(|kind| value["type"].as_str() == Some(*kind))
                    .unwrap_or("other"),
                    _ => "binary",
                };
                let correlated =
                    matches!(message.payload(),Payload::Json(value) if value["requestId"]==id);
                eprintln!(
                    "[cast-response] type={} source_matches={} destination_matches={} namespace_matches={} request_matches={}",
                    kind,
                    message.source() == destination,
                    matches!(message.destination(), "sender-0" | "*"),
                    message.namespace() == namespace,
                    correlated
                );
            }
            let Payload::Json(value) = message.payload() else {
                continue;
            };
            if message.namespace() == HEARTBEAT
                && value["type"] == "PING"
                && matches!(message.source(), "receiver-0" | "transport-0")
                && matches!(message.destination(), "sender-0" | "transport-0" | "*")
            {
                let source = if message.destination() == "*" {
                    "sender-0"
                } else {
                    message.destination()
                };
                Message::json(source, message.source(), HEARTBEAT, json!({"type":"PONG"}))?
                    .write(&mut self.tls)?;
                self.tls.flush()?;
                continue;
            }
            if !matches!(message.destination(), "sender-0" | "*") {
                continue;
            }
            if message.source() != destination
                || message.namespace() != namespace
                || (value["requestId"] != id && (!accept_broadcast || value["requestId"] != 0))
            {
                continue;
            }
            if value["type"] == expected {
                return Ok(value.clone());
            }
            if matches!(
                value["type"].as_str(),
                Some(
                    "INVALID_REQUEST"
                        | "INVALID_PLAYER_STATE"
                        | "ERROR"
                        | "LAUNCH_ERROR"
                        | "LOAD_FAILED"
                        | "LOAD_CANCELLED"
                )
            ) {
                return Err(io::Error::other("Receiver rejected the casting command"));
            }
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Receiver response limit exceeded",
        ))
    }
    pub(crate) fn receiver_status(&mut self) -> io::Result<Value> {
        let response = self.request(
            "receiver-0",
            RECEIVER,
            json!({"type":"GET_STATUS"}),
            "RECEIVER_STATUS",
        )?;
        response
            .get("status")
            .filter(|v| v.is_object())
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid receiver status"))
    }
}
