# Script para gerar relatório completo de testes das rotas
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
        if ($_.Exception.Response) {
            $result.StatusCode = $_.Exception.Response.StatusCode.value__
            try {
                $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
                $result.Response = $reader.ReadToEnd()
            } catch {}
        }
        $result.Error = $_.Exception.Message
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

# Categorias de rotas
$categories = @{
    "Health & Cluster" = @()
    "Index Management" = @()
    "Document Operations" = @()
    "Bulk & Batch" = @()
    "Search" = @()
    "Snapshots" = @()
    "Templates" = @()
    "Aliases" = @()
    "Authentication" = @()
    "Progress & Tasks" = @()
}

# 1. Health Check
Write-Host "[1/13] Testando Health Check..." -ForegroundColor White
$testResults += Test-Route -Method "GET" -Path "/health"
$categories["Health & Cluster"] += $testResults[-1]

# 2. Cluster Endpoints
Write-Host "[2/13] Testando Cluster Endpoints..." -ForegroundColor White
$testResults += Test-Route -Method "GET" -Path "/"
$categories["Health & Cluster"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_cluster/health"
$categories["Health & Cluster"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_cluster/stats"
$categories["Health & Cluster"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_cluster/state"
$categories["Health & Cluster"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_nodes/stats"
$categories["Health & Cluster"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_cluster/settings"
$categories["Health & Cluster"] += $testResults[-1]
$testResults += Test-Route -Method "PUT" -Path "/_cluster/settings" -Body '{"persistent":{}}'
$categories["Health & Cluster"] += $testResults[-1]

# 3. Index Management
Write-Host "[3/13] Testando Index Management..." -ForegroundColor White
$indexName = "test-index-$(Get-Random)"
$indexBody = @{
    name = $indexName
    fields = @(
        @{
            name = "_id"
            type = "text"
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
$categories["Index Management"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/api/v1/indices"
$categories["Index Management"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/api/v1/indices/$indexName"
$categories["Index Management"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/api/v1/indices/$indexName/stats"
$categories["Index Management"] += $testResults[-1]
$testResults += Test-Route -Method "POST" -Path "/api/v1/indices/$indexName/refresh"
$categories["Index Management"] += $testResults[-1]
$testResults += Test-Route -Method "POST" -Path "/api/v1/indices/$indexName/flush"
$categories["Index Management"] += $testResults[-1]

# 4. Document Operations
Write-Host "[4/13] Testando Document Operations..." -ForegroundColor White
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
$categories["Document Operations"] += $testResults[-1]

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
Start-Sleep -Milliseconds 200

$testResults += Test-Route -Method "GET" -Path "/api/v1/indices/$indexName/documents/$docId"
$categories["Document Operations"] += $testResults[-1]
$testResults += Test-Route -Method "PUT" -Path "/api/v1/indices/$indexName/documents/$docId" -Body (@{document=@{_id=$docId; title="Updated"; price=200}} | ConvertTo-Json -Depth 10 -Compress)
$categories["Document Operations"] += $testResults[-1]
$testResults += Test-Route -Method "DELETE" -Path "/api/v1/indices/$indexName/documents/$docId"
$categories["Document Operations"] += $testResults[-1]

# 5. Bulk Operations
Write-Host "[5/13] Testando Bulk Operations..." -ForegroundColor White
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
$categories["Bulk & Batch"] += $testResults[-1]

# 6. Batch Requests
Write-Host "[6/13] Testando Batch Requests..." -ForegroundColor White
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
$categories["Bulk & Batch"] += $testResults[-1]

# 7. Search
Write-Host "[7/13] Testando Search..." -ForegroundColor White
$searchBody = @{
    query = @{
        match_all = $null
    }
    limit = 10
} | ConvertTo-Json -Depth 10 -Compress

$testResults += Test-Route -Method "POST" -Path "/api/v1/indices/$indexName/search" -Body $searchBody
$categories["Search"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/api/v1/indices/$indexName/search?q=test"
$categories["Search"] += $testResults[-1]

# 8. Snapshot Repositories
Write-Host "[8/13] Testando Snapshot Repositories..." -ForegroundColor White
$repoName = "test-repo"
$repoBody = @{
    type = "fs"
    settings = @{
        location = "./snapshots"
    }
} | ConvertTo-Json -Depth 10 -Compress

$testResults += Test-Route -Method "PUT" -Path "/_snapshot/$repoName" -Body $repoBody
$categories["Snapshots"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_snapshot/$repoName"
$categories["Snapshots"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_snapshot"
$categories["Snapshots"] += $testResults[-1]

# 9. Templates
Write-Host "[9/13] Testando Templates..." -ForegroundColor White
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
                type = "text"
            }
        }
    }
} | ConvertTo-Json -Depth 10 -Compress

$testResults += Test-Route -Method "PUT" -Path "/_template/$templateName" -Body $templateBody
$categories["Templates"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_template/$templateName"
$categories["Templates"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_template"
$categories["Templates"] += $testResults[-1]
$testResults += Test-Route -Method "DELETE" -Path "/_template/$templateName"
$categories["Templates"] += $testResults[-1]

# 10. Aliases
Write-Host "[10/13] Testando Aliases..." -ForegroundColor White
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
$categories["Aliases"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_aliases"
$categories["Aliases"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/$indexName/_alias"
$categories["Aliases"] += $testResults[-1]
$testResults += Test-Route -Method "DELETE" -Path "/$indexName/_alias/$aliasName"
$categories["Aliases"] += $testResults[-1]

# 11. Authentication
Write-Host "[11/13] Testando Authentication..." -ForegroundColor White
$testResults += Test-Route -Method "GET" -Path "/api/v1/auth/keys"
$categories["Authentication"] += $testResults[-1]
$testResults += Test-Route -Method "POST" -Path "/api/v1/auth/keys" -Body (@{name="test-key"} | ConvertTo-Json -Compress)
$categories["Authentication"] += $testResults[-1]

# 12. Progress Tracking
Write-Host "[12/13] Testando Progress Tracking..." -ForegroundColor White
$testResults += Test-Route -Method "GET" -Path "/api/v1/progress"
$categories["Progress & Tasks"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/api/v1/progress/stats"
$categories["Progress & Tasks"] += $testResults[-1]

# 13. Reindex
Write-Host "[13/13] Testando Reindex..." -ForegroundColor White
$testResults += Test-Route -Method "GET" -Path "/_tasks"
$categories["Progress & Tasks"] += $testResults[-1]
$testResults += Test-Route -Method "GET" -Path "/_tasks/nonexistent"
$categories["Progress & Tasks"] += $testResults[-1]

# Limpar recursos
Write-Host "`nLimpando recursos..." -ForegroundColor Yellow
Test-Route -Method "DELETE" -Path "/api/v1/indices/$indexName" | Out-Null

# Gerar relatório completo
Write-Host "`n`n" -NoNewline
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "           RELATÓRIO COMPLETO DE TESTES DE ROTAS API          " -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

$successCount = ($testResults | Where-Object { $_.Status -eq "SUCCESS" }).Count
$failedCount = ($testResults | Where-Object { $_.Status -eq "FAILED" }).Count
$totalCount = $testResults.Count
$successRate = [math]::Round(($successCount / $totalCount) * 100, 2)
$avgResponseTime = ($testResults | Where-Object { $_.Status -eq "SUCCESS" } | Measure-Object -Property ResponseTime -Average).Average

Write-Host "📊 ESTATÍSTICAS GERAIS" -ForegroundColor Yellow
Write-Host "───────────────────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host "Total de rotas testadas:     $totalCount" -ForegroundColor White
Write-Host "Rotas com sucesso:          " -NoNewline -ForegroundColor White
Write-Host "$successCount" -ForegroundColor Green -NoNewline
Write-Host " ($successRate%)" -ForegroundColor $(if ($successRate -ge 80) { "Green" } elseif ($successRate -ge 60) { "Yellow" } else { "Red" })
Write-Host "Rotas com falha:            " -NoNewline -ForegroundColor White
Write-Host "$failedCount" -ForegroundColor $(if ($failedCount -eq 0) { "Green" } else { "Red" })
Write-Host "Tempo médio de resposta:    $([math]::Round($avgResponseTime, 2))ms" -ForegroundColor White
Write-Host ""

# Relatório por categoria
Write-Host "📋 RELATÓRIO POR CATEGORIA" -ForegroundColor Yellow
Write-Host "───────────────────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

foreach ($category in $categories.Keys | Sort-Object) {
    $categoryResults = $categories[$category]
    if ($categoryResults.Count -eq 0) { continue }
    
    $catSuccess = ($categoryResults | Where-Object { $_.Status -eq "SUCCESS" }).Count
    $catFailed = ($categoryResults | Where-Object { $_.Status -eq "FAILED" }).Count
    $catRate = [math]::Round(($catSuccess / $categoryResults.Count) * 100, 1)
    
    Write-Host "  $category" -ForegroundColor Cyan
    Write-Host "    Total: $($categoryResults.Count) | " -NoNewline -ForegroundColor White
    Write-Host "✓ $catSuccess" -ForegroundColor Green -NoNewline
    Write-Host " | " -NoNewline -ForegroundColor White
    Write-Host "✗ $catFailed" -ForegroundColor $(if ($catFailed -eq 0) { "Green" } else { "Red" }) -NoNewline
    Write-Host " | Taxa: $catRate%" -ForegroundColor $(if ($catRate -ge 80) { "Green" } elseif ($catRate -ge 60) { "Yellow" } else { "Red" })
    Write-Host ""
}

# Detalhes das rotas
Write-Host "📝 DETALHES DAS ROTAS" -ForegroundColor Yellow
Write-Host "───────────────────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

foreach ($category in $categories.Keys | Sort-Object) {
    $categoryResults = $categories[$category]
    if ($categoryResults.Count -eq 0) { continue }
    
    Write-Host "  [$category]" -ForegroundColor Cyan
    foreach ($result in $categoryResults) {
        $statusIcon = if ($result.Status -eq "SUCCESS") { "✓" } else { "✗" }
        $color = if ($result.Status -eq "SUCCESS") { "Green" } else { "Red" }
        $statusText = if ($result.StatusCode -eq 0) { "N/A" } else { "$($result.StatusCode)" }
        Write-Host "    $statusIcon " -NoNewline -ForegroundColor $color
        Write-Host "$($result.Method.PadRight(6)) " -NoNewline -ForegroundColor White
        Write-Host "$($result.Path.PadRight(50)) " -NoNewline -ForegroundColor White
        Write-Host "Status: $statusText " -NoNewline -ForegroundColor $(if ($result.Status -eq "SUCCESS") { "Green" } else { "Red" })
        Write-Host "($($result.ResponseTime)ms)" -ForegroundColor Gray
    }
    Write-Host ""
}

# Rotas que falharam (detalhado)
$failedRoutes = $testResults | Where-Object { $_.Status -eq "FAILED" }
if ($failedRoutes.Count -gt 0) {
    Write-Host "❌ ROTAS QUE FALHARAM (DETALHADO)" -ForegroundColor Red
    Write-Host "───────────────────────────────────────────────────────────────" -ForegroundColor Gray
    Write-Host ""
    
    foreach ($route in $failedRoutes) {
        Write-Host "  ✗ $($route.Method) $($route.Path)" -ForegroundColor Red
        Write-Host "    Status Code: $($route.StatusCode)" -ForegroundColor Yellow
        if ($route.Error) {
            Write-Host "    Erro: $($route.Error)" -ForegroundColor Yellow
        }
        if ($route.Response -and $route.Response.Length -gt 0) {
            $responsePreview = if ($route.Response.Length -gt 150) { $route.Response.Substring(0, 150) + "..." } else { $route.Response }
            Write-Host "    Resposta: $responsePreview" -ForegroundColor Gray
        }
        Write-Host ""
    }
}

# Resumo final
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "                         RESUMO FINAL                          " -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

$overallStatus = if ($successRate -ge 80) { "✅ EXCELENTE" } elseif ($successRate -ge 60) { "⚠️  BOM" } else { "❌ NECESSITA ATENÇÃO" }
Write-Host "Status Geral: $overallStatus" -ForegroundColor $(if ($successRate -ge 80) { "Green" } elseif ($successRate -ge 60) { "Yellow" } else { "Red" })
Write-Host "Taxa de Sucesso: $successRate%" -ForegroundColor White
Write-Host "Rotas Funcionais: $successCount/$totalCount" -ForegroundColor White
Write-Host ""

# Parar servidor
Write-Host "Parando servidor..." -ForegroundColor Yellow
Stop-Process -Id $serverProcess.Id -Force -ErrorAction SilentlyContinue
Write-Host "Servidor parado.`n" -ForegroundColor Green

# Retornar código de saída
if ($failedCount -gt 0) {
    exit 1
} else {
    exit 0
}

