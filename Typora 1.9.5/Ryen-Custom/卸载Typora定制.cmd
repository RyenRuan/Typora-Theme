@echo off
chcp 65001 >nul
title 卸载 Typora 本地定制
"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "%~dp0Uninstall-Typora-Customization.ps1"
echo.
pause

