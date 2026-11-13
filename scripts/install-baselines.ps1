param([switch]$Install)
$ErrorActionPreference = 'Stop'
Write-Output 'Required Windows tools: Rust, Python 3.11+, FFmpeg, VLC, mpv; optional PresentMon and Windows Performance Toolkit.'
Write-Output 'Modern Media Player is installed through Windows/Microsoft Store; record its Appx version separately.'
if (-not $Install) {
    Write-Output 'To install missing third-party baselines with winget: .\scripts\install-baselines.ps1 -Install'
    exit 0
}
& winget install --exact --id VideoLAN.VLC --source winget
if ($LASTEXITCODE -ne 0) { throw 'winget did not complete for VLC' }
Write-Output 'Install CLI mpv from a build linked at https://mpv.io/installation/ and pass its mpv.exe through --exe.'
Write-Output 'Keep portable tools in tools/ and record the archive URL, SHA-256 and --version in your session manifest.'
