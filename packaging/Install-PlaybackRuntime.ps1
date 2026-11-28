# Download and verify the Windows x64 playback runtime without requiring a
# separately installed Python or archive utility.
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$archiveUri = 'https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20261111/mpv-dev-x86_64-20261111-git-bcde6b93cc.7z'
$archiveSha256 = '273c079b11ea5575822f972756f4c9a71149a5c10130353abeb8586738938452'
$extractorUri = 'https://github.com/ip7z/7zip/releases/download/26.06/7zr.exe'
$extractorSha256 = 'e80db2dad715a6c5adabe292d31ed54e11f4e9b99ca349827b50573b175891bc'

$appRoot = if ((Split-Path -Leaf $PSScriptRoot) -eq 'packaging') {
    Split-Path -Parent $PSScriptRoot
} else {
    $PSScriptRoot
}
$tools = Join-Path $appRoot 'tools'
$target = Join-Path $tools 'libmpv'
if (Test-Path -LiteralPath $target) {
    throw 'Refusing to overwrite tools/libmpv; inspect or remove the existing runtime before retrying.'
}

New-Item -ItemType Directory -Path $tools -Force | Out-Null
$work = Join-Path $tools ('.runtime-install-' + [Guid]::NewGuid().ToString('N'))
$stage = Join-Path $work 'libmpv'
New-Item -ItemType Directory -Path $stage -Force | Out-Null
try {
    $extractor = Join-Path $work '7zr.exe'
    Invoke-WebRequest -Uri $extractorUri -OutFile $extractor -TimeoutSec 600
    $actualExtractorHash = (Get-FileHash -LiteralPath $extractor -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualExtractorHash -ne $extractorSha256) {
        throw 'The standalone archive extractor failed SHA-256 verification.'
    }

    $archive = Join-Path $work 'libmpv.7z'
    Invoke-WebRequest -Uri $archiveUri -OutFile $archive -TimeoutSec 600
    $actualArchiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualArchiveHash -ne $archiveSha256) {
        throw 'The libmpv archive failed SHA-256 verification.'
    }

    & $extractor e -y "-o$stage" $archive 'libmpv-2.dll'
    if ($LASTEXITCODE -ne 0) {
        throw "The verified archive extractor exited with code $LASTEXITCODE."
    }
    $dll = Join-Path $stage 'libmpv-2.dll'
    if (-not (Test-Path -LiteralPath $dll -PathType Leaf) -or (Get-Item -LiteralPath $dll).Length -lt 1024) {
        throw 'The verified archive did not contain a usable libmpv-2.dll.'
    }

    $dllSha256 = (Get-FileHash -LiteralPath $dll -Algorithm SHA256).Hash.ToLowerInvariant()
    $manifest = [ordered]@{
        source = $archiveUri
        archive_sha256 = $archiveSha256
        extractor = $extractorUri
        extractor_sha256 = $extractorSha256
        dll_sha256 = $dllSha256
        notes = 'Third-party development build; redistribution license audit required.'
    }
    $manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stage 'installation.json') -Encoding UTF8
    Move-Item -LiteralPath $stage -Destination $target
    Write-Output 'Playback runtime verified and installed. Launch Foxide.'
}
finally {
    $toolsPrefix = [IO.Path]::GetFullPath($tools) + [IO.Path]::DirectorySeparatorChar
    $workFullPath = [IO.Path]::GetFullPath($work)
    if (Test-Path -LiteralPath $work) {
        if (-not $workFullPath.StartsWith($toolsPrefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to clean a temporary path outside tools/: $workFullPath"
        }
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    }
}
