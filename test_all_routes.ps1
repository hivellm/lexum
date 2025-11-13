# Script para testar todas as rotas da API Lexum
$ErrorActionPreference = "Stop"
$baseUrl = "http://127.0.0.1:17000"
$testResults = @()

# Função para fazer requisições HTTP
function Test-Route {
    param(
        [string]$Method,
        [string]$Path,
        [string]$Body = $null,
        [hashtable]$Headers = @{}
    )
    
    $url = "$baseUrl$Path"
    $result = @{
        Method = $Method
        Path = $Path
        Status = "FAILED"
        StatusCode = 0
        ResponseTime = 0
        Error = ""
        Response = ""
    }
    
    try {
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        
        $params = @{
            Method = $Method
            Uri = $url
            Headers = $Headers
            TimeoutSec = 10
        }
        
        if ($Body) {
            $params.Body = $Body
            $params.ContentType = "application/json"
        }
        
        $response = Invoke-WebRequest @params -UseBasicParsing -ErrorAction Stop
        $stopwatch.Stop()
        
        $result.Status = "SUCCESS"
        $result.StatusCode = $response.StatusCode
        $result.ResponseTime = $stopwatch.ElapsedMilliseconds
        
        # Handle both text and binary responses
        if ($response.Content -is [string]) {
            $result.Response = $response.Content.Substring(0, [Math]::Min(200, $response.Content.Length))
        } else {
            $result.Response = "[Binary response - $($response.Content.Length) bytes]"
        }
    }
    catch {
        $result.Status = "FAILED"
        $result.StatusCode = $_.Exception.Response.StatusCode.value__
        $result.Error = $_.Exception.Message
        if ($_.Exception.Response) {
            try {
                $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
                $result.Response = $reader.ReadToEnd()
            } catch {}
        }
    }
    
    return $result
}

# Função para aguardar servidor estar pronto
function Wait-ForServer {
    Write-Host "Aguardando servidor iniciar..." -ForegroundColor Yellow
    $maxAttempts = 30
    $attempt = 0
    
    while ($attempt -lt $maxAttempts) {
        try {
            $response = Invoke-WebRequest -Uri "$baseUrl/health" -Method GET -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
            if ($response.StatusCode -eq 200) {
                Write-Host "Servidor está pronto!" -ForegroundColor Green
                Start-Sleep -Seconds 2
                return $true
            }
        }
        catch {
            $attempt++
            Start-Sleep -Seconds 1
        }
    }
    
    Write-Host "Servidor não respondeu após $maxAttempts tentativas" -ForegroundColor Red
    return $false
}

# Iniciar servidor em background
Write-Host "`n=== INICIANDO SERVIDOR ===" -ForegroundColor Cyan
$serverProcess = Start-Process -FilePath "cargo" -ArgumentList "run", "--bin", "lexum-server" -PassThru -NoNewWindow -RedirectStandardOutput "server_output.log" -RedirectStandardError "server_error.log"
Write-Host "Servidor iniciado com PID: $($serverProcess.Id)" -ForegroundColor Green

# Aguardar servidor estar pronto
if (-not (Wait-ForServer)) {
    Write-Host "Falha ao iniciar servidor. Verifique os logs." -ForegroundColor Red
    Stop-Process -Id $serverProcess.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

Write-Host "`n=== INICIANDO TESTES DE ROTAS ===" -ForegroundColor Cyan
Write-Host "Base URL: $baseUrl`n" -ForegroundColor Yellow

# 1. Health Check
Write-Host "[1/10] Testando Health Check..." -ForegroundColor White
$testResults += Test-Route -Method "GET" -Path "/health"

# 2. Cluster Endpoints
Write-Host "[2/10] Testando Cluster Endpoints..." -ForegroundColor White
$testResults += Test-Route -Method "GET" -Path "/"
$testResults += Test-Route -Method "GET" -Path "/_cluster/health"
$testResults += Test-Route -Method "GET" -Path "/_cluster/stats"
$testResults += Test-Route -Method "GET" -Path "/_cluster/state"
$testResults += Test-Route -Method "GET" -Path "/_nodes/stats"
$testResults += Test-Route -Method "GET" -Path "/_cluster/settings"
$clusterSettingsBody = '{"settings":{"cluster_name":"test-cluster","persistence":{"storage_path":"./data","snapshot":{"repository_path":"./snapshots","max_snapshots":10}},"network":{"bind_address":"0.0.0.0","port":17000,"enable_cors":true}}}'
$testResults += Test-Route -Method "PUT" -Path "/_cluster/settings" -Body $clusterSettingsBody

# 3. Index Management
Write-Host "[3/10] Testando Index Management..." -ForegroundColor White
$indexName = "test-index-$(Get-Random)"
$indexBody = @{
    name = $indexName
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
            name = "price"
            type = "i64"
            stored = $true
            indexed = $true
        }
    )
} | ConvertTo-Json -Depth 10 -Compress

$testResults += Test-Route -Method "POST" -Path "/api/v1/indices" -Body $indexBody
$testResults += Test-Route -Method "GET" -Path "/api/v1/indices"
$testResults += Test-Route -Method "GET" -Path "/api/v1/indices/$indexName"
$testResults += Test-Route -Method "GET" -Path "/api/v1/indices/$indexName/stats"
$testResults += Test-Route -Method "POST" -Path "/api/v1/indices/$indexName/refresh"
$testResults += Test-Route -Method "POST" -Path "/api/v1/indices/$indexName/flush"

# 4. Document Operations
Write-Host "[4/10] Testando Document Operations..." -ForegroundColor White
$docId = "doc-1"
$docBody = @{
    document = @{
        _id = $docId
        title = "Test Document"
        price = 100
    }
} | ConvertTo-Json -Depth 10 -Compress

$docResult = Test-Route -Method "POST" -Path "/api/v1/indices/$indexName/documents" -Body $docBody
$testResults += $docResult

# Extract document ID from response if not using custom ID
if ($docResult.Status -eq "SUCCESS" -and $docResult.Response) {
    try {
        $responseJson = $docResult.Response | ConvertFrom-Json
        if ($responseJson.id -and $responseJson.id -ne $docId) {
            $docId = $responseJson.id
        }
    } catch {
        # Use default ID if parsing fails
    }
}

# Refresh index to make document searchable
Test-Route -Method "POST" -Path "/api/v1/indices/$indexName/refresh" | Out-Null
Start-Sleep -Milliseconds 300

$testResults += Test-Route -Method "GET" -Path "/api/v1/indices/$indexName/documents/$docId"
$testResults += Test-Route -Method "PUT" -Path "/api/v1/indices/$indexName/documents/$docId" -Body (@{document=@{_id=$docId; title="Updated"; price=200}} | ConvertTo-Json -Depth 10 -Compress)
$testResults += Test-Route -Method "DELETE" -Path "/api/v1/indices/$indexName/documents/$docId"

# 5. Bulk Operations
Write-Host "[5/10] Testando Bulk Operations..." -ForegroundColor White
$bulkBody = @{
    operations = @(
        @{
            action = "index"
            _index = $indexName
            document = @{_id="bulk-1"; title="Bulk Doc 1"; price=10}
        },
        @{
            action = "index"
            _index = $indexName
            document = @{_id="bulk-2"; title="Bulk Doc 2"; price=20}
        }
    )
} | ConvertTo-Json -Depth 10 -Compress

$testResults += Test-Route -Method "POST" -Path "/api/v1/bulk" -Body $bulkBody

# 6. Batch Requests
Write-Host "[6/10] Testando Batch Requests..." -ForegroundColor White
$batchBody = @{
    requests = @(
        @{
            method = "GET"
            path = "/api/v1/indices"
        },
        @{
            method = "GET"
            path = "/health"
        }
    )
} | ConvertTo-Json -Depth 10

$testResults += Test-Route -Method "POST" -Path "/api/v1/_batch" -Body $batchBody

# 7. Search
Write-Host "[7/10] Testando Search..." -ForegroundColor White
$searchBody = '{"query":{"match_all":null},"limit":10}'

$testResults += Test-Route -Method "POST" -Path "/api/v1/indices/$indexName/search" -Body $searchBody
$testResults += Test-Route -Method "GET" -Path "/api/v1/indices/$indexName/search?q=test"

# 8. Snapshot Repositories
Write-Host "[8/10] Testando Snapshot Repositories..." -ForegroundColor White
$repoName = "test-repo"
$repoBody = @{
    type = "fs"
    settings = @{
        location = "./snapshots"
    }
} | ConvertTo-Json -Depth 10

$testResults += Test-Route -Method "PUT" -Path "/_snapshot/$repoName" -Body $repoBody
$testResults += Test-Route -Method "GET" -Path "/_snapshot/$repoName"
$testResults += Test-Route -Method "GET" -Path "/_snapshot"

# 9. Templates
Write-Host "[9/10] Testando Templates..." -ForegroundColor White
$templateName = "test-template"
$templateBody = @{
    index_patterns = @("test-*")
    priority = 0
    version = 1
    order = 0
    settings = @{
        number_of_shards = 1
        number_of_replicas = 0
        refresh_interval = 1
        custom = @{}
    }
    mappings = @{
        properties = @{
            title = @{
                name = "title"
                type = "text"
                stored = $true
                indexed = $true
            }
        }
    }
} | ConvertTo-Json -Depth 10 -Compress

$testResults += Test-Route -Method "PUT" -Path "/_template/$templateName" -Body $templateBody
$testResults += Test-Route -Method "GET" -Path "/_template/$templateName"
$testResults += Test-Route -Method "GET" -Path "/_template"
$testResults += Test-Route -Method "DELETE" -Path "/_template/$templateName"

# 10. Aliases
Write-Host "[10/10] Testando Aliases..." -ForegroundColor White
$aliasName = "test-alias"
$aliasBody = @{
    actions = @(
        @{
            action = "add"
            index = $indexName
            alias = $aliasName
        }
    )
} | ConvertTo-Json -Depth 10 -Compress

$testResults += Test-Route -Method "POST" -Path "/_aliases" -Body $aliasBody
$testResults += Test-Route -Method "GET" -Path "/_aliases"
$testResults += Test-Route -Method "GET" -Path "/$indexName/_alias"
$testResults += Test-Route -Method "DELETE" -Path "/$indexName/_alias/$aliasName"

# 11. Authentication
Write-Host "[11/11] Testando Authentication..." -ForegroundColor White
$testResults += Test-Route -Method "GET" -Path "/api/v1/auth/keys"
$testResults += Test-Route -Method "POST" -Path "/api/v1/auth/keys" -Body (@{name="test-key"} | ConvertTo-Json -Compress)

# 12. Progress Tracking
Write-Host "[12/12] Testando Progress Tracking..." -ForegroundColor White
$testResults += Test-Route -Method "GET" -Path "/api/v1/progress"
$testResults += Test-Route -Method "GET" -Path "/api/v1/progress/stats"

# 13. Reindex
Write-Host "[13/13] Testando Reindex..." -ForegroundColor White
$testResults += Test-Route -Method "GET" -Path "/_tasks"
$testResults += Test-Route -Method "GET" -Path "/_tasks/nonexistent"

# Limpar recursos
Write-Host "`nLimpando recursos..." -ForegroundColor Yellow
Test-Route -Method "DELETE" -Path "/api/v1/indices/$indexName" | Out-Null

# Gerar relatório
Write-Host "`n`n=== RELATÓRIO DE TESTES ===" -ForegroundColor Cyan
Write-Host "Total de rotas testadas: $($testResults.Count)" -ForegroundColor White

$successCount = ($testResults | Where-Object { $_.Status -eq "SUCCESS" }).Count
$failedCount = ($testResults | Where-Object { $_.Status -eq "FAILED" }).Count
$avgResponseTime = ($testResults | Where-Object { $_.Status -eq "SUCCESS" } | Measure-Object -Property ResponseTime -Average).Average

Write-Host "`nEstatísticas:" -ForegroundColor Yellow
Write-Host "  Sucesso: $successCount" -ForegroundColor Green
Write-Host "  Falhas: $failedCount" -ForegroundColor $(if ($failedCount -gt 0) { "Red" } else { "Green" })
Write-Host "  Taxa de sucesso: $([math]::Round(($successCount / $testResults.Count) * 100, 2))%" -ForegroundColor White
Write-Host "  Tempo médio de resposta: $([math]::Round($avgResponseTime, 2))ms" -ForegroundColor White

Write-Host "`nDetalhes das rotas:" -ForegroundColor Yellow
foreach ($result in $testResults) {
    $color = if ($result.Status -eq "SUCCESS") { "Green" } else { "Red" }
    $statusIcon = if ($result.Status -eq "SUCCESS") { "✓" } else { "✗" }
    Write-Host "  $statusIcon $($result.Method) $($result.Path) - Status: $($result.StatusCode) ($($result.Status)) - $($result.ResponseTime)ms" -ForegroundColor $color
    if ($result.Status -eq "FAILED" -and $result.Error) {
        Write-Host "    Erro: $($result.Error)" -ForegroundColor Red
    }
}

# Rotas que falharam
$failedRoutes = $testResults | Where-Object { $_.Status -eq "FAILED" }
if ($failedRoutes.Count -gt 0) {
    Write-Host "`nRotas que falharam:" -ForegroundColor Red
    foreach ($route in $failedRoutes) {
        Write-Host "  ✗ $($route.Method) $($route.Path) - Status: $($route.StatusCode)" -ForegroundColor Red
        if ($route.Error) {
            Write-Host "    Erro: $($route.Error)" -ForegroundColor Red
        }
    }
}

# Parar servidor
Write-Host "`nParando servidor..." -ForegroundColor Yellow
Stop-Process -Id $serverProcess.Id -Force -ErrorAction SilentlyContinue
Write-Host "Servidor parado.`n" -ForegroundColor Green

# Retornar código de saída
if ($failedCount -gt 0) {
    exit 1
} else {
    exit 0
}

