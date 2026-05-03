param(
    [string]$Exe = ".\target\release\qorx.exe",
    [switch]$SkipCargo
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$results = New-Object System.Collections.Generic.List[object]
$claimHits = New-Object System.Collections.Generic.List[object]
$secretHits = New-Object System.Collections.Generic.List[object]
$script:failed = $false

function Add-Result {
    param(
        [string]$Name,
        [string]$Status,
        [string]$Details = ""
    )
    $results.Add([pscustomobject]@{
        name = $Name
        status = $Status
        details = $Details
    }) | Out-Null
    if ($Status -eq "fail") {
        $script:failed = $true
    }
}

function Invoke-Native {
    param(
        [string]$File,
        [string[]]$ArgumentList,
        [string]$WorkDir = $repoRoot
    )
    $oldLocation = Get-Location
    $oldErrorAction = $ErrorActionPreference
    try {
        Set-Location -LiteralPath $WorkDir
        $ErrorActionPreference = "Continue"
        $output = & $File @ArgumentList 2>&1 | ForEach-Object { $_.ToString() }
        $exit = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
        $text = ($output -join "`n").Trim()
        if ($exit -ne 0) {
            throw "$File $($ArgumentList -join ' ') exited $exit`n$text"
        }
        return $text
    } finally {
        $ErrorActionPreference = $oldErrorAction
        Set-Location -LiteralPath $oldLocation
    }
}

function Run-Step {
    param(
        [string]$Name,
        [scriptblock]$Body
    )
    try {
        $details = & $Body
        Add-Result -Name $Name -Status "pass" -Details ([string]$details)
    } catch {
        Add-Result -Name $Name -Status "fail" -Details ([string]$_.Exception.Message)
    }
}

function Resolve-QorxExe {
    param([string]$Requested)
    if (Test-Path -LiteralPath $Requested) {
        return (Resolve-Path -LiteralPath $Requested).Path
    }
    $command = Get-Command $Requested -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }
    throw "Qorx executable not found: $Requested"
}

function Invoke-QorxJson {
    param(
        [string]$Name,
        [string[]]$ArgumentList
    )
    $oldHome = $env:QORX_HOME
    try {
        $env:QORX_HOME = $script:qorxHome
        $raw = Invoke-Native -File $script:exePath -ArgumentList $ArgumentList -WorkDir $repoRoot
        try {
            return ($raw | ConvertFrom-Json)
        } catch {
            throw "$Name did not return JSON: $raw"
        }
    } finally {
        $env:QORX_HOME = $oldHome
    }
}

function Get-RepoRelativePath {
    param([string]$Path)
    $base = [System.IO.Path]::GetFullPath($repoRoot).TrimEnd('\', '/')
    $full = [System.IO.Path]::GetFullPath($Path)
    if ($full.StartsWith($base, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $full.Substring($base.Length).TrimStart('\', '/')
    }
    return $full
}

function Get-ScanFiles {
    $roots = @("README.md", "docs", "src", "tests", "packages", "scripts")
    $files = New-Object System.Collections.Generic.List[object]
    foreach ($root in $roots) {
        $path = Join-Path $repoRoot $root
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $files.Add((Get-Item -LiteralPath $path)) | Out-Null
            continue
        }
        if (Test-Path -LiteralPath $path -PathType Container) {
            Get-ChildItem -LiteralPath $path -Recurse -File | ForEach-Object {
                $files.Add($_) | Out-Null
            }
        }
    }
    $files | Where-Object {
        $relative = Get-RepoRelativePath -Path $_.FullName
        $relative -ne "scripts\safer-check.ps1" -and
        $relative -notmatch '(^|\\)target(\\|$)' -and
        $relative -notmatch 'docs\\benchmarks\\' -and
        $_.Name -notmatch '\.(exe|dll|pdb|pdf|png|jpg|jpeg|gif|svg|tgz|whl|gz|zip|lock)$'
    }
}

function Test-UnsafeClaims {
    $patterns = @(
        @{ name = "public_saas_proof"; regex = '(?i)battle-tested public multi-tenant SaaS|already proven as a public multi-tenant SaaS|public multi-tenant SaaS platform' },
        @{ name = "scientific_proof"; regex = '(?i)scientifically proven|external scientific proof' },
        @{ name = "billing_bypass"; regex = '(?i)billing bypass|bypass provider billing' },
        @{ name = "universal_compression"; regex = '(?i)universal compression' },
        @{ name = "provider_invoice_savings"; regex = '(?i)provider invoice savings' },
        @{ name = "conversion_or_revenue"; regex = '(?i)conversion lift|revenue impact' },
        @{ name = "guarantee"; regex = '(?i)guaranteed cash savings|guaranteed savings' },
        @{ name = "cost_collapse"; regex = '(?i)cost-collapse' },
        @{ name = "slang"; regex = '(?i)\bfuck\b|riding shotgun' }
    )
    $allowContext = '(?i)\b(do not|not allowed|unsafe wording|not true|not|does not|cannot|without|require|requires|boundary|must not)\b'
    foreach ($file in Get-ScanFiles) {
        $relative = Get-RepoRelativePath -Path $file.FullName
        $lines = Get-Content -LiteralPath $file.FullName
        for ($i = 0; $i -lt $lines.Count; $i++) {
            foreach ($pattern in $patterns) {
                if ($lines[$i] -notmatch $pattern.regex) {
                    continue
                }
                $start = [Math]::Max(0, $i - 4)
                $context = ($lines[$start..$i] -join " ")
                if ($context -match $allowContext) {
                    continue
                }
                $claimHits.Add([pscustomobject]@{
                    file = $relative
                    line = $i + 1
                    rule = $pattern.name
                    text = $lines[$i].Trim()
                }) | Out-Null
            }
        }
    }
    if ($claimHits.Count -gt 0) {
        throw "unsafe wording hits: $($claimHits.Count)"
    }
    return "no unsafe wording hits"
}

function Test-SecretPatterns {
    $secretRegex = '(?i)(api[_-]?key|secret|password|token)\s*[:=]\s*["'']?[A-Za-z0-9_./+=-]{24,}'
    foreach ($file in Get-ScanFiles) {
        $relative = Get-RepoRelativePath -Path $file.FullName
        $lines = Get-Content -LiteralPath $file.FullName
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match $secretRegex) {
                $secretHits.Add([pscustomobject]@{
                    file = $relative
                    line = $i + 1
                    text = $lines[$i].Trim()
                }) | Out-Null
            }
        }
    }
    if ($secretHits.Count -gt 0) {
        throw "possible secret hits: $($secretHits.Count)"
    }
    return "no secret pattern hits"
}

Set-Location -LiteralPath $repoRoot
$script:exePath = $null
$script:qorxHome = Join-Path ([System.IO.Path]::GetTempPath()) ("qorx-safer-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $script:qorxHome | Out-Null

try {
    Run-Step "resolve_exe" {
        $script:exePath = Resolve-QorxExe -Requested $Exe
        $script:exePath
    }

    if (-not $SkipCargo) {
        Run-Step "cargo_fmt" { Invoke-Native -File "cargo" -ArgumentList @("fmt", "--check") }
        Run-Step "cargo_build_debug" { Invoke-Native -File "cargo" -ArgumentList @("build", "--locked") }
        Run-Step "cargo_test" { Invoke-Native -File "cargo" -ArgumentList @("test", "--locked") }
        Run-Step "cargo_clippy" { Invoke-Native -File "cargo" -ArgumentList @("clippy", "--all-targets", "--", "-D", "warnings") }
        Run-Step "cargo_package" { Invoke-Native -File "cargo" -ArgumentList @("package", "--locked", "--allow-dirty") }
    } else {
        Add-Result -Name "cargo_checks" -Status "skip" -Details "SkipCargo was set"
    }

    Run-Step "temp_index" {
        $oldHome = $env:QORX_HOME
        try {
            $env:QORX_HOME = $script:qorxHome
            Invoke-Native -File $script:exePath -ArgumentList @("index", ".") -WorkDir $repoRoot
        } finally {
            $env:QORX_HOME = $oldHome
        }
    }

    Run-Step "lexicon_boundary" {
        $lexicon = Invoke-QorxJson -Name "lexicon" -ArgumentList @("lexicon")
        $json = $lexicon | ConvertTo-Json -Depth 8
        if ($json -notmatch "not a physics engine" -or $json -notmatch "not physics claims") {
            throw "lexicon does not clearly bound physics vocabulary"
        }
        "physics vocabulary is bounded"
    }

    Run-Step "science_boundary" {
        $science = Invoke-QorxJson -Name "science" -ArgumentList @("science")
        if ($science.claim_boundary -match "cost-collapse") {
            throw "science boundary still uses cost-collapse"
        }
        $builtIn = @($science.built_in_logic).Count
        $adapters = @($science.external_runtime_adapters).Count
        "built_in=$builtIn adapters=$adapters"
    }

    Run-Step "adapters_status" {
        $adapters = Invoke-QorxJson -Name "adapters" -ArgumentList @("adapters")
        $ready = @($adapters.adapters | Where-Object { $_.ready -eq $true }).Count
        $total = @($adapters.adapters).Count
        "ready=$ready total=$total"
    }

    Run-Step "money_claim_guard" {
        $money = Invoke-QorxJson -Name "money" -ArgumentList @("money", "--claim-usd", "1000000000")
        if ($money.claim_check.allowed -eq $true) {
            throw "billion-dollar claim was allowed"
        }
        "observed_usd=$($money.observed.estimated_total_usd_saved) allowed=$($money.claim_check.allowed)"
    }

    Run-Step "security_attest" {
        $attest = Invoke-QorxJson -Name "security_attest" -ArgumentList @("security", "attest")
        "schema=$($attest.schema)"
    }

    Run-Step "unsafe_wording_scan" { Test-UnsafeClaims }
    Run-Step "secret_scan" { Test-SecretPatterns }
    Run-Step "technical_credibility" {
        Invoke-Native -File "powershell" -ArgumentList @(
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            (Join-Path $repoRoot "scripts\check-technical-credibility.ps1"),
            "-RepoRoot",
            $repoRoot
        )
    }
    Run-Step "testsprite_enterprise_config" {
        Invoke-Native -File "powershell" -ArgumentList @(
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            (Join-Path $repoRoot "scripts\check-testsprite-enterprise.ps1"),
            "-RepoRoot",
            $repoRoot
        )
    }
} finally {
    $resolvedHome = Resolve-Path -LiteralPath $script:qorxHome -ErrorAction SilentlyContinue
    $tempRoot = [System.IO.Path]::GetTempPath()
    if ($resolvedHome -and $resolvedHome.Path.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $resolvedHome.Path -Recurse -Force
    }
}

[pscustomobject]@{
    ok = -not $script:failed
    gate = "SAFE-R"
    meaning = "Substantiated, Auditable, Falsifiable, Evidence-bound, Restricted-claims"
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    checks = $results
    unsafe_wording_hits = $claimHits
    possible_secret_hits = $secretHits
} | ConvertTo-Json -Depth 8

if ($script:failed) {
    exit 1
}
