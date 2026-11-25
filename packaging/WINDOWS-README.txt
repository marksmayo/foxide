Foxide Windows x64 preview

Choose either the Windows setup installer or the ZIP download.

Setup installer:
1. Run Foxide-VERSION-windows-x64-setup.exe. It installs for the current user
   under LocalAppData\Programs\Foxide; no administrator rights are required.
2. Launch Foxide from the Start Menu. If the playback runtime is missing,
   Foxide offers to install it. The setup needs Python 3.11+ and 7-Zip. You can
   also run Start Menu > Foxide > Install playback runtime.
3. The installer can optionally create a
   desktop shortcut and does not change file associations.

ZIP download:
1. Extract the entire ZIP to a writable folder.
2. Launch Foxide.exe. If the playback runtime is missing, Foxide offers to
   install it. The setup needs Python 3.11+ and 7-Zip.
3. You can also install manually in PowerShell with:
   powershell -NoProfile -File .\Install-PlaybackRuntime.ps1

Both downloads install the playback runtime separately. The setup script
downloads the pinned libmpv development archive, verifies its SHA-256 and
extracts its DLL into tools/libmpv. It requires internet access for that
one-time download. Existing runtime installations are preserved; the installer
refuses to overwrite them. The setup installer and ZIP both install/run without
elevation and do not modify PATH or create file associations.

No libmpv DLL is redistributed: the existing development archive has no
notices/source manifest and its redistribution audit remains unfinished.
BUILD-INFO.json records the executable's source commit/hash and a Cargo
dependency license inventory. notices/ contains available dependency notice
files; the inventory is a superset of the desktop dependencies and is not a
completed license audit. Foxide's own source license does not replace
dependency terms.

This preview is unsigned. Windows may show an unknown-publisher prompt.
Experimental ANGLE rendering is not included in the main-branch release.
