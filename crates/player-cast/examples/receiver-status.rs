//! Engineering diagnostic; never launches/stops applications or saves pairing trust.
use player_cast::{CertificatePin, inspect_receiver_certificate, receiver_status};
use std::{io, net::SocketAddr, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: receiver-status inspect IP:PORT | status IP:PORT SHA256_PIN",
        )
    };
    let timeout = Duration::from_secs(5);
    match args.as_slice() {
        [mode, address] if mode == "inspect" => {
            let address: SocketAddr = address.parse()?;
            println!("{}", inspect_receiver_certificate(address, timeout)?);
        }
        [mode, address, pin] if mode == "status" => {
            let address: SocketAddr = address.parse()?;
            let pin: CertificatePin = pin.parse()?;
            let status = receiver_status(address, pin, timeout)?;
            // Deliberately omit receiver/app names, URLs, tokens and identifiers.
            let application_count = status
                .get("applications")
                .and_then(|v| v.as_array())
                .map_or(0, |a| a.len());
            println!(
                "{}",
                serde_json::json!({"status_received":true,"application_count":application_count})
            );
        }
        _ => return Err(usage().into()),
    }
    Ok(())
}
