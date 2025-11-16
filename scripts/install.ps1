# Lexum Installation Script for Windows
# Downloads and installs Lexum directly from GitHub repository

#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"

# Configuration
$RepoUrl = "https://github.com/hivellm/lexum.git"
$InstallDir = $env:LEXUM_INSTALL_DIR
if (-not $InstallDir) {
    $InstallDir = "C:\Program Files\Lexum"
}
$BinDir = "$InstallDir\bin"
$DataDir = $env:LEXUM_DATA_DIR
if (-not $DataDir) {
    $DataDir = "C:\ProgramData\Lexum"
}
$ConfigDir = "$DataDir\config"
$ServiceName = "Lexum"
$ServiceDisplayName = "Lexum Search Engine Server"
$ServiceDescription = "High-performance distributed full-text search engine"

Write-Host "🚀 Lexum Installation Script" -ForegroundColor Green
Write-Host "===========================" -ForegroundColor Green
Write-Host ""
Write-Host "Repository: $RepoUrl"
Write-Host "Install directory: $InstallDir"
Write-Host "Data directory: $DataDir"
Write-Host "Service name: $ServiceName"
Write-Host ""

# Check for admin privileges
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "❌ This script requires Administrator privileges" -ForegroundColor Red
    Write-Host "Please run PowerShell as Administrator" -ForegroundColor Yellow
    exit 1
}

# Check for required tools
Write-Host "🔍 Checking prerequisites..." -ForegroundColor Green

# Check for Git
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Host "📦 Installing Git..." -ForegroundColor Yellow
    winget install --id Git.Git -e --source winget --accept-package-agreements --accept-source-agreements
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
}

# Check for Rust
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "🦀 Installing Rust..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile "$env:TEMP\rustup-init.exe"
    & "$env:TEMP\rustup-init.exe" -y
    Remove-Item "$env:TEMP\rustup-init.exe"
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
    # Refresh PATH
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
}

# Verify Rust installation
try {
    $rustVersion = rustc --version
    Write-Host "✅ Rust installed: $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Rust installation failed. Please install manually from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

# Create temporary directory for build
$TempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
$BuildDir = Join-Path $TempDir "lexum"

Write-Host "📥 Cloning repository..." -ForegroundColor Green
git clone --depth 1 $RepoUrl $BuildDir

Push-Location $BuildDir

# Build release binary
Write-Host "🔨 Building Lexum (this may take a while)..." -ForegroundColor Green
$env:CARGO_TARGET_DIR = Join-Path $TempDir "target"
cargo build --release --bin lexum-server

if (-not (Test-Path "$env:CARGO_TARGET_DIR\release\lexum-server.exe")) {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    Pop-Location
    Remove-Item -Recurse -Force $TempDir
    exit 1
}

# Create directories
Write-Host "📁 Creating directories..." -ForegroundColor Green
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

# Install binary
Write-Host "📦 Installing binary..." -ForegroundColor Green
Copy-Item "$env:CARGO_TARGET_DIR\release\lexum-server.exe" "$BinDir\lexum-server.exe"

# Copy example config if it exists
if (Test-Path "config.example.yml") {
    if (-not (Test-Path "$ConfigDir\config.yml")) {
        Write-Host "📝 Creating default configuration..." -ForegroundColor Green
        Copy-Item "config.example.yml" "$ConfigDir\config.yml"
    }
}

Pop-Location

# Add to PATH
Write-Host "🔗 Adding to PATH..." -ForegroundColor Green
$currentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($currentPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$currentPath;$BinDir", "Machine")
    $env:Path += ";$BinDir"
    Write-Host "✅ Added $BinDir to PATH" -ForegroundColor Green
} else {
    Write-Host "✅ Already in PATH" -ForegroundColor Green
}

# Create Windows Service using NSSM (Non-Sucking Service Manager)
Write-Host "⚙️  Setting up Windows Service..." -ForegroundColor Green

# Download NSSM if not present
$NSSMPath = "$BinDir\nssm.exe"
if (-not (Test-Path $NSSMPath)) {
    Write-Host "📥 Downloading NSSM..." -ForegroundColor Yellow
    $NSSMUrl = "https://nssm.cc/release/nssm-2.24.zip"
    $NSSMZip = "$env:TEMP\nssm.zip"
    Invoke-WebRequest -Uri $NSSMUrl -OutFile $NSSMZip
    
    Expand-Archive -Path $NSSMZip -DestinationPath "$env:TEMP\nssm" -Force
    $NSSMArch = "win64"
    if ([Environment]::Is64BitOperatingSystem -eq $false) {
        $NSSMArch = "win32"
    }
    Copy-Item "$env:TEMP\nssm\nssm-2.24\$NSSMArch\nssm.exe" $NSSMPath
    Remove-Item -Recurse -Force "$env:TEMP\nssm"
    Remove-Item $NSSMZip
}

# Stop and remove existing service if it exists
$existingService = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existingService) {
    Write-Host "🛑 Stopping existing service..." -ForegroundColor Yellow
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    & $NSSMPath remove $ServiceName confirm
}

# Install service
Write-Host "📦 Installing service..." -ForegroundColor Green
& $NSSMPath install $ServiceName "$BinDir\lexum-server.exe"
& $NSSMPath set $ServiceName DisplayName $ServiceDisplayName
& $NSSMPath set $ServiceName Description $ServiceDescription
& $NSSMPath set $ServiceName Start SERVICE_AUTO_START
& $NSSMPath set $ServiceName AppDirectory $DataDir
& $NSSMPath set $ServiceName AppEnvironmentExtra "LEXUM_DATA_DIR=$DataDir`nLEXUM_CONFIG_FILE=$ConfigDir\config.yml"
& $NSSMPath set $ServiceName AppStdout "$DataDir\logs\service.log"
& $NSSMPath set $ServiceName AppStderr "$DataDir\logs\service-error.log"

# Create logs directory
New-Item -ItemType Directory -Force -Path "$DataDir\logs" | Out-Null

# Set service to restart on failure
& $NSSMPath set $ServiceName AppRestartDelay 5000
& $NSSMPath set $ServiceName AppExit Default Restart

# Start service
Write-Host "🚀 Starting service..." -ForegroundColor Green
Start-Service -Name $ServiceName

# Wait a moment
Start-Sleep -Seconds 3

# Check service status
$service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($service -and $service.Status -eq "Running") {
    Write-Host "✅ Service started successfully" -ForegroundColor Green
} else {
    Write-Host "⚠️  Service may not have started. Check status with: Get-Service $ServiceName" -ForegroundColor Yellow
}

# Cleanup
Remove-Item -Recurse -Force $TempDir

Write-Host ""
Write-Host "🎉 Lexum installation complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Installation details:"
Write-Host "  Binary: $BinDir\lexum-server.exe"
Write-Host "  CLI: lexum-server (available in PATH)"
Write-Host "  Data: $DataDir"
Write-Host "  Config: $ConfigDir"
Write-Host "  Service: $ServiceName"
Write-Host ""
Write-Host "Useful commands:"
Write-Host "  Check status: Get-Service $ServiceName"
Write-Host "  View logs: Get-Content $DataDir\logs\service.log -Tail 50 -Wait"
Write-Host "  Restart: Restart-Service $ServiceName"
Write-Host "  Stop: Stop-Service $ServiceName"
Write-Host ""
Write-Host "Test CLI: lexum-server --help"
Write-Host ""

