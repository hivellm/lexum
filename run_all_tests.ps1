# Run All Tests Including E2E
$ErrorActionPreference = "Continue"

# Set data directory
$dataDir = "C:\temp\lexum-test-data"
if (-not (Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
}

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Starting Lexum Server for Tests..." -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# Set environment variable
$env:LEXUM_DATA_DIR = $dataDir

# Start server in background
$serverJob = Start-Job -ScriptBlock {
    Set-Location 'F:\Node\hivellm\lexum'
    $env:LEXUM_DATA_DIR = 'C:\temp\lexum-test-data'
    & cargo run --bin lexum-server 2>&1 | Tee-Object -FilePath "C:\temp\lexum-server-test.log"
}

# Wait for server to start
Write-Host "Waiting for server to start..." -ForegroundColor Yellow
$maxRetries = 30
$retryCount = 0
$serverReady = $false

while ($retryCount -lt $maxRetries -and -not $serverReady) {
    try {
        $response = Invoke-RestMethod -Uri 'http://localhost:17000/health' -Method Get -TimeoutSec 2 -ErrorAction Stop
        $serverReady = $true
        Write-Host "Server is ready!" -ForegroundColor Green
    } catch {
        $retryCount++
        if ($retryCount % 5 -eq 0) {
            Write-Host "Waiting for server... ($retryCount/$maxRetries)" -ForegroundColor Yellow
        }
        Start-Sleep -Seconds 1
    }
}

if (-not $serverReady) {
    Write-Host "Server failed to start!" -ForegroundColor Red
    Get-Job | Stop-Job
    Get-Job | Remove-Job
    exit 1
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Running All Tests" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# Run unit tests
Write-Host "1. Running Unit Tests..." -ForegroundColor Yellow
cargo test --lib --no-fail-fast 2>&1 | Tee-Object -FilePath "C:\temp\lexum-unit-tests.log"
if ($LASTEXITCODE -ne 0) {
    Write-Host "   Unit tests failed!" -ForegroundColor Red
}

Write-Host ""
Write-Host "2. Running Handler Tests..." -ForegroundColor Yellow
cargo test --test handlers_test --no-fail-fast 2>&1 | Tee-Object -FilePath "C:\temp\lexum-handler-tests.log"
if ($LASTEXITCODE -ne 0) {
    Write-Host "   Handler tests failed!" -ForegroundColor Red
}

Write-Host ""
Write-Host "3. Running API Integration Tests..." -ForegroundColor Yellow
cargo test --test api_test --no-fail-fast 2>&1 | Tee-Object -FilePath "C:\temp\lexum-api-tests.log"
if ($LASTEXITCODE -ne 0) {
    Write-Host "   API tests failed!" -ForegroundColor Red
}

Write-Host ""
Write-Host "4. Running Comprehensive Tests..." -ForegroundColor Yellow
cargo test --test comprehensive_test --no-fail-fast 2>&1 | Tee-Object -FilePath "C:\temp\lexum-comprehensive-tests.log"
if ($LASTEXITCODE -ne 0) {
    Write-Host "   Comprehensive tests failed!" -ForegroundColor Red
}

Write-Host ""
Write-Host "5. Running Integration Tests..." -ForegroundColor Yellow
cargo test --test integration_test --no-fail-fast 2>&1 | Tee-Object -FilePath "C:\temp\lexum-integration-tests.log"
if ($LASTEXITCODE -ne 0) {
    Write-Host "   Integration tests failed!" -ForegroundColor Red
}

Write-Host ""
Write-Host "6. Running E2E Tests..." -ForegroundColor Yellow
cargo test --test e2e_test --no-fail-fast 2>&1 | Tee-Object -FilePath "C:\temp\lexum-e2e-tests.log"
if ($LASTEXITCODE -ne 0) {
    Write-Host "   E2E tests failed!" -ForegroundColor Red
}

Write-Host ""
Write-Host "7. Running E2E Module Tests..." -ForegroundColor Yellow
cargo test -p lexum-e2e-tests --no-fail-fast 2>&1 | Tee-Object -FilePath "C:\temp\lexum-e2e-module-tests.log"
if ($LASTEXITCODE -ne 0) {
    Write-Host "   E2E module tests failed!" -ForegroundColor Red
}

Write-Host ""
Write-Host "8. Running Alias Integration Tests..." -ForegroundColor Yellow
cargo test --test alias_integration_test --no-fail-fast 2>&1 | Tee-Object -FilePath "C:\temp\lexum-alias-tests.log"
if ($LASTEXITCODE -ne 0) {
    Write-Host "   Alias tests failed!" -ForegroundColor Red
}

Write-Host ""
Write-Host "9. Running Snapshot Workflow Tests..." -ForegroundColor Yellow
cargo test --test snapshot_restore_workflow_tests --no-fail-fast 2>&1 | Tee-Object -FilePath "C:\temp\lexum-snapshot-tests.log"
if ($LASTEXITCODE -ne 0) {
    Write-Host "   Snapshot tests failed!" -ForegroundColor Red
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Stopping Server..." -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# Stop server
Get-Job | Stop-Job
Get-Job | Remove-Job

Write-Host ""
Write-Host "All tests completed!" -ForegroundColor Green
Write-Host "Check log files in C:\temp\ for detailed results" -ForegroundColor Cyan

