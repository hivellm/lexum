# Manual Server Tests - Simple Version
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
    & cargo run --bin lexum-server 2>&1
}

# Wait for server to start
Write-Host "Waiting for server to start..."
Start-Sleep -Seconds 15

Write-Host ""
Write-Host "=========================================="
Write-Host "Running Manual Tests"
Write-Host "=========================================="
Write-Host ""

# Test 1: Health Check
Write-Host "1. Health Check..."
try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/health' -Method Get -TimeoutSec 5
    Write-Host "   OK" -ForegroundColor Green
    Write-Host "   Response: $($response | ConvertTo-Json -Compress)"
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
}

# Test 2: Root Endpoint
Write-Host ""
Write-Host "2. Root Endpoint (GET /)..."
try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/' -Method Get -TimeoutSec 5
    Write-Host "   OK" -ForegroundColor Green
    Write-Host "   Cluster Name: $($response.cluster_name)"
    Write-Host "   Version: $($response.version.number)"
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
}

# Test 3: Cluster Health
Write-Host ""
Write-Host "3. Cluster Health..."
try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/_cluster/health' -Method Get -TimeoutSec 5
    Write-Host "   OK" -ForegroundColor Green
    Write-Host "   Status: $($response.status)"
    Write-Host "   Nodes: $($response.number_of_nodes)"
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
}

# Test 4: List Indices
Write-Host ""
Write-Host "4. List Indices..."
try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices' -Method Get -TimeoutSec 5
    Write-Host "   OK" -ForegroundColor Green
    if ($response.indices) {
        Write-Host "   Indices: $($response.indices -join ', ')"
    } else {
        Write-Host "   No indices found"
    }
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
}

# Test 5: Create Index
Write-Host ""
Write-Host "5. Create Index..."
$indexBody = @{
    name = 'test-index'
    fields = @(
        @{ name = 'title'; type = 'text' }
        @{ name = 'content'; type = 'text' }
    )
    settings = @{}
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices' -Method Post -Body $indexBody -ContentType 'application/json' -TimeoutSec 10
    Write-Host "   OK" -ForegroundColor Green
    Write-Host "   Index Created: $($response.index)"
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $statusCode = $_.Exception.Response.StatusCode.value__
        Write-Host "   Status Code: $statusCode" -ForegroundColor Yellow
    }
}

# Test 6: Add Document
Write-Host ""
Write-Host "6. Add Document..."
$docBody = @{
    document = @{
        title = 'Test Document'
        content = 'This is a test document for manual testing'
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices/test-index/documents' -Method Post -Body $docBody -ContentType 'application/json' -TimeoutSec 10
    Write-Host "   OK" -ForegroundColor Green
    Write-Host "   Document ID: $($response.id)"
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $statusCode = $_.Exception.Response.StatusCode.value__
        Write-Host "   Status Code: $statusCode" -ForegroundColor Yellow
    }
}

# Test 7: Search
Write-Host ""
Write-Host "7. Search..."
$searchBody = @{
    query = @{
        match = @{
            field = 'title'
            query = 'test'
        }
    }
    limit = 10
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices/test-index/search' -Method Post -Body $searchBody -ContentType 'application/json' -TimeoutSec 10
    Write-Host "   OK" -ForegroundColor Green
    Write-Host "   Total Results: $($response.total)"
    Write-Host "   Hits: $($response.hits.Count)"
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $statusCode = $_.Exception.Response.StatusCode.value__
        Write-Host "   Status Code: $statusCode" -ForegroundColor Yellow
    }
}

# Test 8: Index Stats
Write-Host ""
Write-Host "8. Index Stats..."
try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/api/v1/indices/test-index/stats' -Method Get -TimeoutSec 5
    Write-Host "   OK" -ForegroundColor Green
    Write-Host "   Document Count: $($response.document_count)"
    Write-Host "   Index Size: $($response.index_size)"
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
}

# Test 9: Cluster Stats
Write-Host ""
Write-Host "9. Cluster Stats..."
try {
    $response = Invoke-RestMethod -Uri 'http://localhost:17000/_cluster/stats' -Method Get -TimeoutSec 5
    Write-Host "   OK" -ForegroundColor Green
    Write-Host "   Total Indices: $($response.indices.total)"
    Write-Host "   Total Documents: $($response.indices.docs)"
} catch {
    Write-Host "   Failed: $($_.Exception.Message)" -ForegroundColor Red
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

