use crate::PlayerWindow;
use slint::ComponentHandle;
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[cfg(windows)]
const RELEASES_API: &str = "https://api.github.com/repos/marksmayo/foxide/releases?per_page=20";

pub fn install(ui: &PlayerWindow) {
    let staged: Arc<Mutex<Option<(String, PathBuf)>>> = Arc::default();
    let busy = Arc::new(AtomicBool::new(false));
    let weak = ui.as_weak();
    let begin_check = {
        let staged = staged.clone();
        let busy = busy.clone();
        move || {
            if busy.swap(true, Ordering::AcqRel) {
                return;
            }
            let staged = staged.clone();
            let busy = busy.clone();
            let weak = weak.clone();
            #[cfg(windows)]
            let updates_directory = std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|path| path.join("Foxide/Updates"));
            std::thread::spawn(move || {
                #[cfg(not(windows))]
                let result: Result<Option<(String, PathBuf)>, String> = Err(
                    "Automatic updates are currently available for Windows releases only.".into(),
                );
                #[cfg(windows)]
                let result: Result<Option<(String, PathBuf)>, String> = (|| {
                    let client = player_services::CurlClient::system()?;
                    let cancel = AtomicBool::new(false);
                    let url =
                        player_services::Url::parse(RELEASES_API).map_err(|e| e.to_string())?;
                    let bytes = player_services::HttpClient::get(&client, &url, &cancel)?;
                    if bytes.len() > 2 * 1024 * 1024 {
                        return Err("Release list is too large".into());
                    }
                    let release_version =
                        option_env!("FOXIDE_RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
                    let Some(update) = player_services::latest_windows_update(
                        std::str::from_utf8(&bytes).map_err(|_| "Release list is not UTF-8")?,
                        release_version,
                    )?
                    else {
                        return Ok(None);
                    };
                    let directory =
                        updates_directory.ok_or("Update staging path is unavailable")?;
                    let path = player_services::download_and_stage_windows_update(
                        &client, &update, &directory, &cancel,
                    )?;
                    Ok(Some((update.version, path)))
                })();
                busy.store(false, Ordering::Release);
                match result {
                    Ok(Some((version, path))) => {
                        if let Ok(mut staged) = staged.lock() {
                            *staged = Some((version.clone(), path));
                        }
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = weak.upgrade() {
                                ui.set_staged_update_version(version.into());
                                ui.set_update_status(
                                    "Update downloaded and checksum-checked.".into(),
                                );
                            }
                        });
                    }
                    Ok(None) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = weak.upgrade() {
                                ui.set_update_status("Foxide is up to date.".into());
                            }
                        });
                    }
                    Err(error) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = weak.upgrade() {
                                ui.set_update_status(
                                    format!("Update check failed: {error}").into(),
                                );
                            }
                        });
                    }
                }
            });
        }
    };
    let check = begin_check.clone();
    let weak = ui.as_weak();
    ui.on_check_updates(move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_update_status("Checking for updates…".into());
        }
        check();
    });
    let install_staged = staged;
    let weak = ui.as_weak();
    ui.on_install_update(move || {
        let result = install_staged
            .lock()
            .map_err(|_| "Update state unavailable".to_owned())
            .and_then(|state| {
                state
                    .clone()
                    .ok_or_else(|| "No verified update is staged".to_owned())
            })
            .and_then(|(_, archive)| launch_installer(&archive));
        match result {
            Ok(()) => {
                let _ = slint::quit_event_loop();
            }
            Err(error) => {
                if let Some(ui) = weak.upgrade() {
                    ui.set_update_status(format!("Could not start update: {error}").into());
                }
            }
        }
    });
    ui.set_update_status("Checking for updates…".into());
    begin_check();
}

#[cfg(windows)]
fn launch_installer(archive: &std::path::Path) -> Result<(), String> {
    use std::process::Command;
    const SCRIPT: &str = r#"param([string]$Archive,[string]$Executable,[int]$ParentPid,[string]$Version)
$ErrorActionPreference = 'Stop'
Wait-Process -Id $ParentPid -ErrorAction SilentlyContinue
$destination = Split-Path -Parent $Executable
$temporary = Join-Path $env:TEMP ('Foxide-update-' + [guid]::NewGuid().ToString('N'))
try {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $prefix = 'Foxide-' + $Version + '-windows-x64/'
        foreach ($entry in $zip.Entries) {
            if (!$entry.FullName.StartsWith($prefix, [StringComparison]::Ordinal) -or $entry.FullName.Contains([char]92)) { throw 'Package contains an unexpected path' }
            $candidate = [IO.Path]::GetFullPath((Join-Path $temporary $entry.FullName))
            $rootTemp = [IO.Path]::GetFullPath($temporary).TrimEnd([char]92) + [char]92
            if (!$candidate.StartsWith($rootTemp, [StringComparison]::OrdinalIgnoreCase)) { throw 'Package path escaped its root' }
        }
    } finally { $zip.Dispose() }
    Expand-Archive -LiteralPath $Archive -DestinationPath $temporary
    $package = Join-Path $temporary ('Foxide-' + $Version + '-windows-x64')
    $newExe = Join-Path $package 'Foxide.exe'
    if (!(Test-Path -LiteralPath $newExe -PathType Leaf)) { throw 'Package executable is missing' }
    $root = [IO.Path]::GetFullPath($package).TrimEnd([char]92) + [char]92
    Get-ChildItem -LiteralPath $package -File -Recurse | ForEach-Object {
        $source = [IO.Path]::GetFullPath($_.FullName)
        if (!$source.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) { throw 'Package path escaped its root' }
        $relative = $source.Substring($root.Length)
        $target = Join-Path $destination $relative
        $folder = Split-Path -Parent $target
        New-Item -ItemType Directory -Force -Path $folder | Out-Null
        Copy-Item -LiteralPath $source -Destination $target -Force
    }
    Start-Process -FilePath $Executable
} finally {
    Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}"#;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let version = archive
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| {
            name.strip_prefix("Foxide-")?
                .strip_suffix("-windows-x64.zip")
        })
        .ok_or("Staged update filename is invalid")?;
    if !version
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        return Err("Staged update version is invalid".into());
    }
    let helper = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or("Local application data path is unavailable")?
        .join("Foxide/Updates/apply-update.ps1");
    std::fs::create_dir_all(helper.parent().ok_or("Invalid updater path")?)
        .map_err(|error| error.to_string())?;
    std::fs::write(&helper, SCRIPT).map_err(|error| error.to_string())?;
    let powershell = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .ok_or("Windows system path is unavailable")?
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    Command::new(powershell)
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-File"])
        .arg(helper)
        .arg("-Archive")
        .arg(archive)
        .arg("-Executable")
        .arg(executable)
        .arg("-ParentPid")
        .arg(std::process::id().to_string())
        .arg("-Version")
        .arg(version)
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(not(windows))]
fn launch_installer(_: &std::path::Path) -> Result<(), String> {
    Err("Automatic updates are currently available for Windows releases only.".into())
}
