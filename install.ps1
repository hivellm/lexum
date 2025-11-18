# Lexum Installation Script for Windows - One-liner version
# Usage: powershell -c "irm https://raw.githubusercontent.com/hivellm/lexum/main/install.ps1 | iex"

$ErrorActionPreference = "Stop"

# Download and execute the full installation script
$InstallScriptUrl = "https://raw.githubusercontent.com/hivellm/lexum/main/scripts/install.ps1"

Write-Host "🚀 Downloading Lexum installation script..." -ForegroundColor Green
Invoke-WebRequest -Uri $InstallScriptUrl -UseBasicParsing | Invoke-Expression

