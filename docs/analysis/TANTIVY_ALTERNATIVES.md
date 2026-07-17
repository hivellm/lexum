# Tantivy Alternatives for Cross-Platform Support

## Context

Tantivy has compatibility issues with WSL (Windows Subsystem for Linux) filesystem, especially when accessing Windows-mounted drives (e.g., `/mnt/f/`). This document analyzes alternatives that work on all platforms.

## Current Tantivy Usage Analysis

### Features Used
- **Schema Management**: Field definitions (text, keyword, i64, f64, date)
- **Index Creation**: Creating indices in filesystem directories
- **IndexWriter**: Synchronous document writing
- **IndexReader**: Reading and searching documents
- **BM25 Scoring**: Relevance algorithm
- **Document Storage**: Document storage and retrieval
- **Query Types**: Match, Term, Range, Boolean, Fuzzy, Phrase

### Critical Dependencies
- Filesystem operations (creating indices in directories)
- Synchronous operations (wrapped in `spawn_blocking` for async)
- Dynamic schema (fields defined at runtime)

## Rust Alternatives

### 1. **Meilisearch (Embedded)** ⭐ Recommended

**Status**: Native Rust library, cross-platform

**Advantages**:
- ✅ **100% Rust** - No external dependencies
- ✅ **Cross-platform** - Windows, Linux, macOS work natively
- ✅ **High Performance** - Optimized for speed
- ✅ **Similar API** - Concepts similar to Tantivy
- ✅ **Active** - Active development and large community
- ✅ **Typo Tolerance** - Typo-tolerant search
- ✅ **Faceting** - Native faceting support

**Disadvantages**:
- ⚠️ **Less Flexible** - More opinionated than Tantivy
- ⚠️ **Embedded Mode** - Needs to be used as library (not primary use case)
- ⚠️ **Migration** - Requires significant refactoring

**Integration**:
```rust
// Usage example (conceptual)
use meilisearch_sdk::client::*;

let client = Client::new("http://localhost:7700", "master-key");
// Or use embedded library if available
```

**Compatibility**: ⭐⭐⭐⭐⭐ (Excellent)

---

### 2. **Sonic** ⭐⭐ Good Alternative

**Status**: Native Rust library, cross-platform

**Advantages**:
- ✅ **100% Rust** - Pure Rust implementation
- ✅ **Cross-platform** - Works on all platforms
- ✅ **Lightweight** - Small and efficient library
- ✅ **Fast** - Performance-focused
- ✅ **Simple** - Simpler API than Tantivy

**Disadvantages**:
- ⚠️ **Fewer Features** - More limited functionality
- ⚠️ **Less Mature** - Smaller community than Tantivy
- ⚠️ **Fixed Schema** - Less flexibility for dynamic schemas

**Compatibility**: ⭐⭐⭐⭐ (Very Good)

---

### 3. **RediSearch** (via Redis) ⭐⭐⭐ External Option

**Status**: Redis module, cross-platform

**Advantages**:
- ✅ **Cross-platform** - Redis works on all platforms
- ✅ **Fast** - Excellent performance
- ✅ **Distributed** - Native clustering support
- ✅ **Mature** - Well-established library
- ✅ **Rust Client** - Rust client available (`redis` crate)

**Disadvantages**:
- ⚠️ **External Dependency** - Requires Redis running
- ⚠️ **Not Embeddable** - Cannot be embedded in process
- ⚠️ **Overhead** - Network/pipe communication
- ⚠️ **Complex Migration** - Significant architectural change

**Compatibility**: ⭐⭐⭐⭐ (Very Good, but requires external service)

---

### 4. **SQLite FTS5** ⭐⭐⭐ Simple Option

**Status**: SQLite extension, cross-platform

**Advantages**:
- ✅ **Cross-platform** - SQLite works on all platforms
- ✅ **Embeddable** - Can be embedded in process
- ✅ **Mature** - Very well-established library
- ✅ **Rust Support** - `rusqlite` crate with FTS5 support
- ✅ **Simple** - Familiar SQL API

**Disadvantages**:
- ⚠️ **Fewer Features** - Limited functionality compared to Tantivy
- ⚠️ **Performance** - May be slower for complex cases
- ⚠️ **SQL-based** - Different paradigm (SQL vs programmatic)
- ⚠️ **Less Flexible** - Less control over indexing

**Compatibility**: ⭐⭐⭐⭐⭐ (Excellent)

**Usage Example**:
```rust
use rusqlite::{Connection, Result};

let conn = Connection::open("search.db")?;
conn.execute(
    "CREATE VIRTUAL TABLE docs USING fts5(title, content)",
    [],
)?;
```

---

### 5. **Custom Implementation with SQLite/BTree** ⭐⭐ Custom Option

**Status**: Custom implementation using Rust structures

**Advantages**:
- ✅ **Total Control** - Custom implementation
- ✅ **Cross-platform** - Uses only Rust stdlib
- ✅ **No Dependencies** - Doesn't depend on problematic external libraries
- ✅ **Optimized** - Can be optimized for specific cases

**Disadvantages**:
- ⚠️ **Development** - Significant implementation work
- ⚠️ **Maintenance** - High maintenance cost
- ⚠️ **Bugs** - Risk of own bugs
- ⚠️ **Performance** - May not reach Tantivy performance

**Compatibility**: ⭐⭐⭐⭐⭐ (Excellent, but requires development)

---

## Detailed Comparison

| Alternative | Platform | Performance | Migration Ease | Maturity | Features |
|-------------|----------|-------------|----------------|----------|----------|
| **Meilisearch** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Sonic** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ |
| **RediSearch** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **SQLite FTS5** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Custom** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐ |

## Recommendations by Scenario

### Scenario 1: Quick Migration with Maximum Compatibility
**Recommendation**: **SQLite FTS5**
- Relatively simple migration
- Works on all platforms without issues
- Familiar SQL API
- Good for simple to medium use cases

### Scenario 2: Maximum Performance with Compatibility
**Recommendation**: **Meilisearch (Embedded)** or **Sonic**
- Performance similar or better than Tantivy
- 100% Rust, cross-platform
- Requires more migration work

### Scenario 3: Distributed Solution
**Recommendation**: **RediSearch**
- If you already use or can use Redis
- Native clustering support
- Requires architectural change

### Scenario 4: Keep Tantivy with Workaround
**Recommendation**: **Use Windows native paths** (current)
- Least migration effort
- Works on Windows native
- Documented in `docs/development/WINDOWS_NATIVE.md`

## Migration Effort Analysis

### Meilisearch
- **Effort**: Medium-High
- **Estimated Time**: 2-3 weeks
- **Required Changes**:
  - Replace Index abstraction
  - Adapt Schema to Meilisearch format
  - Refactor DocumentStore
  - Update SearchExecutor

### SQLite FTS5
- **Effort**: Medium
- **Estimated Time**: 1-2 weeks
- **Required Changes**:
  - Create abstraction over SQLite
  - Convert queries to SQL
  - Adapt DocumentStore to SQL
  - Maintain API compatibility

### Sonic
- **Effort**: Medium-High
- **Estimated Time**: 2-3 weeks
- **Required Changes**:
  - Similar to Meilisearch
  - May require more customization

## Final Recommendation

### Option 1: Short Term (Recommended)
**Keep Tantivy and use Windows Native**
- ✅ Zero migration effort
- ✅ Works immediately
- ✅ Documentation already created (`docs/development/WINDOWS_NATIVE.md`)
- ⚠️ Requires running on Windows native

### Option 2: Medium Term
**Migrate to SQLite FTS5**
- ✅ Works on all platforms
- ✅ Relatively simple migration
- ✅ Mature and stable library
- ⚠️ May have performance limitations for complex cases

### Option 3: Long Term
**Migrate to Meilisearch Embedded**
- ✅ Better performance
- ✅ More features
- ✅ 100% Rust, cross-platform
- ⚠️ Requires more migration work

## Next Steps

1. **Immediate**: Use Windows Native (already documented)
2. **Evaluate**: Test SQLite FTS5 in a separate branch
3. **Decide**: Based on results, decide if migration is necessary
4. **Implement**: If necessary, create detailed migration plan

## Migration Example: SQLite FTS5

### Proposed Abstraction

```rust
// lexum-core/src/search/sqlite_executor.rs
use rusqlite::{Connection, Result as SqlResult};
use crate::query::Query;
use crate::search::result::SearchResult;

pub struct SqliteSearchExecutor {
    conn: Connection,
}

impl SqliteSearchExecutor {
    pub fn new(db_path: &str) -> Self {
        let conn = Connection::open(db_path).unwrap();
        // Create FTS5 table
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS docs USING fts5(
                doc_id UNINDEXED,
                title,
                content,
                data UNINDEXED
            )",
            [],
        ).unwrap();
        Self { conn }
    }

    pub async fn search(&self, query: &Query, limit: usize, offset: usize) -> SearchResult {
        // Convert Query to SQL FTS5 query
        let sql_query = self.query_to_sql(query);
        
        let mut stmt = self.conn.prepare(
            "SELECT doc_id, data, rank FROM docs 
             WHERE docs MATCH ? 
             ORDER BY rank 
             LIMIT ? OFFSET ?"
        ).unwrap();
        
        // Execute and convert results
        // ...
    }
}
```

### Migration Advantages
- ✅ Works on all platforms without issues
- ✅ Familiar SQL API
- ✅ Relatively simple migration
- ✅ Mature and stable library

### Disadvantages
- ⚠️ Performance may be lower for very complex cases
- ⚠️ Less control over scoring algorithms
- ⚠️ SQL limitations for very complex queries

## Migration Example: Meilisearch Embedded

### Proposed Abstraction

```rust
// lexum-core/src/search/meilisearch_executor.rs
// Note: Meilisearch currently doesn't have official embedded mode
// Would need to use as service or contribute to embedded library

use meilisearch_sdk::client::*;

pub struct MeilisearchExecutor {
    client: Client,
    index_name: String,
}

impl MeilisearchExecutor {
    pub async fn search(&self, query: &Query, limit: usize, offset: usize) -> SearchResult {
        // Convert Query to Meilisearch query format
        let meilisearch_query = self.query_to_meilisearch(query);
        
        let results = self.client
            .index(&self.index_name)
            .search()
            .with_query(&meilisearch_query)
            .with_limit(limit)
            .with_offset(offset)
            .execute::<serde_json::Value>()
            .await
            .unwrap();
        
        // Convert results to SearchResult
        // ...
    }
}
```

## Migration Impact on Code

### Files That Would Need Changes

1. **lexum-core/src/index/manager.rs**
   - Replace `TantivyIndex` with new abstraction
   - Adapt index creation

2. **lexum-core/src/search/executor.rs**
   - Replace Tantivy queries with new implementation
   - Adapt query conversion

3. **lexum-core/src/document/store.rs**
   - Replace `IndexWriter` with new implementation
   - Adapt write operations

4. **lexum-core/src/schema/builder.rs**
   - Adapt schema construction to new format

### Change Estimate
- **Lines of code**: ~2000-3000 lines affected
- **Files**: ~15-20 files
- **Time**: 2-4 weeks (depending on chosen alternative)
- **Tests**: Requires updating ~100+ tests

## Recommended Decision

### Phase 1: Short Term (Immediate)
✅ **Use Windows Native** (already implemented)
- Zero effort
- Works immediately
- Complete documentation

### Phase 2: Medium Term (If necessary)
🔍 **Evaluate SQLite FTS5**
- Create experimental branch
- Implement prototype
- Compare performance

### Phase 3: Long Term (If performance is critical)
🚀 **Consider Meilisearch or Custom**
- Only if SQLite doesn't meet needs
- Requires deep requirements analysis

## Conclusion

**For the current moment**, the best alternative is **keeping Tantivy and using Windows Native**, because:
1. ✅ Zero migration effort
2. ✅ Works perfectly on Windows
3. ✅ Performance maintained
4. ✅ Documentation already created

**If migration becomes necessary in the future**, **SQLite FTS5** is the best option because:
1. ✅ Works on all platforms
2. ✅ Relatively simple migration
3. ✅ Mature and stable library
4. ✅ Adequate performance for most cases

## References

- [Meilisearch Documentation](https://www.meilisearch.com/docs)
- [Sonic GitHub](https://github.com/valeriansaliou/sonic)
- [RediSearch Documentation](https://redis.io/docs/stack/search/)
- [SQLite FTS5 Documentation](https://www.sqlite.org/fts5.html)
- [Tantivy GitHub Issues - WSL](https://github.com/quickwit-oss/tantivy/issues)
- [rusqlite FTS5 Example](https://github.com/rusqlite/rusqlite/blob/master/examples/fts5.rs)
