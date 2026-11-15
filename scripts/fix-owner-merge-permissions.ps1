[CmdletBinding()]
param([switch]$Apply)

# Run in your normal authenticated PowerShell. No tokens are read or printed.
# Preview by default; -Apply changes only the two owner-exception layers.
$ErrorActionPreference = 'Stop'
$repository = 'marksmayo/foxide'
$ownerId = 14829262
$coreId = 27309208
$exceptionIds = @(27309400, 27309211)
$expectedNames = @{
    27309400 = 'Foxide required reviews - owner exception'
    27309211 = 'Foxide owner-only main merges'
}

function Invoke-GitHubJson {
    param([string[]]$Arguments)
    $response = & gh api --hostname github.com --header 'Accept: application/vnd.github+json' --header 'X-GitHub-Api-Version: 2026-03-10' @Arguments
    if ($LASTEXITCODE -ne 0) { throw ('GitHub API request failed: ' + ($Arguments -join ' ')) }
    return (($response -join [Environment]::NewLine) | ConvertFrom-Json)
}

function Get-Policy {
    param($Ruleset)
    return [ordered]@{
        name = $Ruleset.name
        target = $Ruleset.target
        enforcement = $Ruleset.enforcement
        conditions = $Ruleset.conditions
        rules = @($Ruleset.rules)
        bypass_actors = @($Ruleset.bypass_actors)
    }
}

function Convert-PolicyJson {
    param($Value)
    return ($Value | ConvertTo-Json -Depth 100 -Compress)
}

function Write-Json {
    param($Value, [string]$Path)
    [IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 100), (New-Object Text.UTF8Encoding($false)))
}

Get-Command gh -ErrorAction Stop | Out-Null
$identity = Invoke-GitHubJson -Arguments @('user')
if ($identity.login -ne 'marksmayo' -or $identity.id -ne $ownerId) {
    throw 'Authenticate gh as marksmayo before running this script.'
}
$repo = Invoke-GitHubJson -Arguments @("repos/$repository")
if ($repo.full_name -ne $repository -or $repo.owner.id -ne $ownerId -or -not $repo.permissions.admin) {
    throw 'Repository owner/admin verification failed.'
}

$core = Invoke-GitHubJson -Arguments @("repos/$repository/rulesets/$coreId")
if ($core.name -ne 'Foxide main protection' -or $core.enforcement -ne 'active' -or @($core.bypass_actors).Count -ne 0) {
    throw 'Unexpected core protection: refusing to proceed.'
}
foreach ($kind in @('pull_request', 'required_status_checks', 'deletion', 'non_fast_forward')) {
    if (@($core.rules.type) -notcontains $kind) { throw "Missing mandatory core rule: $kind" }
}
$pull = @($core.rules | Where-Object { $_.type -eq 'pull_request' })
if ($pull.Count -ne 1 -or $pull[0].parameters.required_approving_review_count -ne 0 -or
    $pull[0].parameters.require_last_push_approval -or $pull[0].parameters.require_code_owner_review -or
    $pull[0].parameters.require_extra_approval_for_unattributed_changes -or
    @($pull[0].parameters.required_reviewers).Count -gt 0 -or
    -not $pull[0].parameters.required_review_thread_resolution) {
    throw 'The core PR policy has changed. This script only repairs the existing isolated owner exemptions.'
}
$originalCore = Convert-PolicyJson (Get-Policy $core)
$plans = @()
foreach ($id in $exceptionIds) {
    $rule = Invoke-GitHubJson -Arguments @("repos/$repository/rulesets/$id")
    $actors = @($rule.bypass_actors)
    $expectedType = if ($id -eq 27309400) { 'pull_request' } else { 'update' }
    if ($rule.name -ne $expectedNames[$id] -or $rule.target -ne 'branch' -or $rule.enforcement -ne 'active' -or
        @($rule.rules).Count -ne 1 -or $rule.rules[0].type -ne $expectedType -or
        $actors.Count -ne 1 -or $actors[0].actor_type -ne 'User' -or $actors[0].actor_id -ne $ownerId -or
        $actors[0].bypass_mode -notin @('pull_request', 'always', 'exempt') -or
        (Convert-PolicyJson $rule.conditions) -ne (Convert-PolicyJson $core.conditions)) {
        throw "Unexpected owner ruleset $id. No changes have been made."
    }
    $payload = Get-Policy $rule
    # 'exempt' skips evaluation for this user, rather than requesting an
    # interactive bypass. The non-bypassable core still requires a PR + CI.
    $payload.bypass_actors = @(@{ actor_id = $ownerId; actor_type = 'User'; bypass_mode = 'exempt' })
    $plans += [PSCustomObject]@{ Id = $id; Before = $rule; Payload = $payload }
    Write-Output ("{0}: marksmayo {1} -> exempt" -f $rule.name, $actors[0].bypass_mode)
}

if (-not $Apply) {
    Write-Output 'Preview only. Run again with -Apply to save these two owner exemptions.'
    return
}

$repoRoot = Split-Path $PSScriptRoot -Parent
$backup = Join-Path $repoRoot ('results/local/repository-admin/owner-exempt-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $backup -Force | Out-Null
Write-Json $core (Join-Path $backup 'core-before.json')
foreach ($plan in $plans) { Write-Json $plan.Before (Join-Path $backup ("ruleset-{0}-before.json" -f $plan.Id)) }

foreach ($plan in $plans) {
    $current = Invoke-GitHubJson -Arguments @("repos/$repository/rulesets/$($plan.Id)")
    if ((Convert-PolicyJson (Get-Policy $current)) -ne (Convert-PolicyJson (Get-Policy $plan.Before))) {
        throw "Ruleset $($plan.Id) changed during preparation; refusing to overwrite it. Backups: $backup"
    }
    if ($current.bypass_actors[0].bypass_mode -ne 'exempt') {
        $path = Join-Path $backup ("ruleset-{0}-payload.json" -f $plan.Id)
        Write-Json $plan.Payload $path
        $null = Invoke-GitHubJson -Arguments @("repos/$repository/rulesets/$($plan.Id)", '--method', 'PUT', '--input', $path)
    }
    $verified = Invoke-GitHubJson -Arguments @("repos/$repository/rulesets/$($plan.Id)")
    $actor = @($verified.bypass_actors)
    if ($actor.Count -ne 1 -or $actor[0].actor_type -ne 'User' -or $actor[0].actor_id -ne $ownerId -or
        $actor[0].bypass_mode -ne 'exempt' -or
        (Convert-PolicyJson $verified.rules) -ne (Convert-PolicyJson $plan.Before.rules) -or
        (Convert-PolicyJson $verified.conditions) -ne (Convert-PolicyJson $plan.Before.conditions) -or
        $verified.enforcement -ne 'active') {
        throw "Ruleset $($plan.Id) failed verification. Backups: $backup"
    }
}
$finalCore = Invoke-GitHubJson -Arguments @("repos/$repository/rulesets/$coreId")
if ((Convert-PolicyJson (Get-Policy $finalCore)) -ne $originalCore) {
    throw "Core policy changed concurrently. Inspect the saved snapshot: $backup"
}
Write-Output "Saved and verified owner exemptions. Core protections are unchanged. Backups: $backup"
Write-Output 'Refresh the PR while signed in as marksmayo. Required checks and resolved conversations still apply.'
