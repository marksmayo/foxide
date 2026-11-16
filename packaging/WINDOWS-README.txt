Foxide Windows x64 preview

1. Extract this entire ZIP to a writable folder.
2. Install Python 3.11+ and 7-Zip if they are not already available.
3. In PowerShell, run:
   powershell -NoProfile -File .\Install-PlaybackRuntime.ps1
4. Launch Foxide.exe, then use Media > Open files.

The setup script downloads the pinned libmpv development archive, verifies its
SHA-256 and extracts its DLL into tools/libmpv inside this folder. It requires
internet access for that one-time download. No admin rights, file associations,
PATH changes or automatic updates are requested. Existing runtime installations
are preserved; the installer refuses to overwrite them.

No libmpv DLL is redistributed in this ZIP: the existing development archive has
no notices/source manifest and its redistribution audit remains unfinished.
BUILD-INFO.json records this executable's source commit/hash and a Cargo
dependency license inventory. notices/ contains available dependency notice files;
the inventory is a superset of the desktop's dependencies and is not a completed
license audit. Foxide's own source license does not replace dependency terms.

This preview is unsigned. Windows may show an unknown-publisher prompt.
Experimental ANGLE rendering is not included in the main-branch release.
