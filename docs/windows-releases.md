# Windows preview releases

Merge the release-packaging workflow first. Update workspace.package.version in Cargo.toml (and Cargo.lock when needed) in a reviewed PR before choosing a new base version.

To package the current0.0.1 preview from main:

```powershell
git switch main
git pull --ff-only origin main
git tag -a v0.0.1-preview.1 -m "Foxide Windows preview"
git push origin v0.0.1-preview.1
```

The **Windows release packaging** workflow builds Windows x64, runs tests, packages the desktop, verifies the extracted executable through a layout preview, and creates a **draft** GitHub release. It does not publish automatically. Review the assets, notices and release notes in Releases before publishing.

Manual retries: Actions → Windows release packaging → Run workflow → select main → supply the existing tag. The workflow checks out that exact tag, verifies its commit is on main, and requires the tag's base version to match Cargo.toml. A published release's assets are never replaced; retries may update a draft. No personal access token is required: only the draft-upload job receives contents:write.

Assets:
- Foxide-VERSION-windows-x64-setup.exe: per-user installer with Start Menu shortcuts, an optional desktop shortcut, uninstaller, runtime setup script, README, project license, available dependency notices and BUILD-INFO.json. It does not request elevation, create file associations, or bundle libmpv.
- Foxide-VERSION-windows-x64.zip: the same application payload for manual extraction.
- Foxide-source.zip: exact tagged repository source; dependency sources are identified separately by Cargo.lock and the inventory.
- SHA256SUMS.txt: hashes for the setup installer and both ZIP files.
- BUILD-INFO.json: version, source commit, executable hash and dependency license inventory.

Playback setup after extraction:
```powershell
powershell -NoProfile -File .\Install-PlaybackRuntime.ps1
.\Foxide.exe
```
Python3.11+ and7-Zip are required for the one-time runtime download. The existing pinned installer verifies the upstream archive SHA-256 before extracting libmpv into tools/libmpv. It refuses to overwrite an existing runtime.

The native runtime is not bundled in this first package. Its existing development archive contains no notices/source manifest and records an unfinished redistribution audit. The generated Cargo inventory is a dependency superset, not a legal compliance approval. License/source obligations and unsigned-binary acceptance remain release-review work. The pipeline does not grant LGPL status to GPL-enabled builds.

Pull requests exercise packaging and upload test artifacts; they never create a release. Current packaging targets Windows x64 and the main renderer. The installer is per-user and does not add file associations or experimental ANGLE binaries.

## Update roadmap

- **Windows:** check published releases in the background, automatically download the Windows x64 package, verify its published SHA-256 entry and package size, then stage it under the user's local application data. Show the version and ask before restarting to install it. Draft releases are never offered. The client must stay on its current stable or preview channel.
- **macOS:** add a release package workflow with the signing and notarization policy defined before publishing. Add architecture-aware update metadata, verified automatic downloads, staging, and an explicit install/restart prompt. Decide universal versus separate Apple Silicon and Intel packages as part of that workflow.
- **Linux:** define supported distributions and package formats before publishing. Add per-format release metadata, verified automatic downloads, staging, and an explicit install/restart prompt that respects the package manager's ownership and permissions.

The setup installer is a new-install path; the automatic updater continues to download the ZIP and only stages/applies it after an explicit user action. Cross-platform release channels require their own provenance and verification tests. Update checks must never execute a downloaded installer automatically; staged updates are applied only after an explicit user action. The current Windows channel checks that the archive matches the size and SHA-256 published alongside it; this detects corruption and mismatched assets, but the checksum is not an independent publisher signature. Signed release provenance remains part of release hardening.

[GitHub release management](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)
