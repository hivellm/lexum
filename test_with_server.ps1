# Test Create Index with server
$ErrorActionPreference = "Continue"

# Set data directory
$dataDir = "C:\temp\lexum-data"
if (-not (Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
}

Write-Host "=========================================="
Write-Host "Starting Lexum Server..."
Write-Host "=========================================="

# Start server in background
$serverJob = Start-Job -ScriptBlock {
    Set-Location 'F:\Node\hivellm\lexum'
    $env:LEXUM_DATA_DIR = 'C:\temp\lexum-data'
    & cargo run --bin lexum-server 2>&1
}

# Wait for server to start
Write-Host "Waiting for server to start..."
Start-Sleep -Seconds 15

Write-Host ""
Write-Host "=========================================="
Write-Host "Testing Create Index"
Write-Host "=========================================="
Write-Host ""

$body = @{
    name = 'test-index'
    fields = @(
        @{ name = 'title'; type = 'text' }
        @{ name = 'content'; type = 'text' }
    )
    settings = @{}
} | ConvertTo-Json -Depth 10

Write-Host "Request Body:"
Write-Host $body
Write-Host ""

try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices' -Method Post -Body $body -ContentType 'application/json' -TimeoutSec 10
    Write-Host "Success:" -ForegroundColor Green
    Write-Host ($response | ConvertTo-Json)
} catch {
    Write-Host "Error Status Code: $($_.Exception.Response.StatusCode.value__)" -ForegroundColor Red
    Write-Host "Error Message: $($_.Exception.Message)" -ForegroundColor Red
    
    if ($_.Exception.Response) {
        $stream = $_.Exception.Response.GetResponseStream()
        $reader = New-Object System.IO.StreamReader($stream)
        $responseBody = $reader.ReadToEnd()
        Write-Host "Error Response Body:" -ForegroundColor Yellow
        Write-Host $responseBody
    }
}

Write-Host ""
Write-Host "=========================================="
Write-Host "Stopping Server..."
Write-Host "=========================================="

# Stop server
Get-Job | Stop-Job
Get-Job | Remove-Job

Write-Host "Server stopped"

