param(
    [string]$RepoRoot = ""
)

$ErrorActionPreference = "Stop"

if (-not $RepoRoot) {
    $RepoRoot = Split-Path -Parent $PSScriptRoot
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path

$failures = New-Object System.Collections.Generic.List[string]

function Add-Failure {
    param([string]$Message)
    $failures.Add($Message) | Out-Null
}

function Read-RepoText {
    param([string]$RelativePath)
    $path = Join-Path $RepoRoot $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        Add-Failure "missing $RelativePath"
        return ""
    }
    return Get-Content -LiteralPath $path -Raw
}

function Require-Text {
    param(
        [string]$Name,
        [string]$Text,
        [string]$Pattern,
        [string]$Message
    )
    if ($Text -notmatch $Pattern) {
        Add-Failure "${Name}: $Message"
    }
}

function Reject-Text {
    param(
        [string]$Name,
        [string]$Text,
        [string]$Pattern,
        [string]$Message
    )
    if ($Text -match $Pattern) {
        Add-Failure "${Name}: $Message"
    }
}

$readme = Read-RepoText "README.md"
$index = Read-RepoText "docs\index.md"
$credibility = Read-RepoText "docs\TECHNICAL_CREDIBILITY.md"
$claims = Read-RepoText "docs\handbook\claims.md"
$qorxDoc = Read-RepoText "docs\QORX.md"
$rustBrief = Read-RepoText "docs\QORX_1_0_4_RUST.md"
$testspriteDoc = Read-RepoText "docs\TESTSPRITE.md"
$testspriteResults = Read-RepoText "testsprite_tests\tmp\test_results.json"
$testspriteWorkflow = Read-RepoText ".github\workflows\testsprite-enterprise.yml"
$community = Read-RepoText "docs\COMMUNITY.md"
$commands = Read-RepoText "docs\COMMANDS.md"

Require-Text "README" $readme 'small domain-specific language' "must use bounded DSL wording"
Require-Text "README" $readme 'Technical credibility' "must link the technical credibility page"
Require-Text "README" $readme 'Qorx Community Edition' "must identify public CE"
Require-Text "README" $readme 'Qorx Local Pro' "must separate the paid local product"
Require-Text "docs index" $index 'small domain-specific language' "must use bounded DSL wording"
Require-Text "docs index" $index 'TECHNICAL_CREDIBILITY\.md' "must link the technical credibility page"
Require-Text "QORX doc" $qorxDoc 'small domain-specific language' "must bound the language claim"
Require-Text "Rust brief" $rustBrief 'small domain-specific language' "must bound the language claim"

Require-Text "credibility" $credibility 'not a general-purpose language' "must state Qorx is not general-purpose"
Require-Text "credibility" $credibility 'not Forth-compatible' "must bound qstk/Forth wording"
Require-Text "credibility" $credibility 'protobuf envelope' "must describe the bytecode envelope"
Require-Text "credibility" $credibility 'qstk' "must name the stack tape"
Require-Text "credibility" $credibility 'Do not claim provider invoice savings without' "must bound provider savings"
Require-Text "claims" $claims 'Do not claim Qorx is a general-purpose language' "must prohibit general-purpose language claims"
Require-Text "claims" $claims 'Do not claim Qorx is Forth-compatible' "must prohibit Forth compatibility claims"

Require-Text "community" $community 'Qorx Community Edition' "must define CE"
Require-Text "community" $community 'Qorx Local Pro' "must name Local Pro"
Require-Text "commands" $commands 'Pro-only commands' "must document Pro-only command refusal"
Require-Text "commands" $commands '(?m)^\s*daemon\s*$' "must list daemon as Pro-only"

Reject-Text "README" $readme '(?i)real programming language|full[- ]blown|head[- ]to[- ]head|billions of tokens|mankind' "contains hype wording"
Reject-Text "docs index" $index '(?i)real programming language|full[- ]blown|head[- ]to[- ]head|billions of tokens|mankind' "contains hype wording"
Reject-Text "QORX doc" $qorxDoc '(?i)real programming language|full[- ]blown|head[- ]to[- ]head|billions of tokens|mankind' "contains hype wording"
Reject-Text "Rust brief" $rustBrief '(?i)real programming language|full[- ]blown|head[- ]to[- ]head|billions of tokens|mankind' "contains hype wording"

Require-Text "TestSprite docs" $testspriteDoc 'public staging URL|reachable.*URL' "must state cloud QA needs a reachable target"
Require-Text "TestSprite workflow" $testspriteWorkflow 'Community boundary check' "must include the CE boundary job"
Require-Text "TestSprite workflow" $testspriteWorkflow 'qorx daemon status' "must verify daemon is refused in CE"
Require-Text "TestSprite workflow" $testspriteWorkflow 'TestSprite/run-action@v1' "must call TestSprite"

Reject-Text "TestSprite results" $testspriteResults 'Qorx 1\.0\.3|version 1\.0\.3' "must not carry stale current-release text"
Require-Text "TestSprite results" $testspriteResults 'TC001 Community homepage shows Qorx 1\.0\.4' "must include the current CE homepage test"
Require-Text "TestSprite results" $testspriteResults 'TC003 Technical credibility page bounds language and bytecode claims' "must include the credibility page suite entry"

$tc001 = Join-Path $RepoRoot "testsprite_tests\TC001_Community_homepage_shows_Qorx_1.0.4.py"
if (-not (Test-Path -LiteralPath $tc001 -PathType Leaf)) {
    Add-Failure "missing current-version CE TestSprite TC001 file"
}
$tc003 = Join-Path $RepoRoot "testsprite_tests\TC003_Technical_credibility_page_bounds_language_and_bytecode_claims.py"
if (-not (Test-Path -LiteralPath $tc003 -PathType Leaf)) {
    Add-Failure "missing TestSprite TC003 credibility test file"
}

$forbiddenPaths = @(
    ".github\workflows\release-assets.yml",
    ".github\workflows\publish-registries.yml",
    "packages",
    "packaging",
    "snap",
    "Dockerfile",
    "docker-compose.yml",
    "flake.nix"
)
foreach ($relative in $forbiddenPaths) {
    if (Test-Path -LiteralPath (Join-Path $RepoRoot $relative)) {
        Add-Failure "public distribution surface still exists: $relative"
    }
}

if ($failures.Count -gt 0) {
    [pscustomobject]@{
        ok = $false
        gate = "technical-credibility"
        failures = $failures
    } | ConvertTo-Json -Depth 4
    exit 1
}

[pscustomobject]@{
    ok = $true
    gate = "technical-credibility"
    docs = @(
        "README.md",
        "docs/TECHNICAL_CREDIBILITY.md",
        "docs/INDEPENDENT_REVIEW.md",
        "docs/QORX_1_0_4_RUST.md",
        "docs/COMMUNITY.md"
    )
    testsprite = "testsprite_tests/TC003_Technical_credibility_page_bounds_language_and_bytecode_claims.py"
} | ConvertTo-Json -Depth 4
