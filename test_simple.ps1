# Simple Test Script
$ErrorActionPreference = "Continue"

$dataDir = "C:\temp\lexum-data"
if (-not (Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
}

Write-Host "Starting server..."
$job = Start-Job -ScriptBlock {
    Set-Location 'F:\Node\hivellm\lexum'
    $env:LEXUM_DATA_DIR = 'C:\temp\lexum-data'
    & cargo run --bin lexum-server
}

Start-Sleep -Seconds 30

Write-Host "Testing endpoints..."

# Health Check
try {
    $r = Invoke-RestMethod -Uri 'http://localhost:17000/health' -Method Get -TimeoutSec 5
    Write-Host "Health: OK" -ForegroundColor Green
} catch {
    Write-Host "Health: Failed" -ForegroundColor Red
}

# Create Index
$body = @{
    name = 'test-index'
    fields = @(@{ name = 'title'; type = 'text' })
    settings = @{}
} | ConvertTo-Json -Depth 10

try {
    $r = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices' -Method Post -Body $body -ContentType 'application/json' -TimeoutSec 10
    Write-Host "Create Index: OK" -ForegroundColor Green
} catch {
    Write-Host "Create Index: Failed - $($_.Exception.Message)" -ForegroundColor Red
}

# Refresh Index
try {
    Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices/test-index/refresh' -Method Post -TimeoutSec 10 | Out-Null
    Write-Host "Refresh Index: OK" -ForegroundColor Green
} catch {
    Write-Host "Refresh Index: Failed - $($_.Exception.Message)" -ForegroundColor Red
}

# Flush Index
try {
    Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices/test-index/flush' -Method Post -TimeoutSec 10 | Out-Null
    Write-Host "Flush Index: OK" -ForegroundColor Green
} catch {
    Write-Host "Flush Index: Failed - $($_.Exception.Message)" -ForegroundColor Red
}

# Get Stats
try {
    $r = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices/test-index/stats' -Method Get -TimeoutSec 10
    Write-Host "Get Stats: OK - Docs: $($r.num_docs)" -ForegroundColor Green
} catch {
    Write-Host "Get Stats: Failed - $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "Stopping server..."
Get-Job | Stop-Job
Get-Job | Remove-Job

