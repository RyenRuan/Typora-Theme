#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$TyporaRoot = 'E:\Editor\Typora\Typora 1.9.5'
)

$ErrorActionPreference = 'Stop'
$rendererProject = Split-Path -Parent $PSScriptRoot
$packageRoot = Split-Path -Parent $rendererProject
$dist = Join-Path $rendererProject 'dist'
$resources = Join-Path $TyporaRoot 'resources'
$windowHtml = Join-Path $resources 'window.html'
$windowBackup = $windowHtml + '.bak-before-ryen-mermaid'
$runtimeTarget = Join-Path $resources 'ryen-mermaid-renderer'
$manifestPath = Join-Path $rendererProject 'install-manifest.json'
$marker = '<!-- RYEN-MERMAID-RENDERER:0.1.0 -->'
$scriptTag = $marker + '<script src="./ryen-mermaid-renderer/main.js" defer="defer"></script>'

if (-not (Test-Path -LiteralPath $TyporaRoot)) {
    throw "Typora directory does not exist: $TyporaRoot"
}
foreach ($required in @($windowHtml, (Join-Path $resources 'package.json'), (Join-Path $dist 'main.js'), (Join-Path $dist 'worker.js'), (Join-Path $dist 'renderer.wasm'))) {
    if (-not (Test-Path -LiteralPath $required)) {
        throw "Mermaid renderer package is incomplete: $required"
    }
}

$running = Get-Process -Name 'Typora' -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and ([IO.Path]::GetFullPath($_.Path) -ieq ([IO.Path]::GetFullPath((Join-Path $TyporaRoot 'Typora.exe')))) }
if ($running) {
    throw '请先完全退出 Typora，再安装本机 Mermaid 渲染器；安装不会强制结束可能未保存的文档。'
}

$package = Get-Content -LiteralPath (Join-Path $resources 'package.json') -Raw | ConvertFrom-Json
if ([string]$package.version -ne '1.9.5' -or [string]$package.releaseId -ne 'b20995f3') {
    throw "Typora version mismatch: expected 1.9.5 / b20995f3, found $($package.version) / $($package.releaseId)"
}

New-Item -ItemType Directory -Path $runtimeTarget -Force | Out-Null
if (-not (Test-Path -LiteralPath $windowBackup)) {
    Copy-Item -LiteralPath $windowHtml -Destination $windowBackup
}

$windowContent = [IO.File]::ReadAllText($windowHtml, [Text.UTF8Encoding]::new($false))
if ($windowContent.Contains($marker)) {
    $alreadyInstalled = $true
} else {
    # Wait until Typora's own frame.js has initialized the editor and loaded the
    # document.  Injecting in <head> runs during the bootstrap race and can leave
    # the editor on its empty content.html skeleton.  Loading immediately before
    # </body> keeps the renderer isolated from the file-loading pipeline while
    # still starting before the user can interact with Mermaid blocks.
    $bodyClose = $windowContent.LastIndexOf('</body>', [StringComparison]::OrdinalIgnoreCase)
    if ($bodyClose -lt 0) {
        throw 'Typora window.html has no </body>; refusing an imprecise patch.'
    }
    $windowContent = $windowContent.Insert($bodyClose, $scriptTag)
    $temporaryWindow = $windowHtml + '.ryen-mermaid.tmp'
    [IO.File]::WriteAllText($temporaryWindow, $windowContent, [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporaryWindow -Destination $windowHtml -Force
    $alreadyInstalled = $false
}

foreach ($file in @('main.js', 'worker.js', 'renderer.wasm')) {
    Copy-Item -LiteralPath (Join-Path $dist $file) -Destination (Join-Path $runtimeTarget $file) -Force
}

$runtimeHashes = [ordered]@{}
foreach ($file in @('main.js', 'worker.js', 'renderer.wasm')) {
    $runtimeHashes[$file] = (Get-FileHash -LiteralPath (Join-Path $runtimeTarget $file) -Algorithm SHA256).Hash
}
$manifest = [ordered]@{
    rendererVersion = '0.1.0'
    typoraVersion = [string]$package.version
    typoraReleaseId = [string]$package.releaseId
    installedAt = (Get-Date).ToString('o')
    windowPatched = $true
    marker = $marker
    windowSha256 = (Get-FileHash -LiteralPath $windowHtml -Algorithm SHA256).Hash
    windowBackup = $windowBackup
    runtime = $runtimeHashes
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

Write-Host ''
Write-Host 'Ryen Mermaid 本机渲染器已安装。' -ForegroundColor Green
Write-Host "运行目录：$runtimeTarget"
Write-Host "Typora window.html：$windowHtml"
if ($alreadyInstalled) { Write-Host '本次更新了运行文件，保留原有注入标记和备份。' }
Write-Host '仅接管标准 flowchart/graph LR、TD/TB；其他 Mermaid 类型保持 Typora 原生渲染。'
Write-Host '卸载时会恢复 window.html 备份，不修改 Markdown 源码。'
