#[cfg(any(windows, test))]
#[derive(Debug, PartialEq, Eq)]
enum InstallOffer {
    NotNeeded,
    Declined,
    Installed,
}

#[cfg(any(windows, test))]
fn maybe_install(
    runtime_missing: bool,
    confirm: impl FnOnce() -> bool,
    install: impl FnOnce() -> Result<(), String>,
) -> Result<InstallOffer, String> {
    if !runtime_missing {
        return Ok(InstallOffer::NotNeeded);
    }
    if !confirm() {
        return Ok(InstallOffer::Declined);
    }
    install()?;
    Ok(InstallOffer::Installed)
}

#[cfg(windows)]
pub(super) fn offer_install_if_missing() -> Result<(), String> {
    use futures_lite::future::block_on;
    use rfd::{AsyncMessageDialog, MessageButtons, MessageDialogResult, MessageLevel};

    maybe_install(
        player_render::windows_runtime_missing(),
        || {
            matches!(
                block_on(
                    AsyncMessageDialog::new()
                        .set_level(MessageLevel::Warning)
                        .set_title("Foxide playback runtime missing")
                        .set_description(
                            "Foxide needs its Windows playback runtime before it can open media.\n\nInstall it now? Setup downloads a pinned libmpv runtime and requires Python 3.11+ and 7-Zip. It does not require administrator access.",
                        )
                        .set_buttons(MessageButtons::YesNo)
                        .show()
                ),
                MessageDialogResult::Yes
            )
        },
        install_windows_runtime,
    )?;
    Ok(())
}

#[cfg(windows)]
fn install_windows_runtime() -> Result<(), String> {
    use std::{env, path::PathBuf, process::Command};

    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let application_dir = executable
        .parent()
        .ok_or("Foxide executable has no parent directory")?;
    let script = application_dir.join("Install-PlaybackRuntime.ps1");
    if !script.is_file() {
        return Err(format!(
            "The packaged runtime installer is missing: {}",
            script.display()
        ));
    }

    let windows_dir = env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "C:\\Windows".into());
    let powershell = windows_dir.join("System32/WindowsPowerShell/v1.0/powershell.exe");
    if !powershell.is_file() {
        return Err(format!(
            "Windows PowerShell could not be found at {}",
            powershell.display()
        ));
    }

    let output = Command::new(powershell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(script)
        .output()
        .map_err(|error| format!("Could not start the playback runtime installer: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let details = String::from_utf8_lossy(&output.stderr);
    let details = details.trim();
    if details.is_empty() {
        Err(format!(
            "Playback runtime installation exited with {}. Install Python 3.11+ and 7-Zip, then retry from Start Menu > Foxide > Install playback runtime.",
            output.status
        ))
    } else {
        Err(format!("Playback runtime installation failed: {details}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{InstallOffer, maybe_install};
    use std::cell::Cell;

    #[test]
    fn installed_runtime_skips_the_prompt_and_installer() {
        let prompted = Cell::new(false);
        let installed = Cell::new(false);

        let result = maybe_install(
            false,
            || {
                prompted.set(true);
                true
            },
            || {
                installed.set(true);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(result, InstallOffer::NotNeeded);
        assert!(!prompted.get());
        assert!(!installed.get());
    }

    #[test]
    fn missing_runtime_only_installs_after_the_user_accepts() {
        let installed = Cell::new(false);

        let result = maybe_install(
            true,
            || true,
            || {
                installed.set(true);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(result, InstallOffer::Installed);
        assert!(installed.get());
    }

    #[test]
    fn declining_runtime_install_leaves_playback_setup_untouched() {
        let installed = Cell::new(false);

        let result = maybe_install(
            true,
            || false,
            || {
                installed.set(true);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(result, InstallOffer::Declined);
        assert!(!installed.get());
    }

    #[test]
    fn installer_failure_is_returned_to_the_startup_flow() {
        let result = maybe_install(true, || true, || Err("download failed".into()));

        assert_eq!(result.unwrap_err(), "download failed");
    }
}
