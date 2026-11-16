# Run once after extracting the release ZIP. No elevation or PATH changes are requested.
$ErrorActionPreference = 'Stop'
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { throw 'Install Python 3.11 or newer, then run this script again.' }
$sevenZip = Join-Path $env:ProgramFiles '7-Zip/7z.exe'
if (-not (Test-Path -LiteralPath $sevenZip)) { throw 'Install 7-Zip, then run this script again.' }
& $python.Source (Join-Path $PSScriptRoot 'scripts/install-libmpv.py') --seven-zip $sevenZip
if ($LASTEXITCODE -ne 0) { throw 'Playback runtime installation failed. Inspect the error above.' }
Write-Output 'Playback runtime verified and installed. Launch Foxide.exe.'
