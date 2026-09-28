#Requires -Version 5.1

$ErrorActionPreference = 'Stop'

$typoraUserRoot = Join-Path $env:APPDATA 'Typora'
$targetThemes = Join-Path $typoraUserRoot 'themes'
$targetScripts = Join-Path $typoraUserRoot 'scripts'
$targetScript = Join-Path $targetScripts 'typora-corner-quotes.ps1'
$startupLink = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Startup\Typora Corner Quotes.lnk'
$themeNames = @('ryen-github', 'ryen-newsprint', 'ryen-night', 'ryen-pixyll', 'ryen-whitey')
$mermaidUninstallScript = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) 'mermaid-renderer\scripts\Uninstall-Ryen-Mermaid-Renderer.ps1'

if (Test-Path -LiteralPath $mermaidUninstallScript) {
    & $mermaidUninstallScript
}

$processes = Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" |
    Where-Object { $_.CommandLine -and $_.CommandLine -like ('*' + $targetScript + '*') }
foreach ($process in $processes) {
    Stop-Process -Id $process.ProcessId -Force -ErrorAction SilentlyContinue
}

if (Test-Path -LiteralPath $startupLink) {
    Remove-Item -LiteralPath $startupLink -Force
}

if (Test-Path -LiteralPath $targetScript) {
    Remove-Item -LiteralPath $targetScript -Force
}

foreach ($themeName in $themeNames) {
    $targetCss = Join-Path $targetThemes ($themeName + '.css')
    $backupCss = $targetCss + '.bak-before-ryen-custom'
    if (Test-Path -LiteralPath $backupCss) {
        Copy-Item -LiteralPath $backupCss -Destination $targetCss -Force
    }
    elseif (Test-Path -LiteralPath $targetCss) {
        Remove-Item -LiteralPath $targetCss -Force
    }
}

Write-Host ''
Write-Host 'Typora 引号映射和开机启动项已移除。' -ForegroundColor Green
Write-Host 'Ryen 自定义主题已移除；若安装时创建了同名主题备份，也已恢复。'
Write-Host '合并字体和备份文件已保留，避免影响其他文档或丢失恢复材料。' -ForegroundColor Yellow
Write-Host 'Typora 的智能引号设置不会被自动恢复，可按需要在设置界面重新开启。'
