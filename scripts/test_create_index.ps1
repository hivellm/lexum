$body = @{
    name = "test_debug"
    fields = @(
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
} | ConvertTo-Json -Depth 10 -Compress

Write-Host "Request Body:"
Write-Host $body
Write-Host ""

try {
    $response = Invoke-WebRequest -Uri "http://localhost:17000/api/v1/indices" -Method POST -Body $body -ContentType "application/json"
    Write-Host "Status: $($response.StatusCode)"
    Write-Host "Response: $($response.Content)"
} catch {
    Write-Host "Error: $($_.Exception.Message)"
    if ($_.Exception.Response) {
        $statusCode = $_.Exception.Response.StatusCode.Value__
        Write-Host "Status Code: $statusCode"
        
        try {
            $stream = $_.Exception.Response.GetResponseStream()
            $reader = New-Object System.IO.StreamReader($stream)
            $responseBody = $reader.ReadToEnd()
            $reader.Close()
            $stream.Close()
            Write-Host "Response Body: $responseBody"
        } catch {
            Write-Host "Could not read response body: $($_.Exception.Message)"
        }
    }
}

