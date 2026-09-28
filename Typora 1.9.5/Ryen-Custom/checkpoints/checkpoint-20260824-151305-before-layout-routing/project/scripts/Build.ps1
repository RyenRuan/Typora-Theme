[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$CargoHome = [Environment]::GetEnvironmentVariable('CARGO_HOME', 'User')
$RustupHome = [Environment]::GetEnvironmentVariable('RUSTUP_HOME', 'User')

if ([string]::IsNullOrWhiteSpace($CargoHome)) {
    $CargoHome = 'E:\SDKs\Rust\cargo'
}
if ([string]::IsNullOrWhiteSpace($RustupHome)) {
    $RustupHome = 'E:\SDKs\Rust\rustup'
}

$env:CARGO_HOME = $CargoHome
$env:RUSTUP_HOME = $RustupHome
$Cargo = Join-Path $CargoHome 'bin\cargo.exe'
$Pnpm = 'C:\Users\Ryen\.cache\codex-runtimes\codex-primary-runtime\dependencies\bin\fallback\pnpm.cmd'
$Node = 'C:\Users\Ryen\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe'
$PnpmStore = 'E:\SDKs\pnpm-store'
$env:PNPM_CONFIG_STORE_DIR = $PnpmStore

if (-not (Test-Path -LiteralPath $Cargo)) {
    throw "Rust Cargo not found: $Cargo"
}
if (-not (Test-Path -LiteralPath $Pnpm)) {
    throw "Bundled pnpm not found: $Pnpm"
}
if (-not (Test-Path -LiteralPath $Node)) {
    throw "Bundled Node.js not found: $Node"
}
$env:Path = "$(Split-Path -Parent $Node);$env:Path"

Push-Location $ProjectRoot
try {
    & $Cargo build --target wasm32-unknown-unknown --release
    if ($LASTEXITCODE -ne 0) { throw 'Rust/WASM build failed.' }

    if (-not (Test-Path -LiteralPath (Join-Path $ProjectRoot 'node_modules'))) {
        & $Pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw 'Web dependency installation failed.' }
    }
    & $Pnpm run typecheck
    if ($LASTEXITCODE -ne 0) { throw 'TypeScript type check failed.' }
    & $Pnpm run build:web
    if ($LASTEXITCODE -ne 0) { throw 'TypeScript build failed.' }

    $WasmSource = Join-Path $ProjectRoot 'target\wasm32-unknown-unknown\release\ryen_mermaid_renderer.wasm'
    $Dist = Join-Path $ProjectRoot 'dist'
    New-Item -ItemType Directory -Path $Dist -Force | Out-Null
    Copy-Item -LiteralPath $WasmSource -Destination (Join-Path $Dist 'renderer.wasm') -Force
    "Typora renderer project: $ProjectRoot"
    Get-ChildItem -LiteralPath $Dist -File | Select-Object Name, Length, @{Name='SHA256'; Expression={(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash}}
}
finally {
    Pop-Location
}
