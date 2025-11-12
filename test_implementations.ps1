# Test Implemented Functions
$ErrorActionPreference = "Continue"

# Set data directory
$dataDir = "C:\temp\lexum-data"
if (-not (Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
}

Write-Host "=========================================="
Write-Host "Starting Lexum Server on port 17000..."
Write-Host "=========================================="

# Start server in background
$serverJob = Start-Job -ScriptBlock {
    Set-Location 'F:\Node\hivellm\lexum'
    $env:LEXUM_DATA_DIR = 'C:\temp\lexum-data'
    & cargo run --bin lexum-server 2>&1 | Tee-Object -FilePath "C:\temp\lexum-server.log"
}

# Wait for server to start
Write-Host "Waiting for server to start..."
Start-Sleep -Seconds 20

# Check if server is responding
$maxRetries = 10
$retryCount = 0
$serverReady = $false

while ($retryCount -lt $maxRetries -and -not $serverReady) {
    try {
        $response = Invoke-RestMethod -Uri 'http://localhost:17000/health' -Method Get -TimeoutSec 2 -ErrorAction Stop
        $serverReady = $true
        Write-Host "Server is ready!" -ForegroundColor Green
    } catch {
        $retryCount++
        Write-Host "Waiting for server... ($retryCount/$maxRetries)" -ForegroundColor Yellow
        Start-Sleep -Seconds 2
    }
}

if (-not $serverReady) {
    Write-Host "Server failed to start!" -ForegroundColor Red
    Get-Job | Stop-Job
    Get-Job | Remove-Job
    exit 1
}

Write-Host ""
Write-Host "=========================================="
Write-Host "Testing Implemented Functions"
Write-Host "=========================================="
Write-Host ""

# Test 1: Create Index
Write-Host "1. Create Index..."
$indexBody = @{
    name = 'test-index-refresh'
    fields = @(
        @{ name = 'title'; type = 'text' }
        @{ name = 'content'; type = 'text' }
    )
    settings = @{}
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices' -Method Post -Body $indexBody -ContentType 'application/json' -TimeoutSec 10
    Write-Host "   OK - Index Created: $($response.name)" -ForegroundColor Green
    $indexCreated = $true
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $statusCode = $_.Exception.Response.StatusCode.value__
        Write-Host "   Status Code: $statusCode" -ForegroundColor Yellow
    }
    $indexCreated = $false
}

if ($indexCreated) {
    # Test 2: Add Document
    Write-Host ""
    Write-Host "2. Add Document..."
    $docBody = @{
        document = @{
            title = 'Test Document for Refresh'
            content = 'This is a test document'
        }
    } | ConvertTo-Json -Depth 10

    try {
        $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices/test-index-refresh/documents' -Method Post -Body $docBody -ContentType 'application/json' -TimeoutSec 10
        Write-Host "   OK - Document ID: $($response.id)" -ForegroundColor Green
    } catch {
        Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
    }

    # Test 3: Refresh Index
    Write-Host ""
    Write-Host "3. Refresh Index..."
    try {
        $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices/test-index-refresh/refresh' -Method Post -TimeoutSec 10
        Write-Host "   OK - Index refreshed successfully" -ForegroundColor Green
    } catch {
        Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
        if ($_.Exception.Response) {
            $statusCode = $_.Exception.Response.StatusCode.value__
            Write-Host "   Status Code: $statusCode" -ForegroundColor Yellow
        }
    }

    # Test 4: Flush Index
    Write-Host ""
    Write-Host "4. Flush Index..."
    try {
        $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices/test-index-refresh/flush' -Method Post -TimeoutSec 10
        Write-Host "   OK - Index flushed successfully" -ForegroundColor Green
    } catch {
        Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
        if ($_.Exception.Response) {
            $statusCode = $_.Exception.Response.StatusCode.value__
            Write-Host "   Status Code: $statusCode" -ForegroundColor Yellow
        }
    }

    # Test 5: Get Index Stats
    Write-Host ""
    Write-Host "5. Get Index Stats..."
    try {
        $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices/test-index-refresh/stats' -Method Get -TimeoutSec 10
        Write-Host "   OK - Stats retrieved" -ForegroundColor Green
        Write-Host "   Documents: $($response.num_docs)"
        Write-Host "   Segments: $($response.num_segments)"
    } catch {
        Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
        if ($_.Exception.Response) {
            $statusCode = $_.Exception.Response.StatusCode.value__
            Write-Host "   Status Code: $statusCode" -ForegroundColor Yellow
        }
    }
}

# Test 6: Update Cluster Settings
Write-Host ""
Write-Host "6. Update Cluster Settings..."
$settingsBody = @{
    settings = @{
        cluster_name = 'test-cluster'
        persistence = @{
            storage_path = 'C:\temp\lexum-data'
            snapshot = @{
                repository_path = 'C:\temp\lexum-snapshots'
                max_snapshots = 10
            }
        }
        network = @{
            bind_address = '0.0.0.0'
            port = 17000
            enable_cors = $true
        }
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/_cluster/settings' -Method Put -Body $settingsBody -ContentType 'application/json' -TimeoutSec 10
    Write-Host "   OK - Settings updated successfully" -ForegroundColor Green
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $statusCode = $_.Exception.Response.StatusCode.value__
        Write-Host "   Status Code: $statusCode" -ForegroundColor Yellow
    }
}

# Test 7: Update Cluster Settings (Invalid - Empty Cluster Name)
Write-Host ""
Write-Host "7. Update Cluster Settings (Invalid - Empty Name)..."
$invalidSettingsBody = @{
    settings = @{
        cluster_name = ''
        persistence = @{
            storage_path = 'C:\temp\lexum-data'
            snapshot = @{
                repository_path = 'C:\temp\lexum-snapshots'
                max_snapshots = 10
            }
        }
        network = @{
            bind_address = '0.0.0.0'
            port = 17000
            enable_cors = $true
        }
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/_cluster/settings' -Method Put -Body $invalidSettingsBody -ContentType 'application/json' -TimeoutSec 10
    Write-Host "   Failed - Should have returned error" -ForegroundColor Red
} catch {
    if ($_.Exception.Response.StatusCode.value__ -eq 400) {
        Write-Host "   OK - Correctly rejected invalid settings (400)" -ForegroundColor Green
    } else {
        Write-Host "   Unexpected error: $($_.Exception.Message)" -ForegroundColor Yellow
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
Write-Host ""
Write-Host "=========================================="
Write-Host "Test Complete"
Write-Host "=========================================="

