param(
    [Parameter(Mandatory=$true)][ValidateSet('mpv','vlc')][string]$Player,
    [Parameter(Mandatory=$true)][string]$PlayerExecutable,
    [string]$MediaDirectory = 'media/generated/full',
    [string]$Output = 'results/local/windows',
    [double]$Duration = 630,
    [double]$Warmup = 30,
    [int]$Repeat = 5,
    [switch]$Tuned
)
$ErrorActionPreference = 'Stop'
$manifest = Get-Content -Raw -LiteralPath (Join-Path $MediaDirectory 'manifest.json') | ConvertFrom-Json
if ($manifest.seconds -lt $Duration) { throw 'Media must be longer than the run; generate full fixtures first.' }
& cargo build --release --locked -p player-bench
if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
# Verify hashes BEFORE warm tests; do not do this before cold-reboot tests.
& python scripts/generate-media.py --verify-only --output $MediaDirectory
if ($LASTEXITCODE -ne 0) { throw 'Fixture verification failed' }
foreach ($fixture in $manifest.fixtures) {
    $benchArgs = @('run','--player',$Player,'--exe',$PlayerExecutable,
        '--media',(Join-Path $MediaDirectory $fixture.path),'--cache','warm',
        '--duration',"$Duration",'--warmup',"$Warmup",'--repeat',"$Repeat",'--output',$Output)
    if ($Tuned) { $benchArgs += '--tuned' }
    & .\target\release\player-bench.exe @benchArgs
    if ($LASTEXITCODE -ne 0) { throw "Benchmark incomplete for $($fixture.path); inspect logs." }
}
& .\target\release\player-bench.exe report $Output --output (Join-Path $Output 'comparison.md')
if ($LASTEXITCODE -ne 0) { throw 'Report generation failed' }
