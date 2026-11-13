$ErrorActionPreference = 'Stop'
# This repair/bootstrap is intentionally restricted to ephemeral hosted CI VMs.
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') {
    throw 'This bootstrap is for GitHub-hosted Windows runners only. It is not run on the user machine.'
}
$usable = $false
if (Get-Command winget -ErrorAction SilentlyContinue) {
    & winget --version
    $usable = $LASTEXITCODE -eq 0
}
if (-not $usable) {
    Install-PackageProvider -Name NuGet -Force | Out-Null
    Install-Module -Name Microsoft.WinGet.Client -Force -Repository PSGallery -Scope CurrentUser
    Import-Module Microsoft.WinGet.Client
    Repair-WinGetPackageManager -AllUsers
}
& winget --info
if ($LASTEXITCODE -ne 0) { throw 'WinGet remains unavailable on this hosted runner' }
