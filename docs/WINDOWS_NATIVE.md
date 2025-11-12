# Running Lexum on Windows Native

This guide explains how to run Lexum directly on Windows (PowerShell) to avoid WSL filesystem compatibility issues with Tantivy.

## Why Run on Windows Native?

The Tantivy search engine library has compatibility issues with WSL's filesystem layer, especially when accessing Windows-mounted drives (e.g., `/mnt/f/`). Running Lexum directly on Windows native avoids these issues entirely.

## Prerequisites

- **Rust**: Install Rust for Windows from [rustup.rs](https://rustup.rs/)
- **PowerShell**: Windows PowerShell 5.1+ or PowerShell Core 7+
- **Windows**: Windows 10/11 (64-bit)

## Installation

### 1. Install Rust for Windows

```powershell
# Download and run rustup-init.exe from https://rustup.rs/
# Or use winget:
winget install Rustlang.Rustup
```

### 2. Verify Installation

```powershell
rustc --version
cargo --version
```

### 3. Clone and Build

```powershell
# Clone repository
git clone https://github.com/hivellm/lexum.git
cd lexum

# Build release version
cargo build --release
```

## Running Lexum Server

### Basic Usage

```powershell
# Set data directory (Windows native path)
$env:LEXUM_DATA_DIR = "C:\data\lexum"

# Create data directory if it doesn't exist
New-Item -ItemType Directory -Force -Path "C:\data\lexum"

# Run server
.\target\release\lexum-server.exe
```

### With Configuration File

```powershell
# Create config directory
New-Item -ItemType Directory -Force -Path "C:\etc\lexum"

# Copy example config
Copy-Item config.example.yml C:\etc\lexum\config.yml

# Edit config.yml to use Windows paths:
# storage:
#   path:
#     data: "C:\\data\\lexum"
#     logs: "C:\\var\\log\\lexum"

# Run with config
$env:LEXUM_CONFIG_FILE = "C:\etc\lexum\config.yml"
.\target\release\lexum-server.exe
```

### As a Windows Service (Optional)

For production use, you can run Lexum as a Windows service using NSSM (Non-Sucking Service Manager):

```powershell
# Download NSSM from https://nssm.cc/download
# Extract to C:\tools\nssm

# Install service
C:\tools\nssm\nssm.exe install LexumServer "C:\path\to\lexum-server.exe"
C:\tools\nssm\nssm.exe set LexumServer AppDirectory "C:\path\to\lexum"
C:\tools\nssm\nssm.exe set LexumServer AppEnvironmentExtra "LEXUM_DATA_DIR=C:\data\lexum"
C:\tools\nssm\nssm.exe set LexumServer AppStdout "C:\var\log\lexum\stdout.log"
C:\tools\nssm\nssm.exe set LexumServer AppStderr "C:\var\log\lexum\stderr.log"

# Start service
C:\tools\nssm\nssm.exe start LexumServer
```

## Running Lexum CLI

```powershell
# Run CLI commands
.\target\release\lexum-cli.exe index list
.\target\release\lexum-cli.exe search my-index "query"

# Interactive REPL
.\target\release\lexum-cli.exe repl
```

## Path Configuration

### Important: Use Windows Native Paths

Always use Windows native paths (backslashes or forward slashes work):

```yaml
# config.yml
storage:
  path:
    data: "C:/data/lexum"      # ✅ Good - Windows native path
    logs: "C:/var/log/lexum"   # ✅ Good - Windows native path

# ❌ Avoid WSL paths:
# data: "/mnt/f/Node/hivellm/lexum/data"  # ❌ Bad - WSL mounted drive
```

### Environment Variables

```powershell
# Set data directory
$env:LEXUM_DATA_DIR = "C:\data\lexum"

# Set logs directory
$env:LEXUM_LOGS_PATH = "C:\var\log\lexum"

# Set config file
$env:LEXUM_CONFIG_FILE = "C:\etc\lexum\config.yml"
```

## Troubleshooting

### Port Already in Use

```powershell
# Check if port 9200 is in use
netstat -ano | findstr :9200

# Kill process using port 9200 (replace PID)
taskkill /PID <PID> /F
```

### Permission Issues

```powershell
# Run PowerShell as Administrator if needed
# Or grant permissions to data directory:
icacls "C:\data\lexum" /grant "$env:USERNAME:(OI)(CI)F"
```

### Firewall

```powershell
# Allow Lexum through Windows Firewall
New-NetFirewallRule -DisplayName "Lexum Server" -Direction Inbound -LocalPort 9200 -Protocol TCP -Action Allow
```

## Performance Considerations

### File System

- Use **NTFS** filesystem (default on Windows)
- Avoid network drives or mapped drives for index storage
- Use SSD for better performance

### Memory

- Windows may limit memory differently than Linux
- Monitor with Task Manager or PowerShell:
  ```powershell
  Get-Process lexum-server | Select-Object ProcessName, WorkingSet, CPU
  ```

## Comparison: WSL vs Windows Native

| Aspect | WSL | Windows Native |
|--------|-----|----------------|
| **Tantivy Compatibility** | ❌ Issues with mounted drives | ✅ Full compatibility |
| **Performance** | Slower (filesystem translation) | ✅ Native performance |
| **Setup Complexity** | Medium | ✅ Simple |
| **Development** | Linux tools available | Windows tools |
| **Production** | Not recommended | ✅ Recommended |

## Migration from WSL

If you're currently running on WSL and want to migrate:

1. **Stop WSL server**:
   ```bash
   # In WSL
   pkill lexum-server
   ```

2. **Copy data** (if needed):
   ```powershell
   # In PowerShell
   Copy-Item -Recurse \\wsl$\Ubuntu-24.04\mnt\f\Node\hivellm\lexum\data C:\data\lexum
   ```

3. **Start Windows native server**:
   ```powershell
   $env:LEXUM_DATA_DIR = "C:\data\lexum"
   .\target\release\lexum-server.exe
   ```

## Next Steps

- See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for more help
- Check [DEVELOPMENT.md](DEVELOPMENT.md) for development setup
- Review [README.md](../README.md) for general information

