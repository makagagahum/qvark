[CmdletBinding()]
param(
    [switch]$Install,
    [string]$OutputPath = "target\release\qorx-v1.0.0-adapter-proof-matrix.json"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir "..\..")
$NodeRoot = Join-Path $RepoRoot "target\adapter-proof-node"
$BinDir = Join-Path $RepoRoot "target\adapter-proof-bin"
$ProofHome = Join-Path $RepoRoot "target\adapter-proof-qorx-home"
$PythonExe = Join-Path $RepoRoot ".venv\Scripts\python.exe"
$QorxExe = Join-Path $RepoRoot "target\release\qorx.exe"
if (-not (Test-Path $QorxExe)) {
    $PackagedExe = Join-Path $RepoRoot "qorx.exe"
    if (Test-Path $PackagedExe) {
        $QorxExe = $PackagedExe
    }
}
$Rows = New-Object System.Collections.Generic.List[object]

function Add-Row {
    param(
        [string]$Name,
        [string]$Status,
        [object]$Evidence,
        [string]$Boundary = ""
    )
    $Rows.Add([ordered]@{
        name = $Name
        status = $Status
        evidence = $Evidence
        boundary = $Boundary
    }) | Out-Null
}

function Invoke-Capture {
    param(
        [string]$File,
        [string[]]$Arguments = @(),
        [hashtable]$Environment = @{}
    )

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $File
    $psi.WorkingDirectory = $RepoRoot
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    foreach ($arg in $Arguments) {
        [void]$psi.ArgumentList.Add($arg)
    }
    foreach ($key in $Environment.Keys) {
        $psi.Environment[$key] = [string]$Environment[$key]
    }

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $process = [System.Diagnostics.Process]::Start($psi)
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    $sw.Stop()

    [pscustomobject]@{
        exit_code = $process.ExitCode
        stdout = $stdout.Trim()
        stderr = $stderr.Trim()
        elapsed_ms = $sw.ElapsedMilliseconds
    }
}

function Parse-JsonOutput {
    param([string]$Text)
    if ([string]::IsNullOrWhiteSpace($Text)) {
        return $null
    }
    return $Text | ConvertFrom-Json -Depth 32
}

function Run-Proof {
    param(
        [string]$Name,
        [string]$File,
        [string[]]$Arguments = @(),
        [hashtable]$Environment = @{}
    )

    try {
        $result = Invoke-Capture -File $File -Arguments $Arguments -Environment $Environment
        $json = Parse-JsonOutput -Text $result.stdout
        $status = if ($result.exit_code -eq 0 -and $json -and $json.status -eq "pass") { "pass" } else { "fail" }
        Add-Row -Name $Name -Status $status -Evidence ([ordered]@{
            exit_code = $result.exit_code
            elapsed_ms = $result.elapsed_ms
            json = $json
            stderr = $result.stderr
        }) -Boundary $(if ($json -and $json.boundary) { $json.boundary } else { "" })
    }
    catch {
        Add-Row -Name $Name -Status "fail" -Evidence ([ordered]@{ error = $_.Exception.Message })
    }
}

function Ensure-NodeProofWorkspace {
    if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
        Add-Row -Name "node_runtime" -Status "blocked" -Evidence "node is not on PATH"
        return $false
    }
    if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
        Add-Row -Name "npm_runtime" -Status "blocked" -Evidence "npm is not on PATH"
        return $false
    }
    New-Item -ItemType Directory -Path $NodeRoot -Force | Out-Null
    if (-not (Test-Path (Join-Path $NodeRoot "package.json"))) {
        if (-not $Install) {
            Add-Row -Name "node_adapter_dependencies" -Status "blocked" -Evidence "run scripts\adapter-proofs\prove-adapters.ps1 -Install once to create target\adapter-proof-node"
            return $false
        }
        Push-Location $NodeRoot
        try {
            npm init -y | Out-Null
        }
        finally {
            Pop-Location
        }
    }
    $requiredPackages = @(
        "tree-sitter@0.22.4",
        "tree-sitter-rust@0.24.0",
        "onnxruntime-node@1.25.1",
        "onnx-proto@4.0.4",
        "@xenova/transformers@2.17.2"
    )
    $missing = @()
    foreach ($pkg in @("tree-sitter", "tree-sitter-rust", "onnxruntime-node", "onnx-proto", "@xenova/transformers")) {
        $pkgPath = Join-Path $NodeRoot "node_modules\$pkg"
        if (-not (Test-Path $pkgPath)) {
            $missing += $pkg
        }
    }
    if ($missing.Count -gt 0) {
        if (-not $Install) {
            Add-Row -Name "node_adapter_dependencies" -Status "blocked" -Evidence ([ordered]@{ missing = $missing })
            return $false
        }
        Push-Location $NodeRoot
        try {
            npm install $requiredPackages --legacy-peer-deps | Out-Null
        }
        finally {
            Pop-Location
        }
    }
    Add-Row -Name "node_adapter_dependencies" -Status "pass" -Evidence ([ordered]@{
        node_root = $NodeRoot
        packages = $requiredPackages
    })
    return $true
}

function Ensure-LLMLingua {
    if (-not (Test-Path $PythonExe)) {
        Add-Row -Name "llmlingua_dependency" -Status "blocked" -Evidence ".venv\Scripts\python.exe is missing"
        return $false
    }
    $check = Invoke-Capture -File $PythonExe -Arguments @("-c", "import llmlingua; print(llmlingua.__version__)")
    if ($check.exit_code -ne 0) {
        if (-not $Install) {
            Add-Row -Name "llmlingua_dependency" -Status "blocked" -Evidence ([ordered]@{ stderr = $check.stderr })
            return $false
        }
        $install = Invoke-Capture -File $PythonExe -Arguments @("-m", "pip", "install", "llmlingua==0.2.2")
        if ($install.exit_code -ne 0) {
            Add-Row -Name "llmlingua_dependency" -Status "fail" -Evidence ([ordered]@{ stderr = $install.stderr })
            return $false
        }
    }
    Add-Row -Name "llmlingua_dependency" -Status "pass" -Evidence ([ordered]@{ python = $PythonExe })
    return $true
}

function Write-Wrapper {
    param(
        [string]$Name,
        [string]$Body
    )
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    $path = Join-Path $BinDir $Name
    Set-Content -Path $path -Value $Body -Encoding ascii
    return $path
}

function Write-ProofWrappers {
    $tree = Write-Wrapper -Name "tree-sitter.cmd" -Body "@echo off`r`nset QORX_ADAPTER_NODE_ROOT=$NodeRoot`r`nnode `"$ScriptDir\tree-sitter-proof.js`" %*`r`n"
    $onnx = Write-Wrapper -Name "onnxruntime.cmd" -Body "@echo off`r`nset QORX_ADAPTER_NODE_ROOT=$NodeRoot`r`nnode `"$ScriptDir\onnx-proof.js`" %*`r`n"
    $embed = Write-Wrapper -Name "qorx-embedding.cmd" -Body "@echo off`r`nset QORX_ADAPTER_NODE_ROOT=$NodeRoot`r`nnode `"$ScriptDir\embedding-proof.js`" %*`r`n"
    $llm = Write-Wrapper -Name "llmlingua.cmd" -Body "@echo off`r`nset HF_HOME=$RepoRoot\target\adapter-proof-hf`r`n`"$PythonExe`" `"$ScriptDir\llmlingua-proof.py`" %*`r`n"
    return [ordered]@{
        tree_sitter = $tree
        onnxruntime = $onnx
        embedding = $embed
        llmlingua = $llm
    }
}

function Run-KvHintProof {
    if (-not (Test-Path $QorxExe)) {
        Add-Row -Name "kv_hint_safetensors" -Status "blocked" -Evidence "target\release\qorx.exe is missing; run cargo build --release first"
        return
    }
    Remove-Item -Recurse -Force $ProofHome -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Path $ProofHome -Force | Out-Null
    $envMap = @{ QORX_HOME = $ProofHome }
    $index = Invoke-Capture -File $QorxExe -Arguments @("index", ".") -Environment $envMap
    if ($index.exit_code -ne 0) {
        Add-Row -Name "kv_hint_safetensors" -Status "fail" -Evidence ([ordered]@{ stage = "index"; stderr = $index.stderr })
        return
    }
    $kvPath = Join-Path $NodeRoot "qorx-kv-proof.safetensors"
    $emit = Invoke-Capture -File $QorxExe -Arguments @("kv", "emit", "--model", "vllm", "--task", "adapter proof kv hints", "--out", $kvPath) -Environment $envMap
    if ($emit.exit_code -ne 0) {
        Add-Row -Name "kv_hint_safetensors" -Status "fail" -Evidence ([ordered]@{ stage = "emit"; stderr = $emit.stderr })
        return
    }
    Run-Proof -Name "kv_hint_safetensors" -File "node" -Arguments @((Join-Path $ScriptDir "kv-hint-proof.js"), $kvPath) -Environment @{ QORX_ADAPTER_NODE_ROOT = $NodeRoot }
}

function Run-QorxAdapterReport {
    param([object]$Wrappers)
    if (-not (Test-Path $QorxExe)) {
        Add-Row -Name "qorx_adapters_env_detection" -Status "blocked" -Evidence "target\release\qorx.exe is missing"
        return
    }
    $envMap = @{
        QORX_TREESITTER_CMD = $Wrappers.tree_sitter
        QORX_LLMLINGUA_CMD = $Wrappers.llmlingua
        QORX_ONNX_COMPRESSOR_CMD = $Wrappers.onnxruntime
        QORX_EMBEDDING_CMD = $Wrappers.embedding
        HF_HOME = Join-Path $RepoRoot "target\adapter-proof-hf"
    }
    $result = Invoke-Capture -File $QorxExe -Arguments @("adapters") -Environment $envMap
    $json = Parse-JsonOutput -Text $result.stdout
    $readyNames = @()
    if ($json) {
        foreach ($adapter in $json.adapters) {
            if ($adapter.ready) {
                $readyNames += $adapter.name
            }
        }
    }
    $required = @("tree-sitter parser packs", "LLMLingua compressor", "ONNX compressor", "embedding/vector backend")
    $missing = @($required | Where-Object { $readyNames -notcontains $_ })
    $status = if ($result.exit_code -eq 0 -and $missing.Count -eq 0) { "pass" } else { "fail" }
    Add-Row -Name "qorx_adapters_env_detection" -Status $status -Evidence ([ordered]@{
        ready = $readyNames
        missing = $missing
        stderr = $result.stderr
    })
}

function Run-KvRuntimeProbe {
    $turboquant = Get-Command turboquant -ErrorAction SilentlyContinue
    $vllm = Get-Command vllm -ErrorAction SilentlyContinue
    $nvidia = Get-Command nvidia-smi -ErrorAction SilentlyContinue
    if ($turboquant -or $vllm) {
        Add-Row -Name "turboquant_vllm_runtime_probe" -Status "pass" -Evidence ([ordered]@{
            turboquant = if ($turboquant) { $turboquant.Source } else { $null }
            vllm = if ($vllm) { $vllm.Source } else { $null }
            nvidia_smi = if ($nvidia) { $nvidia.Source } else { $null }
        }) -Boundary "A runtime command is present. Separate latency or VRAM benchmarks are still required before claiming KV-cache compression savings."
    }
    else {
        Add-Row -Name "turboquant_vllm_runtime_probe" -Status "blocked" -Evidence ([ordered]@{
            turboquant = $null
            vllm = $null
            nvidia_smi = if ($nvidia) { $nvidia.Source } else { $null }
            reason = "No turboquant or vllm command is installed on this Windows PATH."
        }) -Boundary "Qorx-side KV export is proven separately; runtime KV-cache savings remain unproven until a local vLLM/TurboQuant adapter consumes the artifact."
    }
}

Set-Location $RepoRoot
$nodeReady = Ensure-NodeProofWorkspace
$llmReady = Ensure-LLMLingua
$wrappers = Write-ProofWrappers

if ($nodeReady) {
    Run-Proof -Name "tree_sitter_runtime_parse" -File "node" -Arguments @((Join-Path $ScriptDir "tree-sitter-proof.js")) -Environment @{ QORX_ADAPTER_NODE_ROOT = $NodeRoot }
    Run-Proof -Name "onnx_runtime_inference" -File "node" -Arguments @((Join-Path $ScriptDir "onnx-proof.js")) -Environment @{ QORX_ADAPTER_NODE_ROOT = $NodeRoot }
    Run-Proof -Name "embedding_backend_inference" -File "node" -Arguments @((Join-Path $ScriptDir "embedding-proof.js")) -Environment @{ QORX_ADAPTER_NODE_ROOT = $NodeRoot }
    Run-KvHintProof
}

if ($llmReady) {
    Run-Proof -Name "llmlingua_prompt_compression" -File $PythonExe -Arguments @((Join-Path $ScriptDir "llmlingua-proof.py")) -Environment @{
        HF_HOME = Join-Path $RepoRoot "target\adapter-proof-hf"
    }
}

Run-QorxAdapterReport -Wrappers $wrappers
Run-KvRuntimeProbe

$pass = @($Rows | Where-Object { $_.status -eq "pass" }).Count
$blocked = @($Rows | Where-Object { $_.status -eq "blocked" }).Count
$fail = @($Rows | Where-Object { $_.status -eq "fail" }).Count
$matrix = [ordered]@{
    schema = "qorx.adapter-proof-matrix.v1"
    generated_at = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    qorx_version = "1.0.0"
    repo = $RepoRoot.Path
    summary = [ordered]@{
        pass = $pass
        blocked = $blocked
        fail = $fail
    }
    rows = $Rows
}

$outputFullPath = if ([System.IO.Path]::IsPathRooted($OutputPath)) { $OutputPath } else { Join-Path $RepoRoot $OutputPath }
$outputDir = Split-Path -Parent $outputFullPath
if (-not [string]::IsNullOrWhiteSpace($outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}
$matrix | ConvertTo-Json -Depth 64 | Set-Content -Path $outputFullPath -Encoding utf8
$matrix | ConvertTo-Json -Depth 64
