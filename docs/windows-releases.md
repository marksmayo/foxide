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
- Foxide-VERSION-windows-x64.zip: Foxide.exe, runtime setup scripts, README, project license, available Cargo dependency notices and BUILD-INFO.json.
- Foxide-source.zip: exact tagged repository source; dependency sources are identified separately by Cargo.lock and the inventory.
- SHA256SUMS.txt: both ZIP hashes.
- BUILD-INFO.json: version, source commit, executable hash and dependency license inventory.

Playback setup after extraction:
```powershell
powershell -NoProfile -File .\Install-PlaybackRuntime.ps1
.\Foxide.exe
```
Python3.11+ and7-Zip are required for the one-time runtime download. The existing pinned installer verifies the upstream archive SHA-256 before extracting libmpv into tools/libmpv. It refuses to overwrite an existing runtime.

The native runtime is not bundled in this first package. Its existing development archive contains no notices/source manifest and records an unfinished redistribution audit. The generated Cargo inventory is a dependency superset, not a legal compliance approval. License/source obligations and unsigned-binary acceptance remain release-review work. The pipeline does not grant LGPL status to GPL-enabled builds.

Pull requests exercise packaging and upload test artifacts; they never create a release. Current packaging targets Windows x64 and the main renderer, with no experimental ANGLE binaries or installer/file-association changes.

[GitHub release management](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)
