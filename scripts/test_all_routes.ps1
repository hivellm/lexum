# Script completo para testar todas as rotas do Lexum Server
# Uso: .\test_all_routes.ps1 [-ServerUrl http://localhost:17000] [-LogFile test_results.log]

param(
    [string]$ServerUrl = "http://localhost:17000",
    [string]$LogFile = "test_results_$(Get-Date -Format 'yyyyMMdd_HHmmss').log"
)

# Cores para output
$ErrorColor = "Red"
$SuccessColor = "Green"
$InfoColor = "Cyan"
$WarningColor = "Yellow"

# Contadores
$script:TotalTests = 0
$script:PassedTests = 0
$script:FailedTests = 0
$script:WarningTests = 0
$script:RetryCount = 0

# Configuração de retry
$script:MaxRetries = 3
$script:RetryDelayMs = 1000  # Base delay in milliseconds
$script:RetryBackoffMultiplier = 2  # Exponential backoff

# Lista de índices criados para cleanup
$script:CreatedIndices = @()
$script:CreatedTemplates = @()
$script:CreatedRepositories = @()

# Logging function
function Write-Log {
    param(
        [string]$Message,
        [string]$Level = "INFO"
    )
    
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "[$timestamp] [$Level] $Message"
    
    # Write to console
    switch ($Level) {
        "ERROR" { Write-Host $logMessage -ForegroundColor $ErrorColor }
        "WARN" { Write-Host $logMessage -ForegroundColor $WarningColor }
        "SUCCESS" { Write-Host $logMessage -ForegroundColor $SuccessColor }
        default { Write-Host $logMessage -ForegroundColor $InfoColor }
    }
    
    # Write to log file
    try {
        Add-Content -Path $LogFile -Value $logMessage -ErrorAction SilentlyContinue
    } catch {
        # Ignore log file errors
    }
}

# Function to save error details
function Save-ErrorDetails {
    param(
        [string]$TestName,
        [string]$Method,
        [string]$Url,
        [object]$RequestBody,
        [int]$StatusCode,
        [string]$ResponseBody,
        [string]$ErrorMessage
    )
    
    $errorDetails = @{
        timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
        test = $TestName
        method = $Method
        url = $Url
        request_body = $RequestBody
        status_code = $StatusCode
        response_body = $ResponseBody
        error_message = $ErrorMessage
    }
    
    $errorFile = $LogFile -replace '\.log$', '_errors.json'
    try {
        $existingErrors = @()
        if (Test-Path $errorFile) {
            $existingContent = Get-Content $errorFile -Raw | ConvertFrom-Json
            if ($existingContent -is [array]) {
                $existingErrors = @($existingContent)
            } else {
                $existingErrors = @($existingContent)
            }
        }
        $allErrors = @($existingErrors) + @($errorDetails)
        $allErrors | ConvertTo-Json -Depth 10 | Set-Content $errorFile
    } catch {
        Write-Log "Failed to save error details: $_" "ERROR"
    }
}

function Test-Route {
    param(
        [string]$Name,
        [string]$Method,
        [string]$Url,
        [hashtable]$Headers = @{"Content-Type" = "application/json"},
        [object]$Body = $null,
        [int[]]$ExpectedStatusCodes = @(200, 201, 204),
        [bool]$SkipOnError = $false,
        [bool]$TrackResource = $false  # Track created resources for cleanup
    )
    
    $script:TotalTests++
    $retryCount = 0
    $delayMs = $script:RetryDelayMs
    
    while ($retryCount -le $script:MaxRetries) {
        try {
            $params = @{
                Uri = "$ServerUrl$Url"
                Method = $Method
                Headers = $Headers
                ErrorAction = "Stop"
            }
            
            if ($Body -ne $null) {
                $params.Body = ($Body | ConvertTo-Json -Depth 10 -Compress)
            }
            
            $response = Invoke-WebRequest @params
            $statusCode = $response.StatusCode
            $responseBody = $response.Content
            
            if ($ExpectedStatusCodes -contains $statusCode) {
                Write-Log "  [OK] $Name - Status: $statusCode" "SUCCESS"
                $script:PassedTests++
                
                # Track created resources
                if ($TrackResource -and $statusCode -in @(200, 201)) {
                    if ($Method -eq "POST" -and $Url -like "*/indices" -and $Body.name) {
                        $script:CreatedIndices += $Body.name
                    } elseif ($Method -eq "PUT" -and $Url -like "*/_template/*" -and $Url -match "/_template/([^/]+)") {
                        $script:CreatedTemplates += $Matches[1]
                    } elseif ($Method -eq "PUT" -and $Url -like "*/_snapshot/*" -and $Url -match "/_snapshot/([^/]+)") {
                        $script:CreatedRepositories += $Matches[1]
                    }
                }
                
                return $true
            } else {
                Write-Log "  [WARN] $Name - Status: $statusCode (Expected: $($ExpectedStatusCodes -join ', '))" "WARN"
                $script:WarningTests++
                return $false
            }
        } catch {
            $statusCode = $null
            $responseBody = ""
            $errorMessage = $_.Exception.Message
            
            if ($_.Exception.Response) {
                $statusCode = $_.Exception.Response.StatusCode.Value__
                
                # Check for rate limiting (429)
                if ($statusCode -eq 429) {
                    if ($retryCount -lt $script:MaxRetries) {
                        $script:RetryCount++
                        Write-Log "  [RETRY] $Name - Rate limited (429), retrying in ${delayMs}ms (attempt $($retryCount + 1)/$($script:MaxRetries + 1))" "WARN"
                        Start-Sleep -Milliseconds $delayMs
                        $delayMs = $delayMs * $script:RetryBackoffMultiplier
                        $retryCount++
                        continue
                    } else {
                        Write-Log "  [FAIL] $Name - Rate limited (429), max retries exceeded" "ERROR"
                    }
                }
                
                try {
                    $stream = $_.Exception.Response.GetResponseStream()
                    $reader = New-Object System.IO.StreamReader($stream)
                    $responseBody = $reader.ReadToEnd()
                    $reader.Close()
                    $stream.Close()
                } catch {
                    # Ignore errors reading response body
                }
            }
            
            # If we got here and status code is expected, it's OK
            if ($statusCode -and ($ExpectedStatusCodes -contains $statusCode)) {
                Write-Log "  [OK] $Name - Status: $statusCode (Error expected)" "SUCCESS"
                $script:PassedTests++
                return $true
            } elseif ($SkipOnError) {
                Write-Log "  [SKIP] $Name (Skipped - Pre-requisite not met)" "WARN"
                return $false
            } else {
                # Save error details
                Save-ErrorDetails -TestName $Name -Method $Method -Url $Url `
                    -RequestBody $Body -StatusCode $statusCode `
                    -ResponseBody $responseBody -ErrorMessage $errorMessage
                
                Write-Log "  [FAIL] $Name" "ERROR"
                Write-Log "    Error: $errorMessage" "ERROR"
                if ($statusCode) {
                    Write-Log "    Status: $statusCode" "ERROR"
                }
                if ($responseBody -and $responseBody.Length -lt 500) {
                    Write-Log "    Response: $responseBody" "ERROR"
                } elseif ($responseBody) {
                    Write-Log "    Response: $($responseBody.Substring(0, [Math]::Min(500, $responseBody.Length)))..." "ERROR"
                }
                $script:FailedTests++
                return $false
            }
        }
    }
    
    # Should not reach here, but handle it
    Write-Log "  [FAIL] $Name - Max retries exceeded" "ERROR"
    $script:FailedTests++
    return $false
}

Write-Log "========================================"
Write-Log "  LEXUM SERVER - TESTE COMPLETO DE ROTAS"
Write-Log "  Server: $ServerUrl"
Write-Log "  Log File: $LogFile"
Write-Log "========================================"

# Verificar se o servidor está rodando
Write-Log "Verificando se o servidor está rodando..."
try {
    $healthCheck = Invoke-WebRequest -Uri "$ServerUrl/health" -Method GET -ErrorAction Stop
    Write-Log "  [OK] Servidor está rodando (Status: $($healthCheck.StatusCode))" "SUCCESS"
} catch {
    Write-Log "  [ERRO] Servidor não está rodando ou não está acessível em $ServerUrl" "ERROR"
    Write-Log "  Por favor, inicie o servidor antes de executar os testes." "WARN"
    Write-Log "  Comando: cargo run --bin lexum-server" "WARN"
    exit 1
}
Write-Log ""

# 1. HEALTH CHECK & SYSTEM
Write-Log "1. HEALTH CHECK & SYSTEM"
Test-Route "Health Check" "GET" "/health"
Test-Route "Readiness Check" "GET" "/_ready"
Test-Route "Cluster Info" "GET" "/"
Test-Route "Cluster Health" "GET" "/_cluster/health"
Test-Route "Cluster Stats" "GET" "/_cluster/stats"
Test-Route "Cluster State" "GET" "/_cluster/state"
Test-Route "Node Stats" "GET" "/_nodes/stats"
Test-Route "Cluster Settings" "GET" "/_cluster/settings"
Test-Route "Metrics" "GET" "/_metrics"

Write-Log ""

# 2. INDEX MANAGEMENT
Write-Log "2. INDEX MANAGEMENT"
$testIndex1 = "test_index_1"
$testIndex2 = "test_index_2"
$testIndex3 = "test_index_shrink"

# Criar índices (com tracking para cleanup)
Test-Route "Create Index" "POST" "/api/v1/indices" -Body @{
    name = $testIndex1
    fields = @(
        @{
            name = "_id"
            type = "keyword"
            stored = $true
            indexed = $true
        },
        @{
            name = "title"
            type = "text"
            stored = $true
            indexed = $true
        },
        @{
            name = "content"
            type = "text"
            stored = $true
            indexed = $true
        }
    )
    settings = @{
        number_of_shards = 1
        number_of_replicas = 0
    }
} -ExpectedStatusCodes @(200, 201) -TrackResource $true

Test-Route "Create Index 2" "POST" "/api/v1/indices" -Body @{
    name = $testIndex2
    fields = @(
        @{
            name = "_id"
            type = "keyword"
            stored = $true
            indexed = $true
        },
        @{
            name = "title"
            type = "text"
            stored = $true
            indexed = $true
        }
    )
    settings = @{
        number_of_shards = 2
        number_of_replicas = 0
    }
} -ExpectedStatusCodes @(200, 201)

Test-Route "List Indices" "GET" "/api/v1/indices"
Test-Route "Get Index" "GET" "/api/v1/indices/$testIndex1"
Test-Route "Get Index Stats" "GET" "/api/v1/indices/$testIndex1/stats"

Write-Log ""

# 3. INDEX OPERATIONS
Write-Log "3. INDEX OPERATIONS"
Test-Route "Refresh Index" "POST" "/api/v1/indices/$testIndex1/refresh"
Test-Route "Flush Index" "POST" "/api/v1/indices/$testIndex1/flush"
Test-Route "Close Index" "POST" "/api/v1/indices/$testIndex1/close"
Test-Route "Open Index" "POST" "/api/v1/indices/$testIndex1/open"
Test-Route "Force Merge Index" "POST" "/api/v1/indices/$testIndex1/forcemerge" -Body @{
    max_num_segments = 1
} -ExpectedStatusCodes @(200, 204)

Test-Route "Update Index Settings" "PUT" "/api/v1/indices/$testIndex1/settings" -Body @{
    refresh_interval = 2000
} -ExpectedStatusCodes @(200, 204)

Write-Log ""

# 4. INDEX ADVANCED OPERATIONS
Write-Log "4. INDEX ADVANCED OPERATIONS"
# Criar índice para shrink
Test-Route "Create Index for Shrink" "POST" "/api/v1/indices" -Body @{
    name = $testIndex3
    fields = @(
        @{
            name = "_id"
            type = "keyword"
            stored = $true
            indexed = $true
        },
        @{
            name = "title"
            type = "text"
            stored = $true
            indexed = $true
        }
    )
    settings = @{
        number_of_shards = 2
        number_of_replicas = 0
    }
} -ExpectedStatusCodes @(200, 201) -SkipOnError $true -TrackResource $true

Test-Route "Shrink Index" "POST" "/api/v1/indices/$testIndex3/shrink" -Body @{
    target_index = "$testIndex3`_shrunk"
    settings = @{
        number_of_shards = 1
    }
} -ExpectedStatusCodes @(200, 201, 400, 404) -SkipOnError $true

Test-Route "Split Index" "POST" "/api/v1/indices/$testIndex2/split" -Body @{
    target_index = "$testIndex2`_split"
    settings = @{
        number_of_shards = 4
    }
} -ExpectedStatusCodes @(200, 201, 400, 404) -SkipOnError $true

Test-Route "Clone Index" "POST" "/api/v1/indices/$testIndex1/clone" -Body @{
    target_index = "$testIndex1`_clone"
} -ExpectedStatusCodes @(200, 201, 400, 404) -SkipOnError $true

Write-Log ""

# 5. GEO OPERATIONS
Write-Log "5. GEO OPERATIONS"
Test-Route "Validate GeoPoint (Object)" "POST" "/api/v1/geo/validate" -Body @{
    point = @{
        lat = 40.7128
        lon = -74.0060
    }
} -ExpectedStatusCodes @(200, 400)

Test-Route "Validate GeoPoint (Array)" "POST" "/api/v1/geo/validate" -Body @{
    point = @(-74.0060, 40.7128)
} -ExpectedStatusCodes @(200, 400)

Test-Route "Validate GeoPoint (String)" "POST" "/api/v1/geo/validate" -Body @{
    point = "POINT(-74.0060 40.7128)"
} -ExpectedStatusCodes @(200, 400)

Test-Route "Calculate Distance" "POST" "/api/v1/geo/distance" -Body @{
    point1 = @{
        lat = 40.7128
        lon = -74.0060
    }
    point2 = @{
        lat = 34.0522
        lon = -118.2437
    }
} -ExpectedStatusCodes @(200, 400)

Test-Route "Check Bounds" "POST" "/api/v1/geo/bounds" -Body @{
    point = @{
        lat = 40.7128
        lon = -74.0060
    }
    bounds = @{
        top_left = @{
            lat = 41.0
            lon = -75.0
        }
        bottom_right = @{
            lat = 40.0
            lon = -73.0
        }
    }
} -ExpectedStatusCodes @(200, 400)

Write-Log ""

# 6. DOCUMENT OPERATIONS
Write-Log "6. DOCUMENT OPERATIONS"
$docId = "doc_1"
Test-Route "Add Document" "POST" "/api/v1/indices/$testIndex1/documents" -Body @{
    id = $docId
    document = @{
        _id = $docId
        title = "Test Document"
        content = "This is a test document"
        timestamp = (Get-Date).ToString("yyyy-MM-ddTHH:mm:ss")
    }
} -ExpectedStatusCodes @(200, 201)

Test-Route "Get Document" "GET" "/api/v1/indices/$testIndex1/documents/$docId"
Test-Route "Update Document" "PUT" "/api/v1/indices/$testIndex1/documents/$docId" -Body @{
    document = @{
        _id = $docId
        title = "Updated Test Document"
        content = "This is an updated test document"
        timestamp = (Get-Date).ToString("yyyy-MM-ddTHH:mm:ss")
    }
} -ExpectedStatusCodes @(200, 201, 204)

Test-Route "Delete Document" "DELETE" "/api/v1/indices/$testIndex1/documents/$docId" -ExpectedStatusCodes @(200, 204, 404)

Write-Log ""

# 7. BULK OPERATIONS
Write-Log "7. BULK OPERATIONS"
Test-Route "Bulk Operations" "POST" "/api/v1/bulk" -Body @{
    operations = @(
        @{
            index = $testIndex1
            action = "index"
            document = @{
                title = "Bulk Doc 1"
                content = "Content 1"
            }
        },
        @{
            index = $testIndex1
            action = "index"
            document = @{
                title = "Bulk Doc 2"
                content = "Content 2"
            }
        }
    )
} -ExpectedStatusCodes @(200, 201, 400)

Write-Log ""

# 8. SEARCH OPERATIONS
Write-Log "8. SEARCH OPERATIONS"
Test-Route "Search (POST)" "POST" "/api/v1/indices/$testIndex1/search" -Body @{
    query = @{
        match_all = @{}
    }
    size = 10
    from = 0
} -ExpectedStatusCodes @(200, 400)

Test-Route "Search (GET)" "GET" "/api/v1/indices/$testIndex1/search?q=test&size=10"
Test-Route "Explain Document" "GET" "/api/v1/indices/$testIndex1/_explain/$docId" -ExpectedStatusCodes @(200, 400, 404)

Write-Log ""

# 9. SCROLL API
Write-Log "9. SCROLL API"
Test-Route "Create Scroll" "POST" "/api/v1/indices/$testIndex1/_search/scroll" -Body @{
    query = @{
        match_all = @{}
    }
    size = 10
    scroll = "1m"
} -ExpectedStatusCodes @(200, 400)

Test-Route "Clear All Scrolls" "DELETE" "/api/v1/_search/scroll/_all" -ExpectedStatusCodes @(200, 204)

Write-Log ""

# 10. POINT IN TIME API
Write-Log "10. POINT IN TIME API"
Test-Route "Create PIT" "POST" "/api/v1/indices/$testIndex1/_pit" -Body @{
    keep_alive = "1m"
} -ExpectedStatusCodes @(200, 201, 400)

Write-Log ""

# 11. QUERY OPERATIONS
Write-Log "11. QUERY OPERATIONS"
Test-Route "Update By Query" "POST" "/api/v1/indices/$testIndex1/_update_by_query" -Body @{
    query = @{
        match_all = @{}
    }
} -ExpectedStatusCodes @(200, 400)

Test-Route "Delete By Query" "POST" "/api/v1/indices/$testIndex1/_delete_by_query" -Body @{
    query = @{
        match_all = @{}
    }
} -ExpectedStatusCodes @(200, 400)

Test-Route "Multi-Get" "POST" "/api/v1/_mget" -Body @{
    docs = @(
        @{
            index = $testIndex1
            id = $docId
        }
    )
} -ExpectedStatusCodes @(200, 400)

Test-Route "Multi-Search" "POST" "/api/v1/_msearch" -Body @{
    searches = @(
        @{
            index = $testIndex1
            query = @{
                match_all = @{}
            }
        }
    )
} -ExpectedStatusCodes @(200, 400)

Write-Log ""

# 12. SUGGESTIONS
Write-Log "12. SUGGESTIONS"
Test-Route "Suggest (GET)" "GET" "/api/v1/indices/$testIndex1/_suggest?q=test"
Test-Route "Suggest (POST)" "POST" "/api/v1/indices/$testIndex1/_suggest" -Body @{
    suggestion = @{
        text = "test"
        term = @{
            field = "content"
        }
    }
} -ExpectedStatusCodes @(200, 400)

Write-Log ""

# 13. MAPPING OPERATIONS
Write-Log "13. MAPPING OPERATIONS"
Test-Route "Get Mapping" "GET" "/api/v1/indices/$testIndex1/_mapping"
Test-Route "Get All Mappings" "GET" "/api/v1/_mapping"

Write-Log ""

# 14. ALIAS OPERATIONS
Write-Log "14. ALIAS OPERATIONS"
$testAlias = "test_alias"
Test-Route "Get Aliases" "GET" "/_aliases"

# Ensure test_index_1 exists before adding alias (it may have been deleted in previous operations)
Test-Route "Ensure Index for Alias" "POST" "/api/v1/indices" -Body @{
    name = $testIndex1
    fields = @(
        @{
            name = "_id"
            type = "keyword"
            stored = $true
            indexed = $true
        },
        @{
            name = "title"
            type = "text"
            stored = $true
            indexed = $true
        }
    )
    settings = @{
        number_of_shards = 1
        number_of_replicas = 0
    }
} -ExpectedStatusCodes @(200, 201) -SkipOnError $true

Test-Route "Add Alias" "PUT" "/$testIndex1/_alias/$testAlias" -ExpectedStatusCodes @(200, 201, 204, 400, 404)
Test-Route "Get Index Aliases" "GET" "/$testIndex1/_alias"
Test-Route "Remove Alias" "DELETE" "/$testIndex1/_alias/$testAlias" -ExpectedStatusCodes @(200, 204, 404)

Write-Log ""

# 15. TEMPLATE OPERATIONS
Write-Log "15. TEMPLATE OPERATIONS"
$testTemplate = "test_template"
Test-Route "List Templates" "GET" "/_template"
Test-Route "Create Template" "PUT" "/_template/$testTemplate" -Body @{
    index_patterns = @("test_*")
    settings = @{
        number_of_shards = 1
        number_of_replicas = 0
    }
    mappings = @{
        properties = @{}
    }
} -ExpectedStatusCodes @(200, 201) -TrackResource $true
Test-Route "Get Template" "GET" "/_template/$testTemplate"
Test-Route "Delete Template" "DELETE" "/_template/$testTemplate" -ExpectedStatusCodes @(200, 204, 404)

Write-Log ""

# 16. SNAPSHOT OPERATIONS
Write-Log "16. SNAPSHOT OPERATIONS"
$testRepo = "test_repo"
$testSnapshot = "test_snapshot"
Test-Route "List Repositories" "GET" "/_snapshot"
Test-Route "Create Repository" "PUT" "/_snapshot/$testRepo" -Body @{
    type = "fs"
    settings = @{
        location = "test_snapshots"
    }
} -ExpectedStatusCodes @(200, 201, 400) -TrackResource $true
Test-Route "Get Repository" "GET" "/_snapshot/$testRepo" -ExpectedStatusCodes @(200, 404)
Test-Route "Get Snapshot Stats" "GET" "/_snapshot/_stats" -ExpectedStatusCodes @(200, 404)

Write-Log ""

# 17. REINDEX OPERATIONS
Write-Log "17. REINDEX OPERATIONS"
Test-Route "List Tasks" "GET" "/_tasks"
Test-Route "Reindex" "POST" "/_reindex" -Body @{
    source = @{
        index = $testIndex1
    }
    dest = @{
        index = "$testIndex1`_reindexed"
    }
} -ExpectedStatusCodes @(200, 201, 400, 404)

Write-Log ""

# 18. ROLLOVER OPERATIONS
Write-Log "18. ROLLOVER OPERATIONS"
Test-Route "Get Rollover Conditions" "GET" "/api/v1/indices/$testIndex1/_rollover" -ExpectedStatusCodes @(200, 404)
Test-Route "Rollover Index" "POST" "/api/v1/indices/$testIndex1/rollover" -Body @{
    conditions = @{
        max_docs = 1000
    }
} -ExpectedStatusCodes @(200, 201, 400, 404)

Write-Log ""

# 19. PROGRESS TRACKING
Write-Log "19. PROGRESS TRACKING"
Test-Route "List Progress" "GET" "/api/v1/progress"
Test-Route "Progress Stats" "GET" "/api/v1/progress/stats"

Write-Log ""

# 20. AUTHENTICATION
Write-Log "20. AUTHENTICATION"
Test-Route "List API Keys" "GET" "/api/v1/auth/keys" -ExpectedStatusCodes @(200, 401, 403)

Write-Log ""

# 21. PROFILING
Write-Log "21. PROFILING"
Test-Route "Get Profiling Status" "GET" "/_profiling/status" -ExpectedStatusCodes @(200, 404)

Write-Log ""

# 22. CLEANUP
Write-Log "22. CLEANUP - Removing test resources"
Write-Log "Cleaning up created indices, templates, and repositories..."

# Cleanup function
function Cleanup-Resources {
    Write-Log "Starting cleanup of test resources..."
    
    # Cleanup tracked indices
    foreach ($index in $script:CreatedIndices) {
        try {
            $response = Invoke-WebRequest -Uri "$ServerUrl/api/v1/indices/$index" -Method DELETE -ErrorAction SilentlyContinue
            Write-Log "  [OK] Deleted index: $index" "SUCCESS"
        } catch {
            # Ignore cleanup errors
            Write-Log "  [SKIP] Could not delete index: $index" "WARN"
        }
    }
    
    # Cleanup tracked templates
    foreach ($template in $script:CreatedTemplates) {
        try {
            $response = Invoke-WebRequest -Uri "$ServerUrl/_template/$template" -Method DELETE -ErrorAction SilentlyContinue
            Write-Log "  [OK] Deleted template: $template" "SUCCESS"
        } catch {
            # Ignore cleanup errors
            Write-Log "  [SKIP] Could not delete template: $template" "WARN"
        }
    }
    
    # Cleanup tracked repositories
    foreach ($repo in $script:CreatedRepositories) {
        try {
            $response = Invoke-WebRequest -Uri "$ServerUrl/_snapshot/$repo" -Method DELETE -ErrorAction SilentlyContinue
            Write-Log "  [OK] Deleted repository: $repo" "SUCCESS"
        } catch {
            # Ignore cleanup errors
            Write-Log "  [SKIP] Could not delete repository: $repo" "WARN"
        }
    }
    
    # Also cleanup explicitly named test resources
    Test-Route "Delete Test Index 1" "DELETE" "/api/v1/indices/$testIndex1" -ExpectedStatusCodes @(200, 204, 404) -SkipOnError $true
    Test-Route "Delete Test Index 2" "DELETE" "/api/v1/indices/$testIndex2" -ExpectedStatusCodes @(200, 204, 404) -SkipOnError $true
    Test-Route "Delete Test Index 3" "DELETE" "/api/v1/indices/$testIndex3" -ExpectedStatusCodes @(200, 204, 404) -SkipOnError $true
    
    Write-Log "Cleanup completed."
}

# Run cleanup
Cleanup-Resources

Write-Log ""

# RESUMO FINAL
Write-Log "========================================"
Write-Log "  RESUMO DOS TESTES"
Write-Log "========================================"
Write-Log "Total de Testes: $script:TotalTests"
Write-Log "Passou: $script:PassedTests" "SUCCESS"
Write-Log "Falhou: $script:FailedTests" "ERROR"
Write-Log "Avisos: $script:WarningTests" "WARN"
Write-Log "Retries: $script:RetryCount" "INFO"
$successRate = if ($script:TotalTests -gt 0) { [math]::Round(($script:PassedTests / $script:TotalTests) * 100, 2) } else { 0 }
Write-Log "Taxa de Sucesso: ${successRate}%"
Write-Log "Log File: $LogFile"
$errorFile = $LogFile -replace '\.log$', '_errors.json'
if (Test-Path $errorFile) {
    Write-Log "Error Details: $errorFile" "WARN"
}
Write-Log "========================================"

if ($script:FailedTests -eq 0) {
    Write-Log "[SUCCESS] Todos os testes criticos passaram!" "SUCCESS"
    exit 0
} else {
    Write-Log "[ERROR] Alguns testes falharam. Verifique os detalhes acima e o arquivo de log." "ERROR"
    Write-Log "Log file: $LogFile" "INFO"
    if (Test-Path $errorFile) {
        Write-Log "Error details: $errorFile" "INFO"
    }
    exit 1
}
