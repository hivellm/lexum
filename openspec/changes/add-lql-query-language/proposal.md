## Why

Lexum needs a powerful, SQL-like query language (LQL) to provide an intuitive and expressive way to search data. While the Query DSL (JSON-based) works for programmatic access, LQL makes it easier for users to write complex queries, perform analytics, and integrate with BI tools.

## What Changes

- Implement LQL lexer and tokenizer
- Create recursive descent parser for LQL syntax
- Build Abstract Syntax Tree (AST) representation
- Implement type system and type checking
- Create query planner and optimizer
- Build execution engine
- Add support for all LQL operations (FROM, WHERE, MATCH, SORT, LIMIT, SELECT, AGGREGATE, etc.)
- Implement LQL-to-QueryDSL translator
- Add LQL validation and error reporting
- Implement LQL API endpoint (POST /_lql)

## Impact

- Affected specs: `lql-language`, `query-parsing`, `query-execution`
- Affected code: Creates `lexum-core/src/query/lql/`:
  - `lexer.rs` - Tokenization
  - `parser.rs` - Parser
  - `ast.rs` - AST types
  - `types.rs` - Type system
  - `planner.rs` - Query planner
  - `optimizer.rs` - Query optimizer
  - `executor.rs` - Execution
- Adds `lexum-server/src/api/lql.rs` - LQL endpoint
- Dependencies: pest (parser), or nom (parser combinator)
- Performance target: Parse + plan < 10ms for typical queries

