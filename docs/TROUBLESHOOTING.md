# Troubleshooting Guide

This guide helps diagnose and resolve common issues with Lexum.

## Table of Contents

- [Configuration Issues](#configuration-issues)
- [Logging Problems](#logging-problems)
- [Index Operations](#index-operations)
- [Search Issues](#search-issues)
- [Server Problems](#server-problems)
- [Performance Issues](#performance-issues)

## Configuration Issues

### Config file not found

**Problem**: `Failed to load config: No such file or directory`

**Solution**:
1. Verify config file path: `ls config.yml`
2. Use absolute path: `lexum --config /full/path/to/config.yml`
3. Copy example: `cp config.example.yml config.yml`

### Invalid port configuration

**Problem**: `Port conflict: HTTP and transport ports must be different`

**Solution**:
```yaml
network:
  http_port: 9200
  transport_port: 9300  # Must be different
```

### Environment variables not working

**Problem**: Environment overrides not applied

**Solution**:
- Use correct prefix: `LEXUM_NETWORK_HTTP_PORT=9201`
- Check variable names match config structure
- Verify env vars with: `printenv | grep LEXUM`

## Logging Problems

### No log output

**Problem**: Logs not appearing

**Solution**:
1. Check log level: Set to `debug` for development
```yaml
logging:
  level: "debug"
  format: "pretty"
```

2. Verify outputs configured:
```yaml
logging:
  outputs:
    - "stdout"
    - "file"
```

### Log file not created

**Problem**: `./logs/lexum.log` doesn't exist

**Solution**:
1. Create logs directory: `mkdir -p ./logs`
2. Check write permissions: `ls -la ./logs`
3. Enable file output in config:
```yaml
logging:
  outputs:
    - "file"
```

### Log rotation not working

**Problem**: Old logs not being rotated

**Solution**:
- Rotation is daily by default
- Check if server ran across midnight boundary
- Verify logs directory: `ls -la ./logs/`

## Index Operations

### Index creation fails

**Problem**: `Failed to create index: Invalid argument`

**Solution**:
1. Check data directory exists: `mkdir -p ./data`
2. Verify permissions: `chmod 755 ./data`
3. Ensure index name is valid (lowercase, no spaces)

**Example**:
```bash
# Good
lexum index create my-index --schema schema.yml

# Bad (uppercase)
lexum index create MY_INDEX --schema schema.yml
```

### Index not found

**Problem**: `Index not found: my-index`

**Solution**:
1. List all indices: `lexum index list`
2. Check exact name (case-sensitive)
3. Verify server is using correct data directory

### Schema validation error

**Problem**: `Schema must have at least one field`

**Solution**:
```yaml
# schema.yml must have fields
fields:
  - name: "title"
    type: "text"
    indexed: true
```

## Search Issues

### No results found

**Problem**: Query returns 0 results but documents exist

**Solutions**:
1. **Check if documents are indexed**:
```bash
lexum index get my-index
# Check num_docs > 0
```

2. **Wait for refresh**: Documents may not be searchable immediately
   - Default refresh: 1 second
   - Force refresh by waiting or adjusting `refresh_interval`

3. **Try MatchAll query**:
```bash
lexum search my-index "*"
```

4. **Check field name**: Ensure querying correct field
```json
{
  "query": {
    "match": {
      "field": "title",  // Must match schema
      "query": "search term"
    }
  }
}
```

### Fuzzy search not working

**Problem**: Fuzzy query returns no results

**Solution**:
- Increase fuzziness: `"fuzziness": 2`
- Reduce prefix_length: `"prefix_length": 0`
- Check field is text type, not keyword

**Example**:
```json
{
  "query": {
    "fuzzy": {
      "field": "name",
      "value": "jhon",
      "fuzziness": 2,
      "prefix_length": 0
    }
  }
}
```

### Phrase query too strict

**Problem**: Phrase query matches nothing

**Solution**: Add slop for flexibility
```json
{
  "query": {
    "phrase": {
      "field": "content",
      "phrase": "quick fox",
      "slop": 2  // Allows "quick brown fox"
    }
  }
}
```

## Server Problems

### Server won't start

**Problem**: `Address already in use`

**Solution**:
1. Check if server already running: `ps aux | grep lexum`
2. Kill existing: `pkill lexum-server`
3. Use different port in config:
```yaml
network:
  http_port: 9201
```

### Graceful shutdown not working

**Problem**: Server doesn't stop on Ctrl+C

**Solution**:
- Wait 5-10 seconds for graceful shutdown
- Check logs for shutdown messages
- Force kill if needed: `kill -9 <pid>`

### Rate limiting blocking requests

**Problem**: `429 Too Many Requests`

**Solution**:
- Default: 100 requests/minute
- Wait for window to reset (60 seconds)
- Or disable in development (future config option)

## Performance Issues

### Slow indexing

**Problem**: Documents taking too long to index

**Diagnostics**:
```bash
# Check system resources
top
df -h  # Disk space

# Monitor logs
tail -f ./logs/lexum.log | grep "Document indexed"
```

**Solutions**:
1. Use bulk operations instead of single documents
2. Increase refresh_interval temporarily
3. Check available disk space
4. Monitor memory usage

### Slow search queries

**Problem**: Queries taking >100ms

**Diagnostics**:
- Check result `took_ms` in response
- Enable debug logging to see cache hits
- Monitor query complexity

**Solutions**:
1. **Use query cache** (enabled by default)
2. **Reduce result limit**: Request fewer documents
3. **Optimize queries**: Use term queries instead of match when possible
4. **Check sorting**: Sorting can be expensive on large result sets

**Example optimization**:
```json
{
  "query": {
    "term": {  // Faster than "match" for exact matches
      "field": "status",
      "value": "active"
    }
  },
  "limit": 10  // Don't request more than needed
}
```

### High memory usage

**Problem**: Server using too much RAM

**Solutions**:
1. Clear query cache: Restart server or implement cache eviction
2. Reduce number of shards (for distributed setup)
3. Monitor with: `cargo run --bin lexum-server -- --profile`

## Common Error Messages

### "Failed to parse query"

**Cause**: Invalid query JSON or DSL syntax

**Solution**: Validate JSON structure
```bash
# Use online JSON validator
# Or check with: cat query.json | jq
```

### "Field not found"

**Cause**: Querying field that doesn't exist in schema

**Solution**:
1. Get index info: `lexum index get my-index`
2. Check schema definition
3. Ensure field name matches exactly (case-sensitive)

### "Task join error"

**Cause**: Internal search executor error

**Solution**:
1. Check server logs for details
2. Enable debug logging
3. Verify index isn't corrupted
4. Try recreating index if persistent

## Getting Help

If issues persist:

1. **Enable debug logging**:
```yaml
logging:
  level: "debug"
```

2. **Check logs**:
```bash
tail -f ./logs/lexum.log
```

3. **Run tests**:
```bash
cargo test --workspace
```

4. **Check GitHub Issues**: Search for similar problems

5. **Create issue** with:
   - Lexum version
   - OS and Rust version
   - Full error message
   - Minimal reproduction steps
   - Log output

## Quick Reference

### Reset Everything
```bash
# Stop server
pkill lexum-server

# Remove all data
rm -rf ./data ./logs

# Recreate directories
mkdir -p ./data ./logs

# Restart
lexum-server
```

### Verify Installation
```bash
# Check versions
lexum-server --version
lexum-cli --version

# Run tests
cargo test --workspace

# Check config
lexum config validate
```

### Performance Monitoring
```bash
# Watch resource usage
watch -n 1 'ps aux | grep lexum'

# Monitor disk I/O
iostat -x 1

# Check cache size (in logs)
grep "cache_size" ./logs/lexum.log
```




