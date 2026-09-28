#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$TyporaRoot = 'E:\Editor\Typora\Typora 1.9.5'
)

$ErrorActionPreference = 'Stop'
$rendererProject = Split-Path -Parent $PSScriptRoot
$resources = Join-Path $TyporaRoot 'resources'
$windowHtml = Join-Path $resources 'window.html'
$windowBackup = $windowHtml + '.bak-before-ryen-mermaid'
$runtimeTarget = Join-Path $resources 'ryen-mermaid-renderer'
$manifestPath = Join-Path $rendererProject 'install-manifest.json'
$marker = '<!-- RYEN-MERMAID-RENDERER:0.1.0 -->'

$running = Get-Process -Name 'Typora' -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and ([IO.Path]::GetFullPath($_.Path) -ieq ([IO.Path]::GetFullPath((Join-Path $TyporaRoot 'Typora.exe')))) }
if ($running) {
    throw '请先完全退出 Typora，再卸载本机 Mermaid 渲染器；卸载不会强制结束可能未保存的文档。'
}

if ((Test-Path -LiteralPath $windowHtml) -and (Test-Path -LiteralPath $windowBackup)) {
    $current = [IO.File]::ReadAllText($windowHtml, [Text.UTF8Encoding]::new($false))
    if ($current.Contains($marker)) {
        Copy-Item -LiteralPath $windowBackup -Destination $windowHtml -Force
        Write-Host '已恢复 Typora window.html 备份。'
    } else {
        Write-Host 'window.html 未发现 Ryen 注入标记，保留现状。' -ForegroundColor Yellow
    }
}

if (Test-Path -LiteralPath $runtimeTarget) {
    $resolvedRuntime = [IO.Path]::GetFullPath($runtimeTarget)
    if (-not $resolvedRuntime.StartsWith(([IO.Path]::GetFullPath($resources) + [IO.Path]::DirectorySeparatorChar), [StringComparison]::OrdinalIgnoreCase)) {
        throw "Unsafe renderer removal target: $resolvedRuntime"
    }
    Remove-Item -LiteralPath $runtimeTarget -Recurse -Force
}
if (Test-Path -LiteralPath $manifestPath) {
    Remove-Item -LiteralPath $manifestPath -Force
}

Write-Host ''
Write-Host 'Ryen Mermaid 本机渲染器已卸载；Markdown 文件和主题 CSS 未修改。' -ForegroundColor Green
Write-Host "window.html 备份仍保留：$windowBackup"

