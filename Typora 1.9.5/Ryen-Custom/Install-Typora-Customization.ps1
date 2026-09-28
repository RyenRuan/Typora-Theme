#Requires -Version 5.1

$ErrorActionPreference = 'Stop'

$packageRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$sourceThemes = Join-Path $packageRoot 'themes'
$sourceScript = Join-Path $packageRoot 'scripts\typora-corner-quotes.ps1'
$sourceFont = Join-Path $packageRoot 'font\consolaslxgw.ttf'
$mermaidInstallScript = Join-Path $packageRoot 'mermaid-renderer\scripts\Install-Ryen-Mermaid-Renderer.ps1'

$typoraUserRoot = Join-Path $env:APPDATA 'Typora'
$targetThemes = Join-Path $typoraUserRoot 'themes'
$targetScripts = Join-Path $typoraUserRoot 'scripts'
$targetScript = Join-Path $targetScripts 'typora-corner-quotes.ps1'
$startupLink = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Startup\Typora Corner Quotes.lnk'
$fontDirectory = Join-Path $env:LOCALAPPDATA 'Microsoft\Windows\Fonts'
$targetFont = Join-Path $fontDirectory (Split-Path $sourceFont -Leaf)
$themeMappings = [ordered]@{
    github    = 'ryen-github'
    newsprint = 'ryen-newsprint'
    night     = 'ryen-night'
    pixyll    = 'ryen-pixyll'
    whitey    = 'ryen-whitey'
}
$legacyThemeMarker = '/* Codex custom font override:'

function Stop-TyporaQuoteHook {
    $processes = Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" |
        Where-Object { $_.CommandLine -and $_.CommandLine -like ('*' + $targetScript + '*') }

    foreach ($process in $processes) {
        Stop-Process -Id $process.ProcessId -Force -ErrorAction SilentlyContinue
    }
}

foreach ($requiredFile in @($sourceScript, $sourceFont, $mermaidInstallScript, (Join-Path $packageRoot 'mermaid-renderer\dist\main.js'), (Join-Path $packageRoot 'mermaid-renderer\dist\worker.js'), (Join-Path $packageRoot 'mermaid-renderer\dist\renderer.wasm'))) {
    if (-not (Test-Path -LiteralPath $requiredFile)) {
        throw "定制包缺少文件：$requiredFile"
    }
}

# Patch and deploy the isolated Mermaid renderer before changing user themes.
# It refuses to run while Typora is open and validates the exact 1.9.5 build.
& $mermaidInstallScript

New-Item -ItemType Directory -Force -Path $targetThemes, $targetScripts, $fontDirectory | Out-Null

# Deploy uniquely named themes so Typora will not replace them as built-in CSS.
foreach ($themeName in $themeMappings.Keys) {
    $sourceCss = Join-Path $sourceThemes ($themeName + '.css')
    $targetCss = Join-Path $targetThemes ($themeMappings[$themeName] + '.css')
    $backupCss = $targetCss + '.bak-before-ryen-custom'

    if (-not (Test-Path -LiteralPath $sourceCss)) {
        throw "定制包缺少主题：$sourceCss"
    }

    if ((Test-Path -LiteralPath $targetCss) -and -not (Test-Path -LiteralPath $backupCss)) {
        Copy-Item -LiteralPath $targetCss -Destination $backupCss
    }

    Copy-Item -LiteralPath $sourceCss -Destination $targetCss -Force

    # Migrate the previous package version without overwriting Typora's
    # built-in theme. The old customization was always appended at the end.
    $legacyCss = Join-Path $targetThemes ($themeName + '.css')
    if (Test-Path -LiteralPath $legacyCss) {
        $legacyContent = [System.IO.File]::ReadAllText($legacyCss, [System.Text.Encoding]::UTF8)
        $markerIndex = $legacyContent.IndexOf($legacyThemeMarker, [System.StringComparison]::Ordinal)
        if ($markerIndex -ge 0) {
            $migrationBackup = $legacyCss + '.bak-before-ryen-theme-migration'
            if (-not (Test-Path -LiteralPath $migrationBackup)) {
                Copy-Item -LiteralPath $legacyCss -Destination $migrationBackup
            }

            $restoredBuiltInCss = $legacyContent.Substring(0, $markerIndex).TrimEnd() + [Environment]::NewLine
            [System.IO.File]::WriteAllText(
                $legacyCss,
                $restoredBuiltInCss,
                [System.Text.UTF8Encoding]::new($false))
        }
    }
}

# Copy the local font assets referenced by the theme CSS files.
foreach ($assetDirectoryName in @('github', 'newsprint', 'night', 'pixyll')) {
    $sourceAssetDirectory = Join-Path $sourceThemes $assetDirectoryName
    if (Test-Path -LiteralPath $sourceAssetDirectory) {
        Copy-Item -LiteralPath $sourceAssetDirectory -Destination $targetThemes -Recurse -Force
    }
}

# Install the merged font for the current Windows user. Windows may keep an
# installed font file open; skip the copy when the exact file is already there.
$fontNeedsCopy = -not (Test-Path -LiteralPath $targetFont)
if (-not $fontNeedsCopy) {
    $sourceFontHash = (Get-FileHash -LiteralPath $sourceFont -Algorithm SHA256).Hash
    $targetFontHash = (Get-FileHash -LiteralPath $targetFont -Algorithm SHA256).Hash
    $fontNeedsCopy = $sourceFontHash -ne $targetFontHash
}
if ($fontNeedsCopy) {
    Copy-Item -LiteralPath $sourceFont -Destination $targetFont -Force
}
$fontRegistryPath = 'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Fonts'
New-Item -Path $fontRegistryPath -Force | Out-Null
New-ItemProperty `
    -Path $fontRegistryPath `
    -Name 'consolaslxgw (TrueType)' `
    -Value $targetFont `
    -PropertyType String `
    -Force | Out-Null

if (-not ('RyenFontRefresh' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class RyenFontRefresh
{
    [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
    public static extern IntPtr SendMessageTimeout(
        IntPtr hWnd,
        uint message,
        IntPtr wParam,
        IntPtr lParam,
        uint flags,
        uint timeout,
        out IntPtr result);
}
'@
}
$fontRefreshResult = [IntPtr]::Zero
[void][RyenFontRefresh]::SendMessageTimeout(
    [IntPtr]0xFFFF,
    0x001D,
    [IntPtr]::Zero,
    [IntPtr]::Zero,
    0x0002,
    1000,
    [ref]$fontRefreshResult)

# Install and start the Typora-only quote hook.
Stop-TyporaQuoteHook
Start-Sleep -Milliseconds 500
Copy-Item -LiteralPath $sourceScript -Destination $targetScript -Force

$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($startupLink)
$shortcut.TargetPath = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
$shortcut.Arguments = '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "' + $targetScript + '"'
$shortcut.WorkingDirectory = $targetScripts
$shortcut.WindowStyle = 7
$shortcut.Description = 'Typora-only Chinese corner quote mapping'
$shortcut.Save()

$startArguments = '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "' + $targetScript + '"'
Start-Process `
    -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" `
    -ArgumentList $startArguments `
    -WindowStyle Hidden

Write-Host ''
Write-Host 'Typora 本地定制已部署。' -ForegroundColor Green
Write-Host "主题目录：$targetThemes"
Write-Host '自定义主题：Ryen Github、Ryen Newsprint、Ryen Night、Ryen Pixyll、Ryen Whitey'
Write-Host "引号脚本：$targetScript"
Write-Host "字体文件：$targetFont"
Write-Host ''
Write-Host '最后一步：请在 Typora 的“编辑 -> 智能标点”中关闭“智能引号”和“渲染时转换”。' -ForegroundColor Yellow
Write-Host '然后重启 Typora，在“主题”菜单中选择一个 Ryen 主题，再测试字体和中英文输入状态。'
