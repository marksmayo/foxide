//! Explicitly paired Cast receiver transport, isolated from playback policy.
use sha2::{Digest, Sha256};
use std::{fmt, io, str::FromStr};

/// SHA-256 of a receiver's exact leaf certificate. Trust must be granted explicitly.
/// This is certificate pinning, not certification by Google's device-auth authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CertificatePin([u8; 32]);
impl CertificatePin {
    pub fn of(certificate: &[u8]) -> Self {
        Self(Sha256::digest(certificate).into())
    }
}
impl fmt::Display for CertificatePin {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(output, "{byte:02x}")?;
        }
        Ok(())
    }
}
impl FromStr for CertificatePin {
    type Err = io::Error;
    fn from_str(text: &str) -> io::Result<Self> {
        if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Expected a 64-digit SHA-256 certificate fingerprint",
            ));
        }
        let mut bytes = [0; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
                .map_err(io::Error::other)?;
        }
        Ok(Self(bytes))
    }
}

mod transport;
pub use transport::{inspect_receiver_certificate, receiver_status};

mod media_origin;
pub use media_origin::MediaOrigin;
