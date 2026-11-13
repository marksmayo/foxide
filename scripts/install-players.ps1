param(
    [switch]$Install,
    [string[]]$Players,
    [string]$Output = 'results/local/player-installation.json'
)
$ErrorActionPreference = 'Stop'
$scriptPath = Join-Path $PSScriptRoot 'install-players.py'
$installerArgs = @($scriptPath, '--output', $Output)
if ($Install) { $installerArgs += '--install' }
if ($Players) { $installerArgs += '--players'; $installerArgs += $Players }
& python @installerArgs
exit $LASTEXITCODE
