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

$workflow = Read-RepoText ".github\workflows\testsprite-enterprise.yml"
Require-Text "workflow" $workflow '(?m)^\s*workflow_dispatch\s*:' "must be manually runnable"
Require-Text "workflow" $workflow 'TestSprite/run-action@v1' "must use the official TestSprite action"
Require-Text "workflow" $workflow 'testsprite-api-key:\s*\$\{\{\s*secrets\.TESTSPRITE_API_KEY\s*\}\}' "must read the API key from the GitHub secret"
Require-Text "workflow" $workflow 'base_url:\s*\$\{\{\s*inputs\.base_url\s*\}\}' "must use the operator-supplied public base URL"
Require-Text "workflow" $workflow 'github-token:\s*\$\{\{\s*github\.token\s*\}\}' "must pass the GitHub token expected by the TestSprite action"
Require-Text "workflow" $workflow 'blocking:\s*\$\{\{\s*inputs\.blocking\s*\}\}' "must make blocking mode explicit"
Require-Text "workflow" $workflow 'continue-on-error:\s*\$\{\{\s*inputs\.blocking\s*==\s*''false''\s*\}\}' "must let non-blocking cloud runs report without failing the workflow"
Require-Text "workflow" $workflow 'Community boundary check' "must include the CE boundary check job"
Require-Text "workflow" $workflow 'qorx daemon status' "must verify daemon is refused in CE"
Require-Text "workflow" $workflow 'mkdir -p testsprite_tests/tmp' "must create the TestSprite output directory before the action runs"
Require-Text "workflow" $workflow 'TESTSPRITE_BASE_URL:\s*\$\{\{\s*inputs\.base_url\s*\}\}' "must expose the public base URL to repo-managed TestSprite tests"

$docs = Read-RepoText "docs\TESTSPRITE.md"
Require-Text "docs" $docs 'TESTSPRITE_API_KEY' "must document the GitHub secret name"
Require-Text "docs" $docs '(?i)revoke|rotate' "must tell operators to revoke or rotate leaked keys"
Require-Text "docs" $docs '(?i)public staging URL|public.*URL' "must explain that TestSprite needs a reachable target"
Require-Text "docs" $docs 'TestSprite Enterprise QA' "must name the workflow"
Require-Text "docs" $docs 'Community Edition' "must identify the public CE boundary"

$community = Read-RepoText "docs\COMMUNITY.md"
Require-Text "community" $community 'Qorx Community Edition' "must define CE"
Require-Text "community" $community 'Qorx Local Pro' "must name the commercial local product"
Require-Text "community" $community '(?m)^\s*daemon\s*$' "must list daemon as refused in CE"
Require-Text "community" $community '(?m)^\s*integrate\s*$' "must list integrations as refused in CE"

$commands = Read-RepoText "docs\COMMANDS.md"
Require-Text "commands" $commands 'Pro-only commands' "must document Pro-only commands"
Require-Text "commands" $commands '(?m)^\s*daemon\s*$' "must document daemon as Pro-only"

$readme = Read-RepoText "README.md"
Require-Text "README" $readme 'Qorx Community Edition' "must present public repo as CE"
Require-Text "README" $readme 'Qorx Local Pro' "must separate the paid local product"
Require-Text "README" $readme 'public CE binary refuses' "must document command refusal"

$forbiddenPaths = @(
    "dist",
    "packages",
    "packaging",
    "snap",
    "Dockerfile",
    "docker-compose.yml",
    "flake.nix",
    ".github\workflows\release-assets.yml",
    ".github\workflows\publish-registries.yml"
)
foreach ($relative in $forbiddenPaths) {
    if (Test-Path -LiteralPath (Join-Path $RepoRoot $relative)) {
        Add-Failure "public distribution surface still exists: $relative"
    }
}

$suiteJsonPath = Join-Path $RepoRoot "testsprite_tests\tmp\test_results.json"
if (-not (Test-Path -LiteralPath $suiteJsonPath -PathType Leaf)) {
    Add-Failure "missing testsprite_tests/tmp/test_results.json"
} else {
    try {
        $suite = Get-Content -LiteralPath $suiteJsonPath -Raw | ConvertFrom-Json
        if (-not $suite -or $suite.Count -lt 1) {
            Add-Failure "testsprite_tests/tmp/test_results.json must contain at least one suite entry"
        }
        foreach ($case in @($suite)) {
            if (-not $case.title) {
                Add-Failure "TestSprite suite entry is missing title"
                continue
            }
            if (($case.PSObject.Properties.Name -contains "testStatus") -and ($case.testStatus -notin @("PASSED", "FAILED"))) {
                Add-Failure "TestSprite suite entry '$($case.title)' has invalid testStatus '$($case.testStatus)'"
            }
            if ($case.testType -notin @("FRONTEND", "BACKEND")) {
                Add-Failure "TestSprite suite entry '$($case.title)' has invalid testType '$($case.testType)'"
            }
            $fileName = ($case.title -replace '\s+', '_') -replace '-', '_'
            $fileName = $fileName -replace '[^a-zA-Z0-9._]', ''
            if (-not $fileName) {
                $fileName = "file"
            }
            $caseFile = Join-Path $RepoRoot ("testsprite_tests\{0}.py" -f $fileName)
            if (-not (Test-Path -LiteralPath $caseFile -PathType Leaf)) {
                Add-Failure "missing TestSprite python file for suite title '$($case.title)'"
            } else {
                $caseText = Get-Content -LiteralPath $caseFile -Raw
                if ($caseText -match 'http://localhost:5173') {
                    Add-Failure "TestSprite python file '$($caseFile | Split-Path -Leaf)' hardcodes localhost instead of using TESTSPRITE_BASE_URL"
                }
                if ($caseText -notmatch 'TESTSPRITE_BASE_URL') {
                    Add-Failure "TestSprite python file '$($caseFile | Split-Path -Leaf)' does not read TESTSPRITE_BASE_URL"
                }
            }
        }
    } catch {
        Add-Failure "testsprite_tests/tmp/test_results.json is not valid JSON"
    }
}

$security = Read-RepoText "SECURITY.md"
Require-Text "SECURITY" $security 'TESTSPRITE_API_KEY' "must include the TestSprite key in the secret-handling policy"
Require-Text "SECURITY" $security '(?i)TestSprite.*secret|secret.*TestSprite' "must mention TestSprite secret handling"

$textExtensions = @(
    ".md", ".ps1", ".yml", ".yaml", ".toml", ".json", ".cff", ".rs", ".js", ".py",
    ".rb", ".spec", ".lock", ".txt", ".sh", ".nix"
)
$literalSecretPattern = '(?i)\bsk-(user|test|live)-[A-Za-z0-9_-]{24,}'
Get-ChildItem -LiteralPath $RepoRoot -Recurse -File | Where-Object {
    $full = $_.FullName
    $relative = $full.Substring($RepoRoot.Length).TrimStart('\', '/')
    $relative -notmatch '(^|[\\/])(\.git|target|node_modules|\.venv)([\\/]|$)' -and
    $textExtensions -contains $_.Extension
} | ForEach-Object {
    $relative = $_.FullName.Substring($RepoRoot.Length).TrimStart('\', '/')
    $content = Get-Content -LiteralPath $_.FullName -Raw
    if ($content -match $literalSecretPattern) {
        Add-Failure "literal TestSprite-style secret found in $relative"
    }
}

if ($failures.Count -gt 0) {
    [pscustomobject]@{
        ok = $false
        gate = "testsprite-enterprise"
        failures = $failures
    } | ConvertTo-Json -Depth 4
    exit 1
}

[pscustomobject]@{
    ok = $true
    gate = "testsprite-enterprise"
    workflow = ".github/workflows/testsprite-enterprise.yml"
    docs = "docs/TESTSPRITE.md"
    boundary = "docs/COMMUNITY.md"
} | ConvertTo-Json -Depth 4
