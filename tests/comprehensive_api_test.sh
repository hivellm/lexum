#!/bin/bash
# Comprehensive API Test Script for Lexum
# Tests all implemented endpoints and functionality

set -e

BASE_URL="${LEXUM_URL:-http://localhost:9200}"
TEST_INDEX="test_index_$(date +%s)"
TEST_REPO="test_repo_$(date +%s)"
TEST_SNAPSHOT="test_snapshot_$(date +%s)"
TEST_TEMPLATE="test_template_$(date +%s)"
TEST_ALIAS="test_alias_$(date +%s)"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASSED=0
FAILED=0

# Helper functions
test_endpoint() {
    local method=$1
    local endpoint=$2
    local data=$3
    local expected_status=$4
    local description=$5
    
    if [ -z "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" 2>/dev/null)
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" \
            -H "Content-Type: application/json" \
            -d "$data" 2>/dev/null)
    fi
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$http_code" -eq "$expected_status" ]; then
        echo -e "${GREEN}✓${NC} $description (HTTP $http_code)"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}✗${NC} $description (Expected HTTP $expected_status, got $http_code)"
        echo "  Response: $body"
        ((FAILED++))
        return 1
    fi
}

echo "=========================================="
echo "Lexum Comprehensive API Test Suite"
echo "=========================================="
echo "Base URL: $BASE_URL"
echo "Test Index: $TEST_INDEX"
echo ""

# 1. Health Check
echo "=== 1. Health Check ==="
test_endpoint "GET" "/health" "" "200" "Health check endpoint"

# 2. Cluster Info
echo ""
echo "=== 2. Cluster Operations ==="
test_endpoint "GET" "/" "" "200" "Cluster info (root endpoint)"
test_endpoint "GET" "/_cluster/health" "" "200" "Cluster health"
test_endpoint "GET" "/_cluster/stats" "" "200" "Cluster stats"
test_endpoint "GET" "/_cluster/state" "" "200" "Cluster state"
test_endpoint "GET" "/_nodes/stats" "" "200" "Node stats"
test_endpoint "GET" "/_cluster/settings" "" "200" "Get cluster settings"

# 3. Index Operations
echo ""
echo "=== 3. Index Operations ==="
SCHEMA='{"fields":[{"name":"title","type":"text","indexed":true,"stored":true},{"name":"content","type":"text","indexed":true}]}'
CREATE_INDEX='{"name":"'$TEST_INDEX'","mappings":{"fields":'$SCHEMA'}}'

test_endpoint "POST" "/api/v1/indices" "$CREATE_INDEX" "201" "Create index"
test_endpoint "GET" "/api/v1/indices" "" "200" "List indices"
test_endpoint "GET" "/api/v1/indices/$TEST_INDEX" "" "200" "Get index info"
test_endpoint "GET" "/api/v1/indices/$TEST_INDEX/stats" "" "200" "Get index stats"

# 4. Document Operations
echo ""
echo "=== 4. Document Operations ==="
DOC='{"document":{"title":"Test Document","content":"This is a test document"}}'
test_endpoint "POST" "/api/v1/indices/$TEST_INDEX/documents" "$DOC" "201" "Add document"

# Get document ID from response
DOC_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/indices/$TEST_INDEX/documents" \
    -H "Content-Type: application/json" \
    -d "$DOC" 2>/dev/null)
DOC_ID=$(echo "$DOC_RESPONSE" | grep -o '"id":"[^"]*' | cut -d'"' -f4)

if [ -n "$DOC_ID" ]; then
    test_endpoint "GET" "/api/v1/indices/$TEST_INDEX/documents/$DOC_ID" "" "200" "Get document"
    
    UPDATE_DOC='{"document":{"title":"Updated Document","content":"Updated content"}}'
    test_endpoint "PUT" "/api/v1/indices/$TEST_INDEX/documents/$DOC_ID" "$UPDATE_DOC" "200" "Update document"
fi

# 5. Search Operations
echo ""
echo "=== 5. Search Operations ==="
SEARCH_POST='{"query":{"match":{"field":"title","query":"Test"}}}'
test_endpoint "POST" "/api/v1/indices/$TEST_INDEX/search" "$SEARCH_POST" "200" "POST search"

test_endpoint "GET" "/api/v1/indices/$TEST_INDEX/search?q=Test" "" "200" "GET search with query string"

SEARCH_WITH_FILTER='{"query":{"match":{"field":"title","query":"Test"}},"filter":[{"term":{"field":"title","value":"Test"}}]}'
test_endpoint "POST" "/api/v1/indices/$TEST_INDEX/search" "$SEARCH_WITH_FILTER" "200" "Search with filter"

# 6. Bulk Operations
echo ""
echo "=== 6. Bulk Operations ==="
BULK='{"operations":[{"action":"index","index":"'$TEST_INDEX'","document":{"title":"Bulk Doc 1","content":"Content 1"}},{"action":"index","index":"'$TEST_INDEX'","document":{"title":"Bulk Doc 2","content":"Content 2"}}]}'
test_endpoint "POST" "/api/v1/bulk" "$BULK" "200" "Bulk operations"

# 7. Snapshot Repository Operations
echo ""
echo "=== 7. Snapshot Repository Operations ==="
REPO_CONFIG='{"type":"fs","settings":{"location":"/tmp/test_repo"}}'
test_endpoint "PUT" "/_snapshot/$TEST_REPO" "$REPO_CONFIG" "200" "Create snapshot repository"
test_endpoint "GET" "/_snapshot/$TEST_REPO" "" "200" "Get snapshot repository"
test_endpoint "GET" "/_snapshot" "" "200" "List snapshot repositories"

# 8. Snapshot Operations
echo ""
echo "=== 8. Snapshot Operations ==="
SNAPSHOT_CONFIG='{"indices":["'$TEST_INDEX'"],"include_global_state":false}'
test_endpoint "PUT" "/_snapshot/$TEST_REPO/$TEST_SNAPSHOT" "$SNAPSHOT_CONFIG" "200" "Create snapshot"
test_endpoint "GET" "/_snapshot/$TEST_REPO/$TEST_SNAPSHOT" "" "200" "Get snapshot"
test_endpoint "GET" "/_snapshot/$TEST_REPO/_all" "" "200" "List snapshots"
test_endpoint "GET" "/_snapshot/$TEST_REPO/_stats" "" "200" "Get snapshot stats"
test_endpoint "GET" "/_snapshot/_stats" "" "200" "Get global snapshot stats"

# 9. Template Operations
echo ""
echo "=== 9. Template Operations ==="
TEMPLATE_CONFIG='{"index_patterns":["test_*"],"mappings":{"fields":'$SCHEMA'},"settings":{"number_of_shards":1}}'
test_endpoint "PUT" "/_template/$TEST_TEMPLATE" "$TEMPLATE_CONFIG" "200" "Create template"
test_endpoint "GET" "/_template/$TEST_TEMPLATE" "" "200" "Get template"
test_endpoint "GET" "/_template" "" "200" "List templates"

# 10. Alias Operations
echo ""
echo "=== 10. Alias Operations ==="
ALIAS_OPS='{"actions":[{"add":{"index":"'$TEST_INDEX'","alias":"'$TEST_ALIAS'"}}]}'
test_endpoint "POST" "/_aliases" "$ALIAS_OPS" "200" "Add alias"
test_endpoint "GET" "/_aliases" "" "200" "List all aliases"
test_endpoint "GET" "/$TEST_INDEX/_alias" "" "200" "Get index aliases"
test_endpoint "GET" "/$TEST_INDEX/_alias/$TEST_ALIAS" "" "200" "Get specific alias"

# 11. Progress Tracking
echo ""
echo "=== 11. Progress Tracking ==="
test_endpoint "GET" "/api/v1/progress" "" "200" "List progress sessions"
test_endpoint "GET" "/api/v1/progress/stats" "" "200" "Get progress stats"

# 12. Reindex Operations
echo ""
echo "=== 12. Reindex Operations ==="
REINDEX_DEST="reindex_dest_$(date +%s)"
REINDEX_CONFIG='{"source":{"index":"'$TEST_INDEX'"},"dest":{"index":"'$REINDEX_DEST'"}}'
# Note: This will fail if destination index doesn't exist, which is expected
test_endpoint "POST" "/_reindex" "$REINDEX_CONFIG" "400" "Reindex operation (expected to fail without dest index)"

test_endpoint "GET" "/_tasks" "" "200" "List tasks"

# 13. Rollover Operations
echo ""
echo "=== 13. Rollover Operations ==="
test_endpoint "GET" "/api/v1/indices/$TEST_INDEX/_rollover" "" "200" "Get rollover conditions"
ROLLOVER_CONFIG='{"conditions":{"max_age":"30d","max_docs":1000}}'
test_endpoint "PUT" "/api/v1/indices/$TEST_INDEX/_rollover" "$ROLLOVER_CONFIG" "200" "Update rollover conditions"

# Cleanup
echo ""
echo "=== Cleanup ==="
test_endpoint "DELETE" "/_template/$TEST_TEMPLATE" "" "200" "Delete template"
test_endpoint "DELETE" "/_snapshot/$TEST_REPO/$TEST_SNAPSHOT" "" "200" "Delete snapshot"
test_endpoint "DELETE" "/api/v1/indices/$TEST_INDEX" "" "200" "Delete test index"

# Summary
echo ""
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo "Total: $((PASSED + FAILED))"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi

