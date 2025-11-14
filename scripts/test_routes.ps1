# Test script for Lexum server routes
$baseUrl = "http://127.0.0.1:17000"
$indexName = "test_index"

Write-Host "=== Testing Lexum Server Routes ===" -ForegroundColor Cyan

# 1. Health Check
Write-Host "`n1. Testing Health Check..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/health" -Method GET
    Write-Host "✓ Health check: OK" -ForegroundColor Green
    $response | ConvertTo-Json
} catch {
    Write-Host "✗ Health check failed: $_" -ForegroundColor Red
    exit 1
}

# 2. Create Index
Write-Host "`n2. Creating test index..." -ForegroundColor Yellow
$indexBody = @{
    mappings = @{
        properties = @{
            title = @{ type = "text" }
            content = @{ type = "text" }
            category = @{ type = "keyword" }
            price = @{ type = "float" }
        }
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName" -Method POST -Body $indexBody -ContentType "application/json"
    Write-Host "✓ Index created" -ForegroundColor Green
} catch {
    Write-Host "✗ Index creation failed: $_" -ForegroundColor Red
    if ($_.Exception.Response.StatusCode -eq 409) {
        Write-Host "  Index already exists, continuing..." -ForegroundColor Yellow
    } else {
        exit 1
    }
}

# 3. Add Documents
Write-Host "`n3. Adding test documents..." -ForegroundColor Yellow
$documents = @(
    @{ id = "1"; title = "Introduction to Rust Programming"; content = "Rust is a systems programming language that focuses on safety and performance. It provides memory safety without garbage collection."; category = "programming"; price = 29.99 },
    @{ id = "2"; title = "Advanced Search Techniques"; content = "Learn advanced search algorithms and data structures. This book covers fuzzy matching, phrase queries, and wildcard searches."; category = "algorithms"; price = 39.99 },
    @{ id = "3"; title = "Database Design Patterns"; content = "Good database design is crucial for application performance and maintainability. This guide covers indexing strategies."; category = "database"; price = 34.99 },
    @{ id = "4"; title = "Regex Mastery Guide"; content = "Master regular expressions with practical examples. Learn pattern matching, lookaheads, and performance optimization."; category = "programming"; price = 24.99 }
)

foreach ($doc in $documents) {
    try {
        $docJson = $doc | ConvertTo-Json -Depth 10
        $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName/documents/$($doc.id)" -Method POST -Body $docJson -ContentType "application/json"
        Write-Host "✓ Document $($doc.id) added" -ForegroundColor Green
    } catch {
        Write-Host "✗ Failed to add document $($doc.id): $_" -ForegroundColor Red
    }
}

Start-Sleep -Seconds 2

# 4. Test Basic Search
Write-Host "`n4. Testing Basic Search..." -ForegroundColor Yellow
$searchBody = @{
    query = @{
        match = @{
            content = "programming"
        }
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName/search" -Method POST -Body $searchBody -ContentType "application/json"
    Write-Host "✓ Basic search: Found $($response.hits.total.value) results" -ForegroundColor Green
    $response.hits.hits | Select-Object -First 2 | ConvertTo-Json -Depth 5
} catch {
    Write-Host "✗ Basic search failed: $_" -ForegroundColor Red
}

# 5. Test Search with Highlighting
Write-Host "`n5. Testing Search with Highlighting..." -ForegroundColor Yellow
$highlightBody = @{
    query = @{
        match = @{
            content = "programming"
        }
    }
    highlight = @{
        fields = @("content", "title")
        pre_tag = "<mark>"
        post_tag = "</mark>"
        fragment_size = 100
        max_fragments = 3
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName/search" -Method POST -Body $highlightBody -ContentType "application/json"
    Write-Host "✓ Highlighting search: Found $($response.hits.total.value) results" -ForegroundColor Green
    $response.hits.hits | Select-Object -First 1 | ConvertTo-Json -Depth 5
} catch {
    Write-Host "✗ Highlighting search failed: $_" -ForegroundColor Red
}

# 6. Test Fuzzy Search
Write-Host "`n6. Testing Fuzzy Search..." -ForegroundColor Yellow
$fuzzyBody = @{
    query = @{
        fuzzy = @{
            field = "title"
            value = "progamming"
            fuzziness = 2
        }
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName/search" -Method POST -Body $fuzzyBody -ContentType "application/json"
    Write-Host "✓ Fuzzy search: Found $($response.hits.total.value) results" -ForegroundColor Green
    $response.hits.hits | Select-Object -First 1 | ConvertTo-Json -Depth 5
} catch {
    Write-Host "✗ Fuzzy search failed: $_" -ForegroundColor Red
}

# 7. Test Phrase Query
Write-Host "`n7. Testing Phrase Query..." -ForegroundColor Yellow
$phraseBody = @{
    query = @{
        phrase = @{
            field = "content"
            value = "database design"
            slop = 2
        }
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName/search" -Method POST -Body $phraseBody -ContentType "application/json"
    Write-Host "✓ Phrase query: Found $($response.hits.total.value) results" -ForegroundColor Green
    $response.hits.hits | Select-Object -First 1 | ConvertTo-Json -Depth 5
} catch {
    Write-Host "✗ Phrase query failed: $_" -ForegroundColor Red
}

# 8. Test Wildcard Query
Write-Host "`n8. Testing Wildcard Query..." -ForegroundColor Yellow
$wildcardBody = @{
    query = @{
        wildcard = @{
            field = "title"
            value = "prog*"
        }
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName/search" -Method POST -Body $wildcardBody -ContentType "application/json"
    Write-Host "✓ Wildcard query: Found $($response.hits.total.value) results" -ForegroundColor Green
    $response.hits.hits | Select-Object -First 1 | ConvertTo-Json -Depth 5
} catch {
    Write-Host "✗ Wildcard query failed: $_" -ForegroundColor Red
}

# 9. Test Regex Query
Write-Host "`n9. Testing Regex Query..." -ForegroundColor Yellow
$regexBody = @{
    query = @{
        regex = @{
            field = "title"
            value = ".*[Rr]ust.*"
            case_sensitive = $false
        }
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName/search" -Method POST -Body $regexBody -ContentType "application/json"
    Write-Host "✓ Regex query: Found $($response.hits.total.value) results" -ForegroundColor Green
    $response.hits.hits | Select-Object -First 1 | ConvertTo-Json -Depth 5
} catch {
    Write-Host "✗ Regex query failed: $_" -ForegroundColor Red
}

# 10. Test Explain API
Write-Host "`n10. Testing Explain API..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName/_explain/1?q=programming" -Method GET
    Write-Host "✓ Explain API: OK" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 5
} catch {
    Write-Host "✗ Explain API failed: $_" -ForegroundColor Red
}

# 11. Test Query with Boost
Write-Host "`n11. Testing Query with Boost..." -ForegroundColor Yellow
$boostBody = @{
    query = @{
        match = @{
            field = "title"
            value = "programming"
            boost = 2.0
        }
    }
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName/search" -Method POST -Body $boostBody -ContentType "application/json"
    Write-Host "✓ Boost query: Found $($response.hits.total.value) results" -ForegroundColor Green
    $response.hits.hits | Select-Object -First 1 | ConvertTo-Json -Depth 5
} catch {
    Write-Host "✗ Boost query failed: $_" -ForegroundColor Red
}

# 12. Test Search with Explain parameter
Write-Host "`n12. Testing Search with Explain parameter..." -ForegroundColor Yellow
$explainBody = @{
    query = @{
        match = @{
            content = "programming"
        }
    }
    explain = $true
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/indices/$indexName/search" -Method POST -Body $explainBody -ContentType "application/json"
    Write-Host "✓ Search with explain: Found $($response.hits.total.value) results" -ForegroundColor Green
    if ($response.hits.hits[0]._explanation) {
        Write-Host "✓ Explanation included in results" -ForegroundColor Green
    }
} catch {
    Write-Host "✗ Search with explain failed: $_" -ForegroundColor Red
}

Write-Host "`n=== All Tests Completed ===" -ForegroundColor Cyan
