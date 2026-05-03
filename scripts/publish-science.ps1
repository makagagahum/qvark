[CmdletBinding()]
param(
    [string]$Repo = "bbrainfuckk/qorx",
    [string]$OriginUrl = "https://github.com/bbrainfuckk/qorx",
    [string]$Version = "",
    [switch]$CreateGitHubRelease,
    [switch]$SkipSoftwareHeritage,
    [switch]$PollSoftwareHeritage,
    [string]$ReportPath = "docs/papers/publication-report.md"
)

$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "==> $Message"
}

function Get-GitSha {
    $sha = (git rev-parse HEAD).Trim()
    if (-not $sha) {
        throw "Could not resolve git HEAD."
    }
    return $sha
}

function Get-GitHubToken {
    if (-not [string]::IsNullOrWhiteSpace($env:GH_TOKEN)) {
        return $env:GH_TOKEN
    }
    if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_TOKEN)) {
        return $env:GITHUB_TOKEN
    }
    return $null
}

function Invoke-GitHubJson {
    param(
        [string]$Uri,
        [string]$Method = "Get",
        [object]$Body = $null
    )

    $token = Get-GitHubToken
    if ([string]::IsNullOrWhiteSpace($token)) {
        throw "GH_TOKEN or GITHUB_TOKEN is required for GitHub release creation."
    }

    $headers = @{
        "Accept"               = "application/vnd.github+json"
        "Authorization"        = "Bearer $token"
        "X-GitHub-Api-Version" = "2022-11-28"
        "User-Agent"           = "qorx-scientific-publisher"
    }

    $args = @{
        Uri        = $Uri
        Method     = $Method
        Headers    = $headers
        TimeoutSec = 60
    }
    if ($null -ne $Body) {
        $args.Body = ($Body | ConvertTo-Json -Depth 8)
        $args.ContentType = "application/json"
    }

    return Invoke-RestMethod @args
}

function New-QorxGitHubRelease {
    param(
        [string]$Tag,
        [string]$TargetSha
    )

    if ([string]::IsNullOrWhiteSpace($Tag)) {
        throw "-Version is required when -CreateGitHubRelease is used."
    }

    $existingUri = "https://api.github.com/repos/$Repo/releases/tags/$Tag"
    try {
        $existing = Invoke-GitHubJson -Uri $existingUri
        Write-Step "GitHub release already exists for $Tag"
        return $existing
    }
    catch {
        $statusCode = $null
        if ($_.Exception.Response) {
            $statusCode = [int]$_.Exception.Response.StatusCode
        }
        if ($statusCode -ne 404) {
            throw
        }
    }

    $bodyText = @"
Scientific publication release for Qorx.

This release is intended to trigger Zenodo archival and DOI versioning when the repository is enabled in Zenodo.

Evidence:
- Zenodo metadata: .zenodo.json
- Citation metadata: CITATION.cff
- Paper: docs/papers/qorx-ai-language-paper.md
- Handbook: docs/handbook/README.md
"@

    $body = @{
        tag_name         = $Tag
        target_commitish = $TargetSha
        name             = "Qorx $Tag"
        body             = $bodyText
        draft            = $false
        prerelease       = $true
    }

    Write-Step "Creating GitHub release $Tag"
    return Invoke-GitHubJson -Uri "https://api.github.com/repos/$Repo/releases" -Method "Post" -Body $body
}

function Request-SoftwareHeritageSave {
    param([string]$Url)

    $encodedOrigin = [System.Uri]::EscapeDataString($Url)
    $uri = "https://archive.softwareheritage.org/api/1/origin/save/?visit_type=git&origin_url=$encodedOrigin"
    Write-Step "Requesting Software Heritage save for $Url"
    return Invoke-RestMethod -Uri $uri -Method Post -Headers @{
        "Accept"     = "application/json"
        "User-Agent" = "qorx-scientific-publisher"
    } -TimeoutSec 60
}

function Wait-SoftwareHeritageSave {
    param(
        [int]$RequestId,
        [int]$MaxPolls = 36
    )

    $uri = "https://archive.softwareheritage.org/api/1/origin/save/$RequestId/"
    for ($i = 0; $i -lt $MaxPolls; $i++) {
        $result = Invoke-RestMethod -Uri $uri -Headers @{
            "Accept"     = "application/json"
            "User-Agent" = "qorx-scientific-publisher"
        } -TimeoutSec 60

        Write-Host ("SWH request={0} task={1} visit={2} snapshot={3}" -f `
            $result.save_request_status, `
            $result.save_task_status, `
            $result.visit_status, `
            $result.snapshot_swhid)

        if ($result.save_task_status -in @("succeeded", "failed")) {
            return $result
        }
        Start-Sleep -Seconds 10
    }
    return $result
}

function Assert-PublishingMetadata {
    if (-not (Test-Path "CITATION.cff")) {
        throw "Missing CITATION.cff."
    }
    if (-not (Test-Path ".zenodo.json")) {
        throw "Missing .zenodo.json."
    }
    if ($Version -and (Test-Path ".zenodo.json")) {
        $zenodo = Get-Content ".zenodo.json" -Raw | ConvertFrom-Json
        $expected = $Version.TrimStart("v")
        if ($zenodo.version -ne $expected) {
            Write-Warning ".zenodo.json version '$($zenodo.version)' does not match requested release '$expected'. Update metadata before creating a formal DOI release if this is intentional."
        }
    }
}

function Write-PublishingReport {
    param(
        [string]$Sha,
        [object]$Release,
        [object]$SoftwareHeritage
    )

    $dir = Split-Path -Parent $ReportPath
    if (-not [string]::IsNullOrWhiteSpace($dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }

    $now = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $releaseUrl = if ($Release -and $Release.html_url) { $Release.html_url } else { "not created by this run" }
    $swhStatus = if ($SoftwareHeritage) { $SoftwareHeritage.save_task_status } else { "not requested" }
    $swhRequest = if ($SoftwareHeritage -and $SoftwareHeritage.request_url) { $SoftwareHeritage.request_url } else { "not available" }
    $swhid = if ($SoftwareHeritage -and $SoftwareHeritage.snapshot_swhid) { $SoftwareHeritage.snapshot_swhid } else { "not available yet" }

    $content = @"
# Scientific Publication Automation Report

Generated: $now

Repository: https://github.com/$Repo

Commit: $Sha

## Automated Surfaces

- GitHub release: $releaseUrl
- Zenodo: triggered by GitHub releases when Zenodo repository preservation is enabled.
- Software Heritage status: $swhStatus
- Software Heritage request: $swhRequest
- Software Heritage snapshot: $swhid

## Account-Bound Or Curated Surfaces

- OSF Preprints: requires OSF login, preprint service selection, and final submission consent.
- ORCID Works: requires authenticated ORCID ownership or a member API integration with permission.
- OpenAIRE: discovery/indexing surface; verify after Zenodo/DataCite harvesting.
- arXiv: requires registered author login, endorsement when applicable, moderation, and author agreement.
- JOSS: requires a mature research-software submission and GitHub-based editorial review.

## Local Metadata Inputs

- .zenodo.json
- CITATION.cff
- docs/papers/qorx-ai-language-paper.md
- docs/handbook/README.md
"@

    Set-Content -Path $ReportPath -Value $content -Encoding utf8
    Write-Step "Wrote $ReportPath"
}

Assert-PublishingMetadata
$sha = Get-GitSha
$release = $null

if ($CreateGitHubRelease) {
    $release = New-QorxGitHubRelease -Tag $Version -TargetSha $sha
}
else {
    Write-Step "GitHub release creation skipped"
}

$swh = $null
if ($SkipSoftwareHeritage) {
    Write-Step "Software Heritage save skipped"
}
else {
    $swh = Request-SoftwareHeritageSave -Url $OriginUrl
    if ($PollSoftwareHeritage -and $swh.id) {
        $swh = Wait-SoftwareHeritageSave -RequestId ([int]$swh.id)
    }
}

Write-PublishingReport -Sha $sha -Release $release -SoftwareHeritage $swh

$summary = [ordered]@{
    repo = $Repo
    commit = $sha
    github_release = if ($release -and $release.html_url) { $release.html_url } else { $null }
    software_heritage_request = if ($swh -and $swh.request_url) { $swh.request_url } else { $null }
    software_heritage_status = if ($swh) { $swh.save_task_status } else { $null }
    software_heritage_snapshot = if ($swh) { $swh.snapshot_swhid } else { $null }
    report = $ReportPath
}

$summary | ConvertTo-Json -Depth 6
