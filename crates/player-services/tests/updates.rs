use player_services::{latest_windows_update, parse_sha256sum, verify_windows_package};
use serde_json::json;

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
