use player_services::{
    CurlClient, WindowsUpdate, download_and_stage_windows_update, latest_windows_update,
    parse_sha256sum, verify_windows_package,
};
use serde_json::json;
use sha2::Digest;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

fn release(version: &str, prerelease: bool, draft: bool) -> serde_json::Value {
    let tag = format!("v{version}");
    json!({
        "tag_name": tag,
        "prerelease": prerelease,
        "draft": draft,
        "assets": [
            {
                "name": format!("Foxide-{version}-windows-x64.zip"),
                "size": 1024,
                "browser_download_url": format!("https://github.com/marksmayo/foxide/releases/download/{tag}/Foxide-{version}-windows-x64.zip")
            },
            {
                "name": "SHA256SUMS.txt",
                "size": 100,
                "browser_download_url": format!("https://github.com/marksmayo/foxide/releases/download/{tag}/SHA256SUMS.txt")
            }
        ]
    })
}

#[test]
fn offers_the_newest_stable_windows_package_and_skips_drafts_and_previews() {
    let releases = json!([
        release("1.4.0", false, false),
        release("1.5.0-preview.1", true, false),
        release("1.6.0", false, true),
        release("1.3.9", false, false),
    ]);
    let update = latest_windows_update(&releases.to_string(), "1.3.0")
        .unwrap()
        .unwrap();
    assert_eq!(update.version, "1.4.0");
    assert_eq!(update.package_size, 1024);
    assert_eq!(
        update.package_url.as_str(),
        "https://github.com/marksmayo/foxide/releases/download/v1.4.0/Foxide-1.4.0-windows-x64.zip"
    );
}

#[test]
fn preview_builds_can_receive_newer_preview_or_stable_releases() {
    let releases = json!([
        release("1.4.0-preview.3", true, false),
        release("1.4.0-preview.2", true, false),
    ]);
    let update = latest_windows_update(&releases.to_string(), "1.4.0-preview.1")
        .unwrap()
        .unwrap();
    assert_eq!(update.version, "1.4.0-preview.3");
}

#[test]
fn refuses_untrusted_asset_hosts_and_incomplete_releases() {
    let mut item = release("1.4.0", false, false);
    item["assets"][0]["browser_download_url"] =
        "https://example.org/Foxide-1.4.0-windows-x64.zip".into();
    assert!(latest_windows_update(&json!([item]).to_string(), "1.3.0").is_err());

    let mut item = release("1.4.0", false, false);
    item["assets"] = json!([item["assets"][0].clone()]);
    assert!(latest_windows_update(&json!([item]).to_string(), "1.3.0").is_err());
}

#[test]
fn checksum_parser_requires_one_exact_sha256_entry() {
    let hash = "ab".repeat(32);
    assert_eq!(
        parse_sha256sum(&format!("{hash}  Foxide.zip\n"), "Foxide.zip").unwrap(),
        [0xab; 32]
    );
    assert!(parse_sha256sum(&format!("{hash}  Other.zip\n"), "Foxide.zip").is_err());
    assert!(
        parse_sha256sum(
            &format!("{hash}  Foxide.zip\n{hash}  Foxide.zip\n"),
            "Foxide.zip"
        )
        .is_err()
    );
    assert!(parse_sha256sum("not-a-hash  Foxide.zip\n", "Foxide.zip").is_err());
}

#[test]
fn package_verification_rejects_metadata_size_mismatches() {
    let update = latest_windows_update(
        &json!([release("1.4.0", false, false)]).to_string(),
        "1.3.0",
    )
    .unwrap()
    .unwrap();
    let package = b"verified package";
    let checksum = format!("{}  {}\n", "00".repeat(32), "Foxide-1.4.0-windows-x64.zip");
    assert!(verify_windows_package(&update, package, &checksum).is_err());
}

#[test]
fn package_verification_rejects_sha256_mismatches() {
    let update = latest_windows_update(
        &json!([release("1.4.0", false, false)]).to_string(),
        "1.3.0",
    )
    .unwrap()
    .unwrap();
    let package = vec![7_u8; update.package_size as usize];
    let checksum = format!("{}  Foxide-1.4.0-windows-x64.zip\n", "00".repeat(32));
    assert!(verify_windows_package(&update, &package, &checksum).is_err());
}

#[test]
fn package_verification_accepts_the_exact_published_size_and_hash() {
    use sha2::Digest;
    let update = latest_windows_update(
        &json!([release("1.4.0", false, false)]).to_string(),
        "1.3.0",
    )
    .unwrap()
    .unwrap();
    let package = vec![0x42; update.package_size as usize];
    let hash = format!("{:x}", sha2::Sha256::digest(&package));
    let checksum = format!("{hash}  Foxide-1.4.0-windows-x64.zip\n");
    assert!(verify_windows_package(&update, &package, &checksum).is_ok());
}

#[test]
fn staging_publishes_only_downloads_matching_release_metadata_and_checksum() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let package = vec![0x42; 1024];
    let checksum = format!(
        "{:x}  Foxide-1.4.0-windows-x64.zip\n",
        sha2::Sha256::digest(&package)
    );
    let stop = Arc::new(AtomicBool::new(false));
    let done = stop.clone();
    let expected_package = package.clone();
    let expected_checksum = checksum.clone();
    let worker = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut served = 0;
        while !done.load(Ordering::Acquire) && Instant::now() < deadline && served < 4 {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut chunk = [0; 512];
                while !request.ends_with(b"\r\n\r\n") && request.len() < 4096 {
                    let count = stream.read(&mut chunk).unwrap();
                    assert!(count > 0, "Client closed before sending request headers");
                    request.extend_from_slice(&chunk[..count]);
                }
                let request = String::from_utf8_lossy(&request);
                let body = if request.starts_with("GET /checksum-bad ") {
                    b"0000000000000000000000000000000000000000000000000000000000000000  Foxide-1.4.0-windows-x64.zip\n".to_vec()
                } else if request.starts_with("GET /checksum ") {
                    expected_checksum.as_bytes().to_vec()
                } else {
                    expected_package.clone()
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                let _ = stream.write_all(&body);
                served += 1;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        served
    });
    let client = CurlClient::system().unwrap();
    let base = format!("http://{address}");
    let make_update = |checksum_path: &str| WindowsUpdate {
        version: "1.4.0".into(),
        tag: "v1.4.0".into(),
        package_url: player_services::Url::parse(&format!("{base}/package")).unwrap(),
        package_size: package.len() as u64,
        checksums_url: player_services::Url::parse(&format!("{base}/{checksum_path}")).unwrap(),
    };
    let directory = std::env::temp_dir().join(format!(
        "foxide-stage-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cancel = AtomicBool::new(false);
    let staged =
        download_and_stage_windows_update(&client, &make_update("checksum"), &directory, &cancel);
    let rejected = download_and_stage_windows_update(
        &client,
        &make_update("checksum-bad"),
        &directory,
        &cancel,
    );
    stop.store(true, Ordering::Release);
    assert_eq!(worker.join().unwrap(), 4);
    let staged = staged.unwrap();
    assert_eq!(std::fs::read(&staged).unwrap(), package);
    assert!(rejected.is_err());
    assert!(
        !directory
            .join("Foxide-1.4.0-windows-x64.zip.download")
            .exists()
    );
    std::fs::remove_dir_all(directory).unwrap();
}
