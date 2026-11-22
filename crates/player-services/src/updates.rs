//! Release metadata validation for the Windows portable update channel.
use crate::{CurlClient, HttpClient, Url};
use semver::Version;
use serde_json::Value;
use std::{path::Path, sync::atomic::AtomicBool};

const RELEASES_HOST: &str = "github.com";
const REPOSITORY: &str = "marksmayo/foxide";
const MAX_PACKAGE_SIZE: u64 = 512 * 1024 * 1024;
const MAX_CHECKSUM_SIZE: u64 = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsUpdate {
    pub version: String,
    pub tag: String,
    pub package_url: Url,
    pub package_size: u64,
    pub checksums_url: Url,
}

/// Select the newest compatible published release newer than `current_version`.
/// Stable installations stay on stable releases; preview installations also see
/// previews. Only the repository's expected ZIP and checksum asset URLs are accepted.
pub fn latest_windows_update(
    response: &str,
    current_version: &str,
) -> Result<Option<WindowsUpdate>, String> {
    let current = Version::parse(current_version).map_err(|_| "Invalid current version")?;
    let releases: Vec<Value> =
        serde_json::from_str(response).map_err(|_| "Invalid release list")?;
    let mut candidates = Vec::new();
    for release in releases {
        if release.get("draft").and_then(Value::as_bool) != Some(false) {
            continue;
        }
        let Some(tag) = release.get("tag_name").and_then(Value::as_str) else {
            continue;
        };
        let Some(version_text) = tag.strip_prefix('v') else {
            continue;
        };
        let Ok(version) = Version::parse(version_text) else {
            continue;
        };
        let prerelease = release
            .get("prerelease")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if prerelease != !version.pre.is_empty()
            || version <= current
            || (current.pre.is_empty() && prerelease)
        {
            continue;
        }
        candidates.push((version, tag.to_owned(), release));
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0));
    let Some((version, tag, release)) = candidates.into_iter().next() else {
        return Ok(None);
    };

    let package_name = format!("Foxide-{version}-windows-x64.zip");
    let assets = release
        .get("assets")
        .and_then(Value::as_array)
        .ok_or("Release has no asset list")?;
    let package = assets
        .iter()
        .find(|asset| asset.get("name").and_then(Value::as_str) == Some(&package_name))
        .ok_or("Release has no Windows package")?;
    let package_size = package
        .get("size")
        .and_then(Value::as_u64)
        .filter(|size| (1..=MAX_PACKAGE_SIZE).contains(size))
        .ok_or("Windows package size is invalid")?;
    let package_url = checked_asset_url(
        package
            .get("browser_download_url")
            .and_then(Value::as_str)
            .ok_or("Windows package URL is missing")?,
        &tag,
        &package_name,
    )?;
    let checksums = assets
        .iter()
        .find(|asset| asset.get("name").and_then(Value::as_str) == Some("SHA256SUMS.txt"))
        .ok_or("Release has no checksum file")?;
    let checksum_size = checksums
        .get("size")
        .and_then(Value::as_u64)
        .filter(|size| (1..=MAX_CHECKSUM_SIZE).contains(size))
        .ok_or("Checksum file size is invalid")?;
    let _ = checksum_size;
    let checksums_url = checked_asset_url(
        checksums
            .get("browser_download_url")
            .and_then(Value::as_str)
            .ok_or("Checksum file URL is missing")?,
        &tag,
        "SHA256SUMS.txt",
    )?;
    Ok(Some(WindowsUpdate {
        version: version.to_string(),
        tag,
        package_url,
        package_size,
        checksums_url,
    }))
}

fn checked_asset_url(text: &str, tag: &str, filename: &str) -> Result<Url, String> {
    let url = Url::parse(text).map_err(|_| "Invalid release asset URL")?;
    if url.scheme() != "https"
        || url.host_str() != Some(RELEASES_HOST)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != format!("/{REPOSITORY}/releases/download/{tag}/{filename}")
    {
        return Err("Release asset URL is outside the expected GitHub release".into());
    }
    Ok(url)
}

/// Parse one standard sha256sum entry for the requested exact filename.
pub fn parse_sha256sum(contents: &str, filename: &str) -> Result<[u8; 32], String> {
    let mut found = None;
    for line in contents.lines() {
        let mut fields = line.split_whitespace();
        let Some(hash) = fields.next() else { continue };
        let Some(name) = fields.next().map(|name| name.trim_start_matches('*')) else {
            continue;
        };
        if name != filename {
            continue;
        }
        if found.is_some() || hash.len() != 64 {
            return Err("Checksum entry is missing, duplicated, or malformed".into());
        }
        let mut bytes = [0; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hash[index * 2..index * 2 + 2], 16)
                .map_err(|_| "Checksum is not hexadecimal")?;
        }
        found = Some(bytes);
    }
    found.ok_or_else(|| "Checksum entry was not found".into())
}

/// Verify downloaded bytes against both the release metadata and its checksum asset.
pub fn verify_windows_package(
    update: &WindowsUpdate,
    package: &[u8],
    checksums: &str,
) -> Result<(), String> {
    if package.len() as u64 != update.package_size {
        return Err("Downloaded package size does not match release metadata".into());
    }
    let filename = format!("Foxide-{}-windows-x64.zip", update.version);
    let expected = parse_sha256sum(checksums, &filename)?;
    use sha2::Digest;
    let actual = sha2::Sha256::digest(package);
    if actual.as_slice() != expected {
        return Err("Downloaded package checksum does not match".into());
    }
    Ok(())
}

/// Download and verify a release, returning its staged archive only after both
/// release size metadata and the published SHA-256 file agree.
pub fn download_and_stage_windows_update(
    client: &CurlClient,
    update: &WindowsUpdate,
    staging_directory: &Path,
    cancel: &AtomicBool,
) -> Result<std::path::PathBuf, String> {
    std::fs::create_dir_all(staging_directory).map_err(|error| error.to_string())?;
    let filename = format!("Foxide-{}-windows-x64.zip", update.version);
    let target = staging_directory.join(&filename);
    let temporary = staging_directory.join(format!(".{filename}.download"));
    let checksums = client.get(&update.checksums_url, cancel)?;
    if checksums.len() > MAX_CHECKSUM_SIZE as usize {
        return Err("Checksum file is too large".into());
    }
    let checksums = std::str::from_utf8(&checksums).map_err(|_| "Checksum file is not UTF-8")?;
    client.download_file(&update.package_url, &temporary, update.package_size, cancel)?;
    let package = match std::fs::read(&temporary) {
        Ok(package) => package,
        Err(error) => {
            let _ = std::fs::remove_file(&temporary);
            return Err(error.to_string());
        }
    };
    if let Err(error) = verify_windows_package(update, &package, checksums) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    publish_staged_archive(&temporary, &target)?;
    Ok(target)
}

fn publish_staged_archive(temporary: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        std::fs::remove_file(target).map_err(|error| error.to_string())?;
    }
    std::fs::rename(temporary, target).map_err(|error| {
        let _ = std::fs::remove_file(temporary);
        error.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::publish_staged_archive;
    use std::{fs, time::SystemTime};

    #[test]
    fn failed_archive_publish_removes_the_temporary_download() {
        let directory = std::env::temp_dir().join(format!(
            "foxide-update-publish-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let temporary = directory.join(".update.download");
        fs::write(&temporary, b"verified archive").unwrap();
        let target = directory.join("missing-parent/update.zip");

        assert!(publish_staged_archive(&temporary, &target).is_err());
        assert!(!temporary.exists());

        fs::remove_dir_all(directory).unwrap();
    }
}
