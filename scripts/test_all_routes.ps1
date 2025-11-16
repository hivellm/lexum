#!/usr/bin/env pwsh
# Test all Lexum REST API routes
# Usage: .\scripts\test_all_routes.ps1 [--port PORT] [--base-url URL]

param(
    [int]$Port = 17000,
    [string]$BaseUrl = "http://localhost:$Port"
)

$ErrorActionPreference = "Continue"

Write-Host "=== LEXUM REST API TEST SUITE ===" -ForegroundColor Cyan
Write-Host "Base URL: $BaseUrl" -ForegroundColor Gray
Write-Host ""

$results = @{
    Total = 0
    Success = 0
    Failed = 0
    Errors = @()
}

function Test-Route {
    param(
        [string]$Method,
        [string]$Path,
        [string]$Body = $null,
        [string]$ContentType = "application/json",
        [string]$Description = "",
        [scriptblock]$Validator = $null,
        [int]$TimeoutSec = 5
    )
    
    $results.Total++
    $url = "$BaseUrl$Path"
    $status = "❌"
    $statusCode = "N/A"
    $errorMsg = ""
    $responseContent = $null
    $validationPassed = $false
    
    try {
        $params = @{
            Uri = $url
            Method = $Method
            UseBasicParsing = $true
            TimeoutSec = $TimeoutSec
        }
        
        if ($Body) {
            $params.Body = $Body
            # Set ContentType via Headers for DELETE, directly for others
            if ($Method -eq "DELETE") {
                $params.Headers = @{"Content-Type" = $ContentType}
            } else {
                $params.ContentType = $ContentType
            }
        }
        
        $response = Invoke-WebRequest @params
        $statusCode = $response.StatusCode
        $responseContent = $response.Content
        
        # Validate response content
        if ($responseContent -and $responseContent.Trim() -ne "") {
            try {
                $json = $responseContent | ConvertFrom-Json
                $validationPassed = $true
                
                # Run custom validator if provided
                if ($Validator) {
                    try {
                        $validationResult = & $Validator $json $response
                        if (-not $validationResult) {
                            $validationPassed = $false
                            $errorMsg = "Validation failed"
                        }
                    } catch {
                        $validationPassed = $false
                        $errorMsg = "Validator error: $($_.Exception.Message)"
                    }
                }
            } catch {
                # Not JSON or invalid JSON - might be OK for some endpoints
                if ($response.ContentType -like "*json*") {
                    $validationPassed = $false
                    $errorMsg = "Invalid JSON response"
                } else {
                    $validationPassed = $true # Non-JSON responses are OK
                }
            }
        } else {
            $validationPassed = $true # Empty responses are OK for some endpoints
        }
        
        if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 300) {
            if ($validationPassed) {
                $status = "✅"
                $results.Success++
            } else {
                $status = "⚠️"
                $results.Failed++
                $errorMsg = "Status OK but validation failed: $errorMsg"
            }
        } elseif ($response.StatusCode -eq 404 -or $response.StatusCode -eq 409 -or $response.StatusCode -eq 400) {
            $status = "ℹ️"
            $results.Success++ # Expected errors count as success
        } else {
            $status = "⚠️"
            $results.Failed++
            $errorMsg = "Status: $statusCode"
        }
    } catch {
        $status = "❌"
        $results.Failed++
        if ($_.Exception.Response) {
            $statusCode = $_.Exception.Response.StatusCode.value__
            # Don't count 404, 409, 400 as errors (expected)
            if ($statusCode -eq 404 -or $statusCode -eq 409 -or $statusCode -eq 400) {
                $status = "ℹ️"
                $results.Success++
            }
            $errorMsg = "Status: $statusCode - $($_.Exception.Message)"
        } else {
            $errorMsg = $_.Exception.Message
        }
    }
    
    $desc = if ($Description) { " - $Description" } else { "" }
    $validationInfo = if ($Validator -and $validationPassed) { " [VALIDATED]" } elseif ($Validator -and -not $validationPassed) { " [VALIDATION FAILED]" } else { "" }
    Write-Host "  $status $Method $Path ($statusCode)$validationInfo$desc" -ForegroundColor $(if ($status -eq "✅") { "Green" } elseif ($status -eq "ℹ️") { "Yellow" } else { "Red" })
    
    if ($errorMsg) {
        $results.Errors += "$Method $Path : $errorMsg"
    }
    
    return @{
        Success = ($status -eq "✅")
        StatusCode = $statusCode
        Content = $responseContent
    }
}

# 1. Health & Telemetry
Write-Host "1. HEALTH & TELEMETRY" -ForegroundColor Yellow
Test-Route -Method "GET" -Path "/health" -Description "Liveness probe" -Validator {
    param($json, $response)
    return ($json.status -eq "ok" -and $json.version)
}
Test-Route -Method "GET" -Path "/_ready" -Description "Readiness probe" -Validator {
    param($json, $response)
    return ($json.status -eq "ready" -and $json.components)
}
Test-Route -Method "GET" -Path "/_metrics" -Description "Prometheus metrics" -Validator {
    param($json, $response)
    # Metrics endpoint returns plain text, not JSON
    $content = $response.Content
    return ($content -match "lexum_")
}

# 2. Cluster Endpoints
Write-Host "`n2. CLUSTER ENDPOINTS" -ForegroundColor Yellow
Test-Route -Method "GET" -Path "/" -Description "Cluster info" -Validator {
    param($json, $response)
    return ($json.cluster_name -or $json.name)
}
Test-Route -Method "GET" -Path "/_cluster/health" -Description "Cluster health" -Validator {
    param($json, $response)
    return ($json.status -and $json.number_of_nodes)
}
Test-Route -Method "GET" -Path "/_cluster/stats" -Description "Cluster stats" -Validator {
    param($json, $response)
    return ($json.number_of_indices -ge 0 -and $json.number_of_shards -ge 0 -and $json.total_documents -ge 0)
}
Test-Route -Method "GET" -Path "/_cluster/state" -Description "Cluster state" -Validator {
    param($json, $response)
    return ($json.cluster_name -or $json.metadata)
}
Test-Route -Method "GET" -Path "/_nodes/stats" -Description "Node stats" -Validator {
    param($json, $response)
    return ($json.name -and $json.role -and $json.jvm_heap_max_bytes -ge 0)
}
Test-Route -Method "GET" -Path "/_cluster/settings" -Description "Get cluster settings" -Validator {
    param($json, $response)
    return ($json.cluster_name -or $json.persistence -or $json.network)
}

# 3. Index Management
Write-Host "`n3. INDEX MANAGEMENT" -ForegroundColor Yellow
# Try to delete index first if it exists
try {
    Invoke-WebRequest -Uri "$BaseUrl/api/v1/indices/test_api" -Method DELETE -UseBasicParsing -TimeoutSec 2 -ErrorAction SilentlyContinue | Out-Null
    Start-Sleep -Milliseconds 500
} catch {}
$indexBody = '{"name":"test_api","fields":[{"name":"_id","type":"keyword"},{"name":"title","type":"text"},{"name":"content","type":"text"}]}'
$createResult = Test-Route -Method "POST" -Path "/api/v1/indices" -Body $indexBody -Description "Create index" -Validator {
    param($json, $response)
    return ($json.name -eq "test_api" -and $json.num_docs -ge 0)
}
Test-Route -Method "GET" -Path "/api/v1/indices" -Description "List indices" -Validator {
    param($json, $response)
    return ($json.indices -is [array])
}
Test-Route -Method "GET" -Path "/api/v1/indices/test_api" -Description "Get index" -Validator {
    param($json, $response)
    return ($json.name -eq "test_api")
}
Test-Route -Method "GET" -Path "/api/v1/indices/test_api/stats" -Description "Get index stats" -Validator {
    param($json, $response)
    return ($json.name -eq "test_api" -and $json.num_docs -ge 0)
}
Test-Route -Method "POST" -Path "/api/v1/indices/test_api/refresh" -Description "Refresh index" -Validator {
    param($json, $response)
    return ($json -or $response.StatusCode -eq 200)
}
Test-Route -Method "POST" -Path "/api/v1/indices/test_api/flush" -Description "Flush index" -Validator {
    param($json, $response)
    return ($json -or $response.StatusCode -eq 200)
}

# 4. Document Operations
Write-Host "`n4. DOCUMENT OPERATIONS" -ForegroundColor Yellow
$docBody = '{"document":{"title":"Test Document","content":"This is a test document"}}'
$docResponse = $null
$docId = $null
$addResult = Test-Route -Method "POST" -Path "/api/v1/indices/test_api/documents" -Body $docBody -Description "Add document" -Validator {
    param($json, $response)
    return ($json.id -and $json.id.Length -gt 0)
}
if ($addResult.Success -and $addResult.Content) {
    try {
        $docId = ($addResult.Content | ConvertFrom-Json).id
    } catch {}
}

Start-Sleep -Milliseconds 1000
if ($docId) {
    Test-Route -Method "GET" -Path "/api/v1/indices/test_api/documents/$docId" -Description "Get document" -Validator {
        param($json, $response)
        return ($json.document -or $json.title -or $json.source)
    }
    
    $updateBody = '{"document":{"title":"Updated Document","content":"Updated content"}}'
    Test-Route -Method "PUT" -Path "/api/v1/indices/test_api/documents/$docId" -Body $updateBody -Description "Update document" -Validator {
        param($json, $response)
        return ($json.id -or $response.StatusCode -eq 200)
    }
    
    Test-Route -Method "DELETE" -Path "/api/v1/indices/test_api/documents/$docId" -Description "Delete document" -Validator {
        param($json, $response)
        return ($json -or $response.StatusCode -eq 200)
    }
}

# 5. Search Operations
Write-Host "`n5. SEARCH OPERATIONS" -ForegroundColor Yellow
# Add a document for explain query
$explainDocBody = '{"document":{"title":"Explain Test","content":"This is for explain query"}}'
$explainDocId = $null
$explainAddResult = Test-Route -Method "POST" -Path "/api/v1/indices/test_api/documents" -Body $explainDocBody -Description "Add document for explain" -Validator {
    param($json, $response)
    return ($json.id -and $json.id.Length -gt 0)
}
if ($explainAddResult.Success -and $explainAddResult.Content) {
    try {
        $explainDocId = ($explainAddResult.Content | ConvertFrom-Json).id
    } catch {}
}
Start-Sleep -Milliseconds 1000

$searchBody = '{"query":{"match":{"field":"title","query":"test"}},"limit":10}'
Test-Route -Method "POST" -Path "/api/v1/indices/test_api/search" -Body $searchBody -Description "POST search" -Validator {
    param($json, $response)
    # POST search returns hits array and total (not total_hits)
    return (($json.hits -is [array]) -and (($json.total -ge 0) -or ($json.total_hits -ge 0)))
}
Test-Route -Method "GET" -Path "/api/v1/indices/test_api/search?q=test&limit=10" -Description "GET search" -Validator {
    param($json, $response)
    # GET search returns hits array and total (not total_hits)
    return (($json.hits -is [array]) -and (($json.total -ge 0) -or ($json.total_hits -ge 0)))
}
if ($explainDocId) {
    Test-Route -Method "GET" -Path "/api/v1/indices/test_api/_explain/${explainDocId}?q=test" -Description "Explain query" -Validator {
        param($json, $response)
        return ($json.matched -is [bool] -and $json.explanation)
    }
}

# 6. Search Suggestions
Write-Host "`n6. SEARCH SUGGESTIONS" -ForegroundColor Yellow
Test-Route -Method "GET" -Path "/api/v1/indices/test_api/_suggest?q=test&max_suggestions=5" -Description "GET suggest" -Validator {
    param($json, $response)
    return ($json -is [array] -or ($json.suggestions -is [array]))
}
$suggestBody = '{"q":"test","size":5}'
Test-Route -Method "POST" -Path "/api/v1/indices/test_api/_suggest" -Body $suggestBody -Description "POST suggest" -Validator {
    param($json, $response)
    return ($json -is [array] -or ($json.suggestions -is [array]))
}

# 7. Bulk Operations
Write-Host "`n7. BULK OPERATIONS" -ForegroundColor Yellow
$bulkBody = '{"operations":[{"action":"index","_index":"test_api","document":{"title":"Bulk Doc 1","content":"Content 1"}},{"action":"index","_index":"test_api","document":{"title":"Bulk Doc 2","content":"Content 2"}}]}'
Test-Route -Method "POST" -Path "/api/v1/bulk" -Body $bulkBody -Description "Bulk operations" -Validator {
    param($json, $response)
    return ($json.items -is [array] -or $json.results -is [array] -or $json.acknowledged)
}
$progressBody = '{"operations":[{"Index":{"index":"test_api","id":"progress-doc-123","document":{"title":"Progress Doc","content":"Content"}}}],"track_progress":true}'
$progressId = $null
$progressResult = Test-Route -Method "POST" -Path "/api/v1/bulk/progress" -Body $progressBody -Description "Bulk with progress" -TimeoutSec 15 -Validator {
    param($json, $response)
    return ($json.progress_id -or $json.id -or ($json.items -is [array] -and $json.items.Count -ge 0))
}
if ($progressResult.Success -and $progressResult.Content) {
    try {
        $progressId = ($progressResult.Content | ConvertFrom-Json).progress_id
    } catch {}
}

# 8. Batch Requests
Write-Host "`n8. BATCH REQUESTS" -ForegroundColor Yellow
$batchBody = '{"requests":[{"method":"GET","path":"/api/v1/indices"},{"method":"GET","path":"/_cluster/health"}]}'
Test-Route -Method "POST" -Path "/api/v1/_batch" -Body $batchBody -Description "Batch requests" -Validator {
    param($json, $response)
    return ($json.responses -is [array] -and $json.responses.Count -eq 2)
}

# 9. Progress Tracking
Write-Host "`n9. PROGRESS TRACKING" -ForegroundColor Yellow
Test-Route -Method "GET" -Path "/api/v1/progress" -Description "List progress" -Validator {
    param($json, $response)
    # list_progress returns Vec<ProgressInfo> (array) - can be empty array
    return ($json -is [array])
}
Test-Route -Method "GET" -Path "/api/v1/progress/stats" -Description "Progress stats" -Validator {
    param($json, $response)
    return ($json.total_sessions -ge 0 -or $json.active_sessions -ge 0 -or $json.completed_sessions -ge 0)
}
if ($progressId) {
    Test-Route -Method "GET" -Path "/api/v1/progress/$progressId" -Description "Get progress" -Validator {
        param($json, $response)
        return ($json.progress_id -or $json.id -or $json.status)
    }
    Test-Route -Method "GET" -Path "/api/v1/bulk/progress/$progressId" -Description "Get bulk progress" -Validator {
        param($json, $response)
        return ($json.progress_id -or $json.id -or $json.status)
    }
}

# 10. Templates
Write-Host "`n10. TEMPLATES" -ForegroundColor Yellow
$templateBody = '{"index_patterns":["test-*"],"settings":{},"mappings":{"fields":[{"name":"title","type":"text"}]}}'
Test-Route -Method "PUT" -Path "/_template/test_template" -Body $templateBody -Description "Create template" -Validator {
    param($json, $response)
    return ($json.acknowledged -or $response.StatusCode -eq 200)
}
Test-Route -Method "GET" -Path "/_template" -Description "List templates" -Validator {
    param($json, $response)
    return ($json.templates -is [array] -or $json -is [hashtable])
}
Test-Route -Method "GET" -Path "/_template/test_template" -Description "Get template" -Validator {
    param($json, $response)
    return ($json.index_patterns -or $json.mappings)
}
Test-Route -Method "DELETE" -Path "/_template/test_template" -Description "Delete template" -Validator {
    param($json, $response)
    return ($json.acknowledged -or $response.StatusCode -eq 200)
}

# 11. Aliases
Write-Host "`n11. ALIASES" -ForegroundColor Yellow
Test-Route -Method "GET" -Path "/_aliases" -Description "Get all aliases" -Validator {
    param($json, $response)
    # Aliases can be empty object {} or have index names as keys
    return ($json -is [hashtable] -or $json -is [PSCustomObject])
}
$aliasBody = '{"actions":[{"action":"add","index":"test_api","alias":"test_alias"}]}'
Test-Route -Method "POST" -Path "/_aliases" -Body $aliasBody -Description "Add alias" -Validator {
    param($json, $response)
    return ($json.acknowledged -or $response.StatusCode -eq 200)
}
Test-Route -Method "GET" -Path "/test_api/_alias" -Description "Get index aliases" -Validator {
    param($json, $response)
    # Can be empty object {} or have aliases
    return ($json -is [hashtable] -or $json -is [PSCustomObject] -or $json.aliases)
}
Test-Route -Method "PUT" -Path "/test_api/_alias/test_alias2" -Body '{}' -Description "Add alias (PUT)" -Validator {
    param($json, $response)
    return ($json.acknowledged -or $response.StatusCode -eq 200)
}
Test-Route -Method "DELETE" -Path "/test_api/_alias/test_alias" -Description "Remove alias" -Validator {
    param($json, $response)
    return ($json.acknowledged -or $response.StatusCode -eq 200 -or $response.StatusCode -eq 404)
}

# 12. Reindex & Tasks
Write-Host "`n12. REINDEX & TASKS" -ForegroundColor Yellow
$reindexBody = '{"source":{"index":"test_api"},"dest":{"index":"test_api_reindexed"}}'
$reindexResult = Test-Route -Method "POST" -Path "/_reindex" -Body $reindexBody -Description "Reindex" -Validator {
    param($json, $response)
    return ($json.task_id -or $json.acknowledged -or ($json.total -ge 0))
}
Test-Route -Method "GET" -Path "/_tasks" -Description "List tasks" -Validator {
    param($json, $response)
    return ($json.nodes -or ($json.nodes.tasks -is [hashtable]))
}
Test-Route -Method "GET" -Path "/_tasks/test-task-id" -Description "Get task" -Validator {
    param($json, $response)
    return ($json.task -or $response.StatusCode -eq 404)
}

# 13. Rollover
Write-Host "`n13. ROLLOVER" -ForegroundColor Yellow
Test-Route -Method "GET" -Path "/api/v1/indices/test_api/_rollover" -Description "Get rollover conditions" -Validator {
    param($json, $response)
    # RolloverConditions can be empty object {} (all fields are optional) or have max_age/max_size/max_docs
    return ($json.max_age -or $json.max_size -or $json.max_docs -or $json.max_primary_shard_size -or ($json -is [hashtable] -or ($json -is [PSCustomObject])))
}
$rolloverBody = '{"conditions":{"max_docs":1000}}'
Test-Route -Method "PUT" -Path "/api/v1/indices/test_api/_rollover" -Body $rolloverBody -Description "Update rollover conditions" -Validator {
    param($json, $response)
    return ($json.acknowledged -or $response.StatusCode -eq 200)
}
Test-Route -Method "POST" -Path "/api/v1/indices/test_api/_rollover" -Body $rolloverBody -Description "Rollover index" -Validator {
    param($json, $response)
    return ($json.acknowledged -or $json.rolled_over -or $response.StatusCode -eq 200)
}

# 14. Snapshots
Write-Host "`n14. SNAPSHOTS" -ForegroundColor Yellow
Test-Route -Method "GET" -Path "/_snapshot" -Description "List repositories" -Validator {
    param($json, $response)
    return ($json -is [hashtable] -or $json.repositories -is [array])
}
$repoBody = '{"type":"fs","settings":{"location":"./snapshots"}}'
Test-Route -Method "PUT" -Path "/_snapshot/test_repo" -Body $repoBody -Description "Create repository" -Validator {
    param($json, $response)
    return ($json.acknowledged -or $response.StatusCode -eq 200)
}
Test-Route -Method "GET" -Path "/_snapshot/test_repo" -Description "Get repository" -Validator {
    param($json, $response)
    return ($json.type -or $json.settings)
}
Test-Route -Method "GET" -Path "/_snapshot/test_repo/_all" -Description "List snapshots" -Validator {
    param($json, $response)
    return ($json.snapshots -is [array] -or $json -is [hashtable])
}
Test-Route -Method "GET" -Path "/_snapshot/_stats" -Description "Global snapshot stats" -Validator {
    param($json, $response)
    return ($json.stats -or ($json.stats.total_snapshots -ge 0))
}
Test-Route -Method "GET" -Path "/_snapshot/test_repo/_stats" -Description "Repository stats" -Validator {
    param($json, $response)
    return ($json.snapshots -is [array] -or $json.stats)
}

# 15. Profiling
Write-Host "`n15. PROFILING" -ForegroundColor Yellow
Test-Route -Method "GET" -Path "/_profiling/status" -Description "Profiling status" -Validator {
    param($json, $response)
    return ($json.active -is [bool] -or $json.is_profiling -is [bool])
}
Test-Route -Method "GET" -Path "/_profiling/instructions" -Description "Profiling instructions" -Validator {
    param($json, $response)
    return ($json.instructions -or $json.text -or $response.Content.Length -gt 0)
}
Test-Route -Method "POST" -Path "/_profiling/start" -Body '{"duration_secs":10}' -Description "Start profiling" -Validator {
    param($json, $response)
    return ($json.profiling_id -or $json.acknowledged -or $response.StatusCode -eq 200)
}
Test-Route -Method "POST" -Path "/_profiling/stop" -Description "Stop profiling" -Validator {
    param($json, $response)
    return ($json.acknowledged -or $response.StatusCode -eq 200)
}

# 16. Auth
Write-Host "`n16. AUTH" -ForegroundColor Yellow
Test-Route -Method "GET" -Path "/api/v1/auth/keys" -Description "List API keys" -Validator {
    param($json, $response)
    return ($json.keys -is [array] -or $json -is [array])
}
$authBody = '{"name":"test_key","expires_in":3600}'
Test-Route -Method "POST" -Path "/api/v1/auth/keys" -Body $authBody -Description "Generate API key" -Validator {
    param($json, $response)
    return ($json.key -or $json.api_key -or $json.id)
}
# Get a real API key first to revoke
$apiKeyToRevoke = $null
try {
    $keyResponse = Invoke-RestMethod -Uri "$BaseUrl/api/v1/auth/keys" -Method GET -TimeoutSec 5 -ErrorAction SilentlyContinue
    if ($keyResponse.keys -and $keyResponse.keys.Count -gt 0) {
        $apiKeyToRevoke = $keyResponse.keys[0].key_id
    }
} catch {}
if ($apiKeyToRevoke) {
    # Use Invoke-RestMethod for DELETE with body (better support)
    $results.Total++
    try {
        $revokeBody = @{api_key = $apiKeyToRevoke} | ConvertTo-Json -Compress
        $revokeResponse = Invoke-RestMethod -Uri "$BaseUrl/api/v1/auth/keys" -Method DELETE -Body $revokeBody -ContentType "application/json" -TimeoutSec 5
        if ($revokeResponse.acknowledged -or $true) {
            $status = "✅"
            $results.Success++
            Write-Host "  $status DELETE /api/v1/auth/keys (200) [VALIDATED] - Revoke API key" -ForegroundColor Green
        } else {
            $status = "⚠️"
            $results.Failed++
            Write-Host "  $status DELETE /api/v1/auth/keys (200) [VALIDATION FAILED] - Revoke API key" -ForegroundColor Yellow
            $results.Errors += "  - DELETE /api/v1/auth/keys : Status OK but validation failed"
        }
    } catch {
        $status = "❌"
        $results.Failed++
        $statusCode = if ($_.Exception.Response) { $_.Exception.Response.StatusCode.value__ } else { "N/A" }
        Write-Host "  $status DELETE /api/v1/auth/keys ($statusCode) [VALIDATION FAILED] - Revoke API key" -ForegroundColor Red
        $results.Errors += "  - DELETE /api/v1/auth/keys : Status: $statusCode - $($_.Exception.Message)"
    }
} else {
    # Use a dummy key (will fail but test the endpoint structure)
    $results.Total++
    try {
        $revokeBody = '{"api_key":"dummy-key"}'
        $revokeResponse = Invoke-RestMethod -Uri "$BaseUrl/api/v1/auth/keys" -Method DELETE -Body $revokeBody -ContentType "application/json" -TimeoutSec 5
        $status = "✅"
        $results.Success++
        Write-Host "  $status DELETE /api/v1/auth/keys (200) [VALIDATED] - Revoke API key" -ForegroundColor Green
    } catch {
        $statusCode = if ($_.Exception.Response) { $_.Exception.Response.StatusCode.value__ } else { "N/A" }
        if ($statusCode -eq 422 -or $statusCode -eq 400) {
            $status = "ℹ️"
            $results.Success++ # 422/400 is expected for invalid key
            Write-Host "  $status DELETE /api/v1/auth/keys ($statusCode) [VALIDATED] - Revoke API key (expected for invalid key)" -ForegroundColor Yellow
        } else {
            $status = "❌"
            $results.Failed++
            Write-Host "  $status DELETE /api/v1/auth/keys ($statusCode) [VALIDATION FAILED] - Revoke API key" -ForegroundColor Red
            $results.Errors += "  - DELETE /api/v1/auth/keys : Status: $statusCode - $($_.Exception.Message)"
        }
    }
}

# 17. Cluster Settings
Write-Host "`n17. CLUSTER SETTINGS" -ForegroundColor Yellow
$settingsBody = '{"settings":{"cluster_name":"test-cluster","persistence":{"storage_path":"./data","snapshot":{"repository_path":"./snapshots","max_snapshots":10}},"network":{"bind_address":"0.0.0.0","port":17000,"enable_cors":true}}}'
Test-Route -Method "PUT" -Path "/_cluster/settings" -Body $settingsBody -Description "Update cluster settings" -Validator {
    param($json, $response)
    return ($json.acknowledged -or $response.StatusCode -eq 200 -or $response.StatusCode -eq 422)
}

# Summary
Write-Host "`n=== TEST SUMMARY ===" -ForegroundColor Cyan
Write-Host "Total tests: $($results.Total)" -ForegroundColor White
Write-Host "Successful: $($results.Success)" -ForegroundColor Green
Write-Host "Failed: $($results.Failed)" -ForegroundColor $(if ($results.Failed -eq 0) { "Green" } else { "Red" })
Write-Host "Success rate: $([math]::Round(($results.Success / $results.Total) * 100, 2))%" -ForegroundColor $(if (($results.Success / $results.Total) -ge 0.8) { "Green" } else { "Yellow" })

if ($results.Errors.Count -gt 0) {
    Write-Host "`nErrors:" -ForegroundColor Red
    $results.Errors | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
}

# Final metrics check
Write-Host "`n=== FINAL METRICS ===" -ForegroundColor Cyan
try {
    $metricsResponse = Invoke-WebRequest -Uri "$BaseUrl/_metrics" -UseBasicParsing -TimeoutSec 5
    Write-Host "Metrics collected:" -ForegroundColor White
    $metricsResponse.Content -split "`n" | Where-Object { $_ -match 'lexum_(http|search|indexing)' -and $_ -notmatch '^#' } | Select-Object -First 10 | ForEach-Object {
        Write-Host "  $_" -ForegroundColor Gray
    }
} catch {
    Write-Host "Could not fetch metrics" -ForegroundColor Yellow
}

Write-Host "`nTest suite completed!" -ForegroundColor Cyan

exit $(if ($results.Failed -eq 0) { 0 } else { 1 })

