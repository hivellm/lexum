# Test Create Index with detailed error
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

