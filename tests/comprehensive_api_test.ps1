# Comprehensive API Test Script for Lexum (PowerShell)
# Tests all implemented endpoints and functionality

param(
    [string]$BaseUrl = "http://localhost:9200"
)

$ErrorActionPreference = "Stop"

$TestIndex = "test_index_$(Get-Date -Format 'yyyyMMddHHmmss')"
$TestRepo = "test_repo_$(Get-Date -Format 'yyyyMMddHHmmss')"
$TestSnapshot = "test_snapshot_$(Get-Date -Format 'yyyyMMddHHmmss')"
$TestTemplate = "test_template_$(Get-Date -Format 'yyyyMMddHHmmss')"
$TestAlias = "test_alias_$(Get-Date -Format 'yyyyMMddHHmmss')"

$Passed = 0
$Failed = 0

function Test-Endpoint {
    param(
        [string]$Method,
        [string]$Endpoint,
        [string]$Data = $null,
        [int]$ExpectedStatus,
        [string]$Description
    )
    
    try {
        $headers = @{
            "Content-Type" = "application/json"
        }
        
        if ($Data) {
            $response = Invoke-WebRequest -Uri "$BaseUrl$Endpoint" `
                -Method $Method `
                -Headers $headers `
                -Body $Data `
                -UseBasicParsing `
                -ErrorAction Stop
        } else {
            $response = Invoke-WebRequest -Uri "$BaseUrl$Endpoint" `
                -Method $Method `
                -UseBasicParsing `
                -ErrorAction Stop
        }
        
        if ($response.StatusCode -eq $ExpectedStatus) {
            Write-Host "✓ $Description (HTTP $($response.StatusCode))" -ForegroundColor Green
            $script:Passed++
            return $true
        } else {
            Write-Host "✗ $Description (Expected HTTP $ExpectedStatus, got $($response.StatusCode))" -ForegroundColor Red
            Write-Host "  Response: $($response.Content)" -ForegroundColor Yellow
            $script:Failed++
            return $false
        }
    } catch {
        $statusCode = $_.Exception.Response.StatusCode.value__
        if ($statusCode -eq $ExpectedStatus) {
            Write-Host "✓ $Description (HTTP $statusCode)" -ForegroundColor Green
            $script:Passed++
            return $true
        } else {
            Write-Host "✗ $Description (Expected HTTP $ExpectedStatus, got $statusCode)" -ForegroundColor Red
            Write-Host "  Error: $($_.Exception.Message)" -ForegroundColor Yellow
            $script:Failed++
            return $false
        }
    }
}

Write-Host "=========================================="
Write-Host "Lexum Comprehensive API Test Suite"
Write-Host "=========================================="
Write-Host "Base URL: $BaseUrl"
Write-Host "Test Index: $TestIndex"
Write-Host ""

# 1. Health Check
Write-Host "=== 1. Health Check ==="
Test-Endpoint -Method "GET" -Endpoint "/health" -ExpectedStatus 200 -Description "Health check endpoint"

# 2. Cluster Info
Write-Host ""
Write-Host "=== 2. Cluster Operations ==="
Test-Endpoint -Method "GET" -Endpoint "/" -ExpectedStatus 200 -Description "Cluster info (root endpoint)"
Test-Endpoint -Method "GET" -Endpoint "/_cluster/health" -ExpectedStatus 200 -Description "Cluster health"
Test-Endpoint -Method "GET" -Endpoint "/_cluster/stats" -ExpectedStatus 200 -Description "Cluster stats"
Test-Endpoint -Method "GET" -Endpoint "/_cluster/state" -ExpectedStatus 200 -Description "Cluster state"
Test-Endpoint -Method "GET" -Endpoint "/_nodes/stats" -ExpectedStatus 200 -Description "Node stats"
Test-Endpoint -Method "GET" -Endpoint "/_cluster/settings" -ExpectedStatus 200 -Description "Get cluster settings"

# 3. Index Operations
Write-Host ""
Write-Host "=== 3. Index Operations ==="
$schema = '{"fields":[{"name":"title","type":"text","indexed":true,"stored":true},{"name":"content","type":"text","indexed":true}]}'
$createIndex = "{`"name`":`"$TestIndex`",`"mappings`":{`"fields`":$schema}}"

Test-Endpoint -Method "POST" -Endpoint "/api/v1/indices" -Data $createIndex -ExpectedStatus 201 -Description "Create index"
Test-Endpoint -Method "GET" -Endpoint "/api/v1/indices" -ExpectedStatus 200 -Description "List indices"
Test-Endpoint -Method "GET" -Endpoint "/api/v1/indices/$TestIndex" -ExpectedStatus 200 -Description "Get index info"
Test-Endpoint -Method "GET" -Endpoint "/api/v1/indices/$TestIndex/stats" -ExpectedStatus 200 -Description "Get index stats"

# 4. Document Operations
Write-Host ""
Write-Host "=== 4. Document Operations ==="
$doc = '{"document":{"title":"Test Document","content":"This is a test document"}}'
Test-Endpoint -Method "POST" -Endpoint "/api/v1/indices/$TestIndex/documents" -Data $doc -ExpectedStatus 201 -Description "Add document"

# Get document ID from response
try {
    $docResponse = Invoke-RestMethod -Uri "$BaseUrl/api/v1/indices/$TestIndex/documents" `
        -Method POST `
        -Headers @{"Content-Type" = "application/json"} `
        -Body $doc
    $docId = $docResponse.id
    
    if ($docId) {
        Test-Endpoint -Method "GET" -Endpoint "/api/v1/indices/$TestIndex/documents/$docId" -ExpectedStatus 200 -Description "Get document"
        
        $updateDoc = '{"document":{"title":"Updated Document","content":"Updated content"}}'
        Test-Endpoint -Method "PUT" -Endpoint "/api/v1/indices/$TestIndex/documents/$docId" -Data $updateDoc -ExpectedStatus 200 -Description "Update document"
    }
} catch {
    Write-Host "  Warning: Could not test document operations: $($_.Exception.Message)" -ForegroundColor Yellow
}

# 5. Search Operations
Write-Host ""
Write-Host "=== 5. Search Operations ==="
$searchPost = '{"query":{"match":{"field":"title","query":"Test"}}}'
Test-Endpoint -Method "POST" -Endpoint "/api/v1/indices/$TestIndex/search" -Data $searchPost -ExpectedStatus 200 -Description "POST search"

Test-Endpoint -Method "GET" -Endpoint "/api/v1/indices/$TestIndex/search?q=Test" -ExpectedStatus 200 -Description "GET search with query string"

$searchWithFilter = '{"query":{"match":{"field":"title","query":"Test"}},"filter":[{"term":{"field":"title","value":"Test"}}]}'
Test-Endpoint -Method "POST" -Endpoint "/api/v1/indices/$TestIndex/search" -Data $searchWithFilter -ExpectedStatus 200 -Description "Search with filter"

# 6. Bulk Operations
Write-Host ""
Write-Host "=== 6. Bulk Operations ==="
$bulk = "{`"operations`":[{`"action`":`"index`",`"index`":`"$TestIndex`",`"document`":{`"title`":`"Bulk Doc 1`",`"content`":`"Content 1`"}},{`"action`":`"index`",`"index`":`"$TestIndex`",`"document`":{`"title`":`"Bulk Doc 2`",`"content`":`"Content 2`"}}]}"
Test-Endpoint -Method "POST" -Endpoint "/api/v1/bulk" -Data $bulk -ExpectedStatus 200 -Description "Bulk operations"

# 7. Snapshot Repository Operations
Write-Host ""
Write-Host "=== 7. Snapshot Repository Operations ==="
$repoConfig = '{"type":"fs","settings":{"location":"C:\\tmp\\test_repo"}}'
Test-Endpoint -Method "PUT" -Endpoint "/_snapshot/$TestRepo" -Data $repoConfig -ExpectedStatus 200 -Description "Create snapshot repository"
Test-Endpoint -Method "GET" -Endpoint "/_snapshot/$TestRepo" -ExpectedStatus 200 -Description "Get snapshot repository"
Test-Endpoint -Method "GET" -Endpoint "/_snapshot" -ExpectedStatus 200 -Description "List snapshot repositories"

# 8. Snapshot Operations
Write-Host ""
Write-Host "=== 8. Snapshot Operations ==="
$snapshotConfig = "{`"indices`":[`"$TestIndex`"],`"include_global_state`":false}"
Test-Endpoint -Method "PUT" -Endpoint "/_snapshot/$TestRepo/$TestSnapshot" -Data $snapshotConfig -ExpectedStatus 200 -Description "Create snapshot"
Test-Endpoint -Method "GET" -Endpoint "/_snapshot/$TestRepo/$TestSnapshot" -ExpectedStatus 200 -Description "Get snapshot"
Test-Endpoint -Method "GET" -Endpoint "/_snapshot/$TestRepo/_all" -ExpectedStatus 200 -Description "List snapshots"
Test-Endpoint -Method "GET" -Endpoint "/_snapshot/$TestRepo/_stats" -ExpectedStatus 200 -Description "Get snapshot stats"
Test-Endpoint -Method "GET" -Endpoint "/_snapshot/_stats" -ExpectedStatus 200 -Description "Get global snapshot stats"

# 9. Template Operations
Write-Host ""
Write-Host "=== 9. Template Operations ==="
$templateConfig = "{`"index_patterns`":[`"test_*`"],`"mappings`":{`"fields`":$schema},`"settings`":{`"number_of_shards`":1}}"
Test-Endpoint -Method "PUT" -Endpoint "/_template/$TestTemplate" -Data $templateConfig -ExpectedStatus 200 -Description "Create template"
Test-Endpoint -Method "GET" -Endpoint "/_template/$TestTemplate" -ExpectedStatus 200 -Description "Get template"
Test-Endpoint -Method "GET" -Endpoint "/_template" -ExpectedStatus 200 -Description "List templates"

# 10. Alias Operations
Write-Host ""
Write-Host "=== 10. Alias Operations ==="
$aliasOps = "{`"actions`":[{`"add`":{`"index`":`"$TestIndex`",`"alias`":`"$TestAlias`"}}]}"
Test-Endpoint -Method "POST" -Endpoint "/_aliases" -Data $aliasOps -ExpectedStatus 200 -Description "Add alias"
Test-Endpoint -Method "GET" -Endpoint "/_aliases" -ExpectedStatus 200 -Description "List all aliases"
Test-Endpoint -Method "GET" -Endpoint "/$TestIndex/_alias" -ExpectedStatus 200 -Description "Get index aliases"
Test-Endpoint -Method "GET" -Endpoint "/$TestIndex/_alias/$TestAlias" -ExpectedStatus 200 -Description "Get specific alias"

# 11. Progress Tracking
Write-Host ""
Write-Host "=== 11. Progress Tracking ==="
Test-Endpoint -Method "GET" -Endpoint "/api/v1/progress" -ExpectedStatus 200 -Description "List progress sessions"
Test-Endpoint -Method "GET" -Endpoint "/api/v1/progress/stats" -ExpectedStatus 200 -Description "Get progress stats"

# 12. Reindex Operations
Write-Host ""
Write-Host "=== 12. Reindex Operations ==="
$reindexDest = "reindex_dest_$(Get-Date -Format 'yyyyMMddHHmmss')"
$reindexConfig = "{`"source`":{`"index`":`"$TestIndex`"},`"dest`":{`"index`":`"$reindexDest`"}}"
# Note: This will fail if destination index doesn't exist, which is expected
Test-Endpoint -Method "POST" -Endpoint "/_reindex" -Data $reindexConfig -ExpectedStatus 400 -Description "Reindex operation (expected to fail without dest index)"

Test-Endpoint -Method "GET" -Endpoint "/_tasks" -ExpectedStatus 200 -Description "List tasks"

# 13. Rollover Operations
Write-Host ""
Write-Host "=== 13. Rollover Operations ==="
Test-Endpoint -Method "GET" -Endpoint "/api/v1/indices/$TestIndex/_rollover" -ExpectedStatus 200 -Description "Get rollover conditions"
$rolloverConfig = '{"conditions":{"max_age":"30d","max_docs":1000}}'
Test-Endpoint -Method "PUT" -Endpoint "/api/v1/indices/$TestIndex/_rollover" -Data $rolloverConfig -ExpectedStatus 200 -Description "Update rollover conditions"

# Cleanup
Write-Host ""
Write-Host "=== Cleanup ==="
Test-Endpoint -Method "DELETE" -Endpoint "/_template/$TestTemplate" -ExpectedStatus 200 -Description "Delete template"
Test-Endpoint -Method "DELETE" -Endpoint "/_snapshot/$TestRepo/$TestSnapshot" -ExpectedStatus 200 -Description "Delete snapshot"
Test-Endpoint -Method "DELETE" -Endpoint "/api/v1/indices/$TestIndex" -ExpectedStatus 200 -Description "Delete test index"

# Summary
Write-Host ""
Write-Host "=========================================="
Write-Host "Test Summary"
Write-Host "=========================================="
Write-Host "Passed: $Passed" -ForegroundColor Green
Write-Host "Failed: $Failed" -ForegroundColor Red
Write-Host "Total: $($Passed + $Failed)"

if ($Failed -eq 0) {
    Write-Host "All tests passed!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "Some tests failed!" -ForegroundColor Red
    exit 1
}

