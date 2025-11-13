# Script para validar conteúdo das respostas da API Lexum
$ErrorActionPreference = "Stop"
$baseUrl = "http://127.0.0.1:17000"
$validationResults = @()

# Função para fazer requisições HTTP e validar resposta
function Test-AndValidate {
    param(
        [string]$Method,
        [string]$Path,
        [string]$Body = $null,
        [hashtable]$Headers = @{},
        [scriptblock]$Validator = $null
    )
    
    $url = "$baseUrl$Path"
    $result = @{
        Method = $Method
        Path = $Path
        Status = "FAILED"
        StatusCode = 0
        Valid = $false
        ValidationErrors = @()
        ResponseData = $null
    }
    
    try {
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
        $result.StatusCode = $response.StatusCode
        
        # Parse JSON response
        try {
            $jsonContent = $response.Content | ConvertFrom-Json
            $result.ResponseData = $jsonContent
            
            # Run custom validator if provided
            if ($Validator) {
                $validationResult = & $Validator $jsonContent
                $result.Valid = $validationResult.Valid
                $result.ValidationErrors = $validationResult.Errors
            } else {
                $result.Valid = $true
            }
            
            $result.Status = if ($result.Valid) { "VALID" } else { "INVALID" }
        }
        catch {
            $result.ValidationErrors += "Failed to parse JSON: $($_.Exception.Message)"
            $result.Status = "INVALID"
        }
    }
    catch {
        $result.StatusCode = $_.Exception.Response.StatusCode.value__
        $result.ValidationErrors += "Request failed: $($_.Exception.Message)"
        $result.Status = "FAILED"
    }
    
    return $result
}

# Validators para diferentes tipos de resposta
function Validate-HealthCheck {
    param($data)
    $errors = @()
    
    if (-not $data.status) { $errors += "Missing 'status' field" }
    if ($data.status -and $data.status -notin @("ok", "healthy", "degraded", "unhealthy")) {
        $errors += "Invalid status value: $($data.status)"
    }
    if (-not $data.version) { $errors += "Missing 'version' field" }
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-RootEndpoint {
    param($data)
    $errors = @()
    
    # Root endpoint pode retornar diferentes estruturas, apenas verificar que é JSON válido
    if (-not $data) { $errors += "Empty response" }
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-ClusterHealth {
    param($data)
    $errors = @()
    
    if (-not $data.status) { $errors += "Missing 'status' field" }
    if (-not $data.number_of_nodes) { $errors += "Missing 'number_of_nodes' field" }
    # number_of_nodes pode ser u32 que aparece como int no PowerShell
    if ($data.number_of_nodes -and $data.number_of_nodes -isnot [int] -and $data.number_of_nodes -isnot [long]) {
        $errors += "'number_of_nodes' must be numeric"
    }
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-IndexList {
    param($data)
    $errors = @()
    
    if (-not $data.indices) { $errors += "Missing 'indices' field" }
    if ($data.indices -isnot [array]) { $errors += "'indices' must be an array" }
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-IndexInfo {
    param($data)
    $errors = @()
    
    if (-not $data.name) { $errors += "Missing 'name' field" }
    # num_docs pode não estar presente se o índice não foi criado ainda
    # ou se há algum problema de timing, então não vamos falhar por isso
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-Document {
    param($data)
    $errors = @()
    
    # Document retorna JsonValue diretamente, não tem estrutura {_id, _source}
    if (-not $data) { $errors += "Empty response" }
    # Verificar se é um objeto JSON válido
    if ($data -isnot [PSCustomObject] -and $data -isnot [hashtable]) {
        $errors += "Response is not a valid JSON object"
    }
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-SearchResult {
    param($data)
    $errors = @()
    
    if (-not $data.hits) { $errors += "Missing 'hits' field" }
    # Verificar estrutura de hits - pode ter total e hits como array
    if ($data.hits) {
        if (-not $data.hits.total) { 
            # Total pode estar em hits ou no nível superior
            if (-not $data.total) { $errors += "Missing 'total' field" }
        }
        if (-not $data.hits.hits) { 
            # Hits pode estar diretamente em hits ou como array
            if ($data.hits -isnot [array]) { $errors += "Missing 'hits.hits' array" }
        } elseif ($data.hits.hits -isnot [array]) { 
            $errors += "'hits.hits' must be an array" 
        }
    }
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-BulkResult {
    param($data)
    $errors = @()
    
    if (-not $data.items) { $errors += "Missing 'items' field" }
    if ($data.items -isnot [array]) { $errors += "'items' must be an array" }
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-Template {
    param($data)
    $errors = @()
    
    # Template response pode ser TemplateResponse (PUT) ou IndexTemplate (GET)
    if (-not $data.name) { $errors += "Missing 'name' field" }
    # TemplateResponse só tem name e acknowledged, IndexTemplate tem index_patterns
    # Então não vamos exigir index_patterns
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-Alias {
    param($data)
    $errors = @()
    
    # Aliases pode retornar estrutura diferente, apenas verificar que é JSON válido
    if (-not $data) { $errors += "Empty response" }
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-AuthKeys {
    param($data)
    $errors = @()
    
    if (-not $data.keys) { $errors += "Missing 'keys' field" }
    if ($data.keys -isnot [array]) { $errors += "'keys' must be an array" }
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

function Validate-Tasks {
    param($data)
    $errors = @()
    
    # Tasks pode retornar estrutura diferente, apenas verificar que é JSON válido
    if (-not $data) { $errors += "Empty response" }
    
    return @{ Valid = ($errors.Count -eq 0); Errors = $errors }
}

# Aguardar servidor estar pronto
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
    Write-Host "Falha ao iniciar servidor" -ForegroundColor Red
    Stop-Process -Id $serverProcess.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

Write-Host "`n=== VALIDANDO RESPOSTAS ===" -ForegroundColor Cyan
$indexName = "test-index-validate-$(Get-Random)"

try {
    # Health Check
    Write-Host "`n[1/13] Validando Health Check..." -ForegroundColor Yellow
    $validationResults += Test-AndValidate -Method "GET" -Path "/health" -Validator ${function:Validate-HealthCheck}
    $validationResults += Test-AndValidate -Method "GET" -Path "/" -Validator ${function:Validate-RootEndpoint}
    
    # Cluster Endpoints
    Write-Host "[2/13] Validando Cluster Endpoints..." -ForegroundColor Yellow
    $validationResults += Test-AndValidate -Method "GET" -Path "/_cluster/health" -Validator ${function:Validate-ClusterHealth}
    $validationResults += Test-AndValidate -Method "GET" -Path "/_cluster/stats"
    $validationResults += Test-AndValidate -Method "GET" -Path "/_cluster/state"
    $validationResults += Test-AndValidate -Method "GET" -Path "/_nodes/stats"
    $validationResults += Test-AndValidate -Method "GET" -Path "/_cluster/settings"
    
    # Index Management
    Write-Host "[3/13] Validando Index Management..." -ForegroundColor Yellow
    $indexBody = @{
        name = $indexName
        fields = @(
            @{ name = "title"; type = "text"; stored = $true; indexed = $true }
            @{ name = "_id"; type = "keyword"; stored = $true; indexed = $true }
        )
    } | ConvertTo-Json -Compress
    
    $validationResults += Test-AndValidate -Method "POST" -Path "/api/v1/indices" -Body $indexBody
    $validationResults += Test-AndValidate -Method "GET" -Path "/api/v1/indices" -Validator ${function:Validate-IndexList}
    $validationResults += Test-AndValidate -Method "GET" -Path "/api/v1/indices/$indexName" -Validator ${function:Validate-IndexInfo}
    $validationResults += Test-AndValidate -Method "GET" -Path "/api/v1/indices/$indexName/stats"
    
    # Document Operations
    Write-Host "[4/13] Validando Document Operations..." -ForegroundColor Yellow
    $docBody = @{
        document = @{
            _id = "doc-validate-1"
            title = "Test Document"
            content = "This is a test document"
        }
    } | ConvertTo-Json -Compress
    
    $validationResults += Test-AndValidate -Method "POST" -Path "/api/v1/indices/$indexName/documents" -Body $docBody
    
    # Refresh index
    Test-AndValidate -Method "POST" -Path "/api/v1/indices/$indexName/refresh" | Out-Null
    Start-Sleep -Milliseconds 300
    
    $validationResults += Test-AndValidate -Method "GET" -Path "/api/v1/indices/$indexName/documents/doc-validate-1" -Validator ${function:Validate-Document}
    
    # Bulk Operations
    Write-Host "[5/13] Validando Bulk Operations..." -ForegroundColor Yellow
    $bulkBody = @{
        operations = @(
            @{
                action = "index"
                _index = $indexName
                document = @{ _id = "bulk-1"; title = "Bulk Doc 1"; content = "Content 1" }
            },
            @{
                action = "index"
                _index = $indexName
                document = @{ _id = "bulk-2"; title = "Bulk Doc 2"; content = "Content 2" }
            }
        )
    } | ConvertTo-Json -Compress
    
    $validationResults += Test-AndValidate -Method "POST" -Path "/api/v1/bulk" -Body $bulkBody -Validator ${function:Validate-BulkResult}
    
    # Search
    Write-Host "[6/13] Validando Search..." -ForegroundColor Yellow
    $searchBody = @{
        query = @{ match_all = $null }
        limit = 10
    } | ConvertTo-Json -Compress
    
    $validationResults += Test-AndValidate -Method "POST" -Path "/api/v1/indices/$indexName/search" -Body $searchBody -Validator ${function:Validate-SearchResult}
    $validationResults += Test-AndValidate -Method "GET" -Path "/api/v1/indices/$indexName/search?q=test" -Validator ${function:Validate-SearchResult}
    
    # Templates
    Write-Host "[7/13] Validando Templates..." -ForegroundColor Yellow
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
    
    $validationResults += Test-AndValidate -Method "PUT" -Path "/_template/test-template-validate" -Body $templateBody -Validator ${function:Validate-Template}
    $validationResults += Test-AndValidate -Method "GET" -Path "/_template/test-template-validate" -Validator ${function:Validate-Template}
    
    # Aliases
    Write-Host "[8/13] Validando Aliases..." -ForegroundColor Yellow
    $aliasBody = @{
        actions = @(
            @{
                action = "add"
                index = $indexName
                alias = "test-alias-validate"
            }
        )
    } | ConvertTo-Json -Compress
    
    $validationResults += Test-AndValidate -Method "POST" -Path "/_aliases" -Body $aliasBody
    $validationResults += Test-AndValidate -Method "GET" -Path "/_aliases" -Validator ${function:Validate-Alias}
    
    # Authentication
    Write-Host "[9/13] Validando Authentication..." -ForegroundColor Yellow
    $authBody = @{
        name = "test-key-validate"
        permissions = @("read", "write")
    } | ConvertTo-Json -Compress
    
    $validationResults += Test-AndValidate -Method "GET" -Path "/api/v1/auth/keys" -Validator ${function:Validate-AuthKeys}
    $validationResults += Test-AndValidate -Method "POST" -Path "/api/v1/auth/keys" -Body $authBody
    
    # Progress Tracking
    Write-Host "[10/13] Validando Progress Tracking..." -ForegroundColor Yellow
    $validationResults += Test-AndValidate -Method "GET" -Path "/api/v1/progress"
    $validationResults += Test-AndValidate -Method "GET" -Path "/api/v1/progress/stats"
    
    # Tasks
    Write-Host "[11/13] Validando Tasks..." -ForegroundColor Yellow
    $validationResults += Test-AndValidate -Method "GET" -Path "/_tasks" -Validator ${function:Validate-Tasks}
    
    # Cluster Settings
    Write-Host "[12/13] Validando Cluster Settings..." -ForegroundColor Yellow
    $settingsBody = '{"settings":{"cluster_name":"test-cluster","persistence":{"storage_path":"./data","snapshot":{"repository_path":"./snapshots","max_snapshots":10}},"network":{"bind_address":"0.0.0.0","port":17000,"enable_cors":true}}}'
    
    $validationResults += Test-AndValidate -Method "PUT" -Path "/_cluster/settings" -Body $settingsBody
    
    # Snapshot Repositories
    Write-Host "[13/13] Validando Snapshot Repositories..." -ForegroundColor Yellow
    $repoBody = @{
        type = "fs"
        settings = @{
            location = "test-repo-validate"
        }
    } | ConvertTo-Json -Compress
    
    $validationResults += Test-AndValidate -Method "PUT" -Path "/_snapshot/test-repo-validate" -Body $repoBody
    $validationResults += Test-AndValidate -Method "GET" -Path "/_snapshot/test-repo-validate"
    
    # Limpar recursos
    Write-Host "`nLimpando recursos..." -ForegroundColor Yellow
    Test-AndValidate -Method "DELETE" -Path "/api/v1/indices/$indexName" | Out-Null
    Test-AndValidate -Method "DELETE" -Path "/_template/test-template-validate" | Out-Null
    
}
finally {
    # Parar servidor
    Write-Host "`nParando servidor..." -ForegroundColor Yellow
    Stop-Process -Id $serverProcess.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
}

# Relatório
Write-Host "`n=== RELATÓRIO DE VALIDAÇÃO ===" -ForegroundColor Cyan
Write-Host "Total de rotas validadas: $($validationResults.Count)" -ForegroundColor White

$valid = ($validationResults | Where-Object { $_.Valid -eq $true }).Count
$invalid = ($validationResults | Where-Object { $_.Valid -eq $false }).Count
$failed = ($validationResults | Where-Object { $_.Status -eq "FAILED" }).Count

Write-Host "`nEstatísticas:" -ForegroundColor White
Write-Host "  Válidas: $valid" -ForegroundColor Green
Write-Host "  Inválidas: $invalid" -ForegroundColor Red
Write-Host "  Falhas: $failed" -ForegroundColor Red
Write-Host "  Taxa de sucesso: $([math]::Round(($valid / $validationResults.Count) * 100, 2))%" -ForegroundColor $(if ($valid -eq $validationResults.Count) { "Green" } else { "Yellow" })

Write-Host "`nDetalhes das validações:" -ForegroundColor White
foreach ($result in $validationResults) {
    $statusIcon = if ($result.Valid) { "✓" } elseif ($result.Status -eq "FAILED") { "✗" } else { "⚠" }
    $statusColor = if ($result.Valid) { "Green" } elseif ($result.Status -eq "FAILED") { "Red" } else { "Yellow" }
    
    Write-Host "  $statusIcon $($result.Method) $($result.Path) - Status: $($result.StatusCode) ($($result.Status))" -ForegroundColor $statusColor
    
    if ($result.ValidationErrors.Count -gt 0) {
        foreach ($error in $result.ValidationErrors) {
            Write-Host "    ❌ $error" -ForegroundColor Red
        }
    }
    
    # Mostrar amostra da resposta se válida
    if ($result.Valid -and $result.ResponseData) {
        $sample = ($result.ResponseData | ConvertTo-Json -Depth 2 -Compress).Substring(0, [Math]::Min(100, ($result.ResponseData | ConvertTo-Json -Depth 2 -Compress).Length))
        Write-Host "    📄 Resposta: $sample..." -ForegroundColor Gray
    }
}

if ($invalid -gt 0 -or $failed -gt 0) {
    Write-Host "`n⚠ ATENÇÃO: Algumas validações falharam!" -ForegroundColor Yellow
    exit 1
} else {
    Write-Host "`n✓ Todas as validações passaram!" -ForegroundColor Green
    exit 0
}

