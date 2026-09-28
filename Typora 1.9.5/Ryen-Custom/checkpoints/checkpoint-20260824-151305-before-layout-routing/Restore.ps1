[CmdletBinding()]
param(
    [string]$TyporaRoot = 'E:\Editor\Typora\Typora 1.9.5',
    [string]$RendererProject = 'E:\Editor\Typora\Typora 1.9.5\Ryen-Custom\mermaid-renderer'
)

$ErrorActionPreference = 'Stop'
$snapshotProject = Join-Path $PSScriptRoot 'project'
$snapshotDeployed = Join-Path $PSScriptRoot 'deployed'

$running = Get-Process -Name 'Typora' -ErrorAction SilentlyContinue |
    Where-Object {
        $_.Path -and
        ([IO.Path]::GetFullPath($_.Path) -ieq [IO.Path]::GetFullPath((Join-Path $TyporaRoot 'Typora.exe')))
    }
if ($running) {
    throw '请先完全退出 Typora，再执行检查点恢复。'
}

foreach ($directory in @('dist', 'scripts', 'src', 'tests', 'web')) {
    $source = Join-Path $snapshotProject $directory
    $destination = Join-Path $RendererProject $directory
    Copy-Item -LiteralPath $source -Destination $RendererProject -Recurse -Force
}

foreach ($file in @(
    'Cargo.lock',
    'Cargo.toml',
    'install-manifest.json',
    'package.json',
    'pnpm-lock.yaml',
    'pnpm-workspace.yaml',
    'README.md',
    'tsconfig.json'
)) {
    Copy-Item -LiteralPath (Join-Path $snapshotProject $file) -Destination (Join-Path $RendererProject $file) -Force
}

$runtimeTarget = Join-Path $TyporaRoot 'resources\ryen-mermaid-renderer'
New-Item -ItemType Directory -Path $runtimeTarget -Force | Out-Null
foreach ($file in @('main.js', 'worker.js', 'renderer.wasm')) {
    Copy-Item -LiteralPath (Join-Path $snapshotDeployed "ryen-mermaid-renderer\$file") -Destination (Join-Path $runtimeTarget $file) -Force
}
Copy-Item -LiteralPath (Join-Path $snapshotDeployed 'window.html') -Destination (Join-Path $TyporaRoot 'resources\window.html') -Force

Write-Host "已恢复检查点：$PSScriptRoot"
