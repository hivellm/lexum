# SPEC-014 — LQL (Lexum Query Language)

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | Cross-cutting over phase2_search-kernel-parity (kernel lowering, [proposal](../../.rulebook/tasks/phase2_search-kernel-parity/proposal.md)) and phase5_aggregations-facets (AGGREGATE/HISTOGRAM/TERMS lowering); ES\|QL-style extensions tracked as Elastic plan P2 #14 |
| **Planning source** | [docs/api/QUERY_LANGUAGE.md](../api/QUERY_LANGUAGE.md) (existing language description, normativized here); Elastic [§4.5 ES\|QL](../analysis/elastic/04-query-dsl.md) (F-030: piped languages beat nested JSON for humans and LLMs — LQL is convergent evolution); Meilisearch plan [A-04](../analysis/meilisearch/08-execution-plan.md) (LQL is the power layer, never the only door) |

Requirement IDs `LQL-xxx`. RFC 2119 keywords normative. LQL is the power-user/agent front door over the search kernel of SPEC-004 — one execution path, two front doors (three counting the ES DSL). Aggregation operations lower onto SPEC-007. Errors per SPEC-003.

## 1. Model

An LQL query is a **pipeline**: a `FROM` source clause followed by zero or more operations chained with `|`, each consuming the previous stage's document stream.

```
FROM <index_pattern> [| <operation>]*
```

- **LQL-001** LQL MUST lower onto the SPEC-004 kernel: the compiled plan is built from the same core `Query`, filter-context, sort, pagination, source-filtering, and aggregation (SPEC-007) primitives used by the simple path and the ES DSL. LQL has no private execution engine; a result reachable both via LQL and via SPEC-004 parameters MUST be identical.
- **LQL-002** LQL is never required: every LQL capability that maps onto SPEC-004/SPEC-007 surface is also reachable without LQL (A-04). Conversely, LQL MAY express compositions the flat parameters cannot (multiple `WHERE` stages, projection with `EXCEPT`).
- **LQL-003** Conformance tiers. **Core** operations (§4) are normative and MUST be implemented. **Future** constructs (§10) are documented, reserved, and non-normative: a conforming engine MUST parse far enough to recognize them and reject them with `lql_unsupported_feature` (§8) — never misparse them as something else.

## 2. Endpoint

`POST /api/v1/_lql`

```json
{
  "query": "FROM users | WHERE age > $min_age | SORT created_at DESC | LIMIT $limit",
  "params": { "min_age": 18, "limit": 10 }
}
```

- **LQL-010** `query` (required) is a single LQL pipeline. `params` (optional) binds `$name` placeholders; a placeholder with no binding, or a binding with no placeholder, fails with `lql_invalid_params`. Placeholders substitute **values** (literals only), never identifiers, operators, or clause structure — LQL parameters are injection-safe by construction.
- **LQL-011** The response uses the SPEC-004 native envelope (§6.6) for document-returning pipelines and the SPEC-007 shapes for aggregation-returning pipelines, plus `"language": "lql"` and the echoed `query`.
- **LQL-012** The CLI (`lexum lql`) MUST accept the same language with identical semantics; output format (`table`/`json`/`csv`) is presentation only.

## 3. Lexical structure and grammar

### 3.1 Lexical rules

- **LQL-020** Keywords are case-insensitive (`from` ≡ `FROM`); the canonical form is upper-case. Identifiers (index names, field paths) are case-sensitive. Field paths use `.` for nesting (`address.city`). Identifiers colliding with a reserved keyword (§7) MUST be back-quoted: `` `limit` ``.
- **LQL-021** Literals: strings in double quotes with `\"` and `\\` escapes; 64-bit integers; 64-bit floats; `true`/`false`; `null`; arrays `[v1, v2, ...]` of literals. Dates are ISO 8601 strings, interpreted per field type.
- **LQL-022** Whitespace (spaces, tabs, newlines) between tokens is insignificant; pipelines MAY span multiple lines. Comments: `--` to end of line.

### 3.2 Grammar (normative summary, EBNF)

```ebnf
Query          = FromClause ("|" Operation)*
FromClause     = "FROM" IndexPattern ("," IndexPattern)* ("AS" Identifier)?
Operation      = WhereClause | MatchClause | SortClause | LimitClause
               | SelectClause | AggregateClause | HistogramClause
               | TermsClause | JoinClause          (* JoinClause: Future, §10 *)

WhereClause    = "WHERE" Expression
MatchClause    = "MATCH" String ("IN" FieldList)? ("OPTIONS" Object)?
SortClause     = "SORT" SortField ("," SortField)*
SortField      = Field ("ASC" | "DESC")?
LimitClause    = "LIMIT" Number ("OFFSET" Number)? | "LIMIT" Number "," Number
SelectClause   = "SELECT" ("*" ("EXCEPT" "(" FieldList ")")? | SelectItem ("," SelectItem)*)
SelectItem     = Field ("AS" Identifier)?
AggregateClause= "AGGREGATE" AggItem ("," AggItem)* ("BY" FieldList)?
AggItem        = AggFunc "(" (Field ("," Number)?)? ")" ("AS" Identifier)?
HistogramClause= "HISTOGRAM" Field "BY" (Number | String) ("AS" Identifier)?
TermsClause    = "TERMS" Field ("SIZE" Number)?

Expression     = OrExpr
OrExpr         = AndExpr ("OR" AndExpr)*
AndExpr        = NotExpr ("AND" NotExpr)*
NotExpr        = ("NOT")? CompareExpr
CompareExpr    = Term (CompareOp Term)? | Term "BETWEEN" Term "AND" Term
               | Term ("NOT")? "IN" Array | Term "IS" ("NOT")? "NULL"
               | Term "CONTAINS" Literal
CompareOp      = "=" | "!=" | ">" | ">=" | "<" | "<="
Term           = Field | Literal | Param | "(" Expression ")"
Param          = "$" Identifier
FieldList      = FieldSpec ("," FieldSpec)* | "(" FieldSpec ("," FieldSpec)* ")"
FieldSpec      = Field ("^" Number)?
IndexPattern   = Identifier with optional trailing "*"
```

- **LQL-023** This grammar is the conformance surface; a parse that this grammar rejects MUST produce a positioned syntax error (§8), and constructs beyond it (subqueries, functions in `SELECT`, window functions, `UNNEST`, `JOIN`) fall under LQL-003's Future rule.

## 4. The nine operations

The pipeline vocabulary is `FROM` plus nine operation types:

| # | Operation | Purpose | Lowers to |
|---|---|---|---|
| 1 | `WHERE` | Boolean predicate filter | SPEC-004 filter context (§4.2) |
| 2 | `MATCH` | Full-text search | SPEC-004 query context (`match`/`multi_match`/`match_phrase`) |
| 3 | `SORT` | Result ordering | SPEC-004 sort contract (§9) |
| 4 | `LIMIT` | Pagination | SPEC-004 `offset`/`limit` (§5.1) |
| 5 | `SELECT` | Field projection | SPEC-004 source filtering (§6.1) |
| 6 | `AGGREGATE` | Metrics, optional grouping | SPEC-007 metrics / `terms` buckets |
| 7 | `HISTOGRAM` | Numeric/date bucketing | SPEC-007 `histogram` / `date_histogram` |
| 8 | `TERMS` | Top-terms grouping | SPEC-007 `terms` bucket |
| 9 | `JOIN` | Cross-index join | **Future** (§10) — reserved, rejected |

- **LQL-030** Operation ordering rules: `FROM` MUST be first and appear exactly once. `WHERE` and `MATCH` MAY each appear multiple times at any position before aggregation operations; multiple occurrences combine as AND (all `WHERE` stages into one filter-context `bool.filter`; multiple `MATCH` stages into `bool.must`). At most one each of `SORT`, `LIMIT`, `SELECT`; duplicates fail with a positioned `lql_syntax_error`.
- **LQL-031** Aggregation pipelines: `TERMS` and `HISTOGRAM` stages define grouping keys for a following `AGGREGATE ... [BY bucket_alias]`; `AGGREGATE ... BY field` alone is shorthand for `TERMS field` + per-bucket metrics. A pipeline is either document-returning (no aggregation stages) or aggregation-returning; `SELECT` after an aggregation stage projects aggregate aliases, and `SORT`/`LIMIT` after an aggregation stage order/truncate buckets, not documents.
- **LQL-032** `FROM a, b` and index patterns (`logs-*`) resolve to a multi-index search over the same kernel; `AS` aliases the source for field qualification. Zero resolved indices fails with `index_not_found`.

## 5. `WHERE` semantics

- **LQL-040** `WHERE` expressions compile to **filter context** (SPEC-004 §4.2): scoreless and cache-eligible. Adding a `WHERE` stage MUST NOT change any hit's `_score`.
- **LQL-041** Operator lowering:

| LQL | Kernel query |
|---|---|
| `f = v` / `f != v` | `term` / `bool.must_not(term)` |
| `f > v`, `>=`, `<`, `<=` | `range` (gt/gte/lt/lte) |
| `f BETWEEN a AND b` | `range { gte: a, lte: b }` (both ends inclusive) |
| `f IN [a, b]` / `NOT IN` | `terms` / `bool.must_not(terms)` |
| `f IS NULL` / `IS NOT NULL` | `bool.must_not(exists)` / `exists` |
| `f CONTAINS v` | `term` on the multi-valued field (array membership) |
| `AND` / `OR` / `NOT`, parentheses | nested `bool` (filter context throughout, per SRCH-043) |

- **LQL-042** Comparisons are typed: comparing a literal of the wrong type for the field fails at planning time with `lql_type_error` and the expression's position — never a silent zero-result coercion.
- **LQL-043** The same expression grammar (from `Expression` down) is the string form of the SPEC-004 `filter` parameter (SRCH-021); the two MUST share one parser.

## 6. `MATCH` semantics

- **LQL-050** `MATCH "text"` with no `IN` searches all searchable attributes and lowers to the same expansion as the SPEC-004 `q` parameter — including typo tolerance (SRCH-070..074) and `matchingStrategy` (default `last`). `MATCH` executes in query context and contributes to `_score`.
- **LQL-051** `MATCH "text" IN field` lowers to `match`; `IN (f1^3, f2)` lowers to `multi_match` with field boosts; an embedded quoted phrase (`"\"exact phrase\""`) lowers to `match_phrase`.
- **LQL-052** `OPTIONS { ... }` accepts: `operator` (`"OR"` default \| `"AND"`), `fuzziness` (int 0–2, overriding the typo-tolerance budget for this stage), `slop` (int ≥ 0, phrase only), `matchingStrategy` (SRCH-080), `boost` (float). Unknown option keys fail with `lql_invalid_option` and position.
- **LQL-053** The trailing `~` fuzzy marker on a word (`"searhc~"`) requests fuzziness per the SRCH-070 budget for that word; `~N` (N ∈ 1..2) forces distance N.

## 7. Reserved keywords

- **LQL-060** Reserved (case-insensitive), usable as identifiers only when back-quoted: `FROM, AS, WHERE, MATCH, IN, OPTIONS, SORT, ASC, DESC, LIMIT, OFFSET, SELECT, EXCEPT, AGGREGATE, BY, HISTOGRAM, TERMS, SIZE, JOIN, LEFT, ON, AND, OR, NOT, IS, NULL, BETWEEN, CONTAINS, TRUE, FALSE, UNNEST, OVER, CAST, INTERVAL, STATS, EVAL, KEEP, DROP, RENAME, DISSECT, GROK, ENRICH`. The tail of the list reserves the planned ES|QL-style verbs (§10) so adopting them later is not a breaking change.
- **LQL-061** Special fields `_score`, `_id`, `_timestamp` are valid in `SORT` and `SELECT`; `_score` in `SORT` follows SRCH-091.

## 8. Errors and position reporting

- **LQL-070** Every LQL error uses the SPEC-003 uniform object with codes `lql_syntax_error`, `lql_type_error`, `lql_invalid_params`, `lql_invalid_option`, `lql_unsupported_feature`, plus kernel codes (SPEC-004 §10, SPEC-007) for post-parse failures.
- **LQL-071** Parse-time and plan-time errors MUST carry a `position` object: `{ "line": int (1-based), "column": int (1-based, chars), "offset": int (0-based, bytes) }` pointing at the first offending token, plus the offending token text in `message`. Multi-error reporting is not required; the first error wins.
- **LQL-072** `lql_unsupported_feature` (LQL-003) MUST name the recognized construct (`"JOIN"`, `"window function"`, `"subquery"`, ...) and carry its position, so tooling can distinguish "not yet" from "typo".

## 9. Aggregation operations — lowering onto SPEC-007

- **LQL-080** `AGGREGATE` functions map 1:1 onto SPEC-007 §4.3: `COUNT()` → `value_count`/bucket `doc_count`, `SUM/AVG/MIN/MAX(f)` → the like-named metrics, `PERCENTILE(f, p)` → `percentiles` with `percents: [p]`, `CARDINALITY(f)` → `cardinality`, `STATS(f)` → `stats`. `AS alias` names the result column; without `AS`, the canonical name is the lower-cased call text (`count()`, `sum(amount)`).
- **LQL-081** `AGGREGATE ... BY f1, f2` lowers to nested `terms` buckets (f1 outer, f2 inner) with the metrics as leaf sub-aggregations; `TERMS f SIZE n` lowers to `terms { field: f, size: n }` (default size 10); `HISTOGRAM f BY 10` lowers to `histogram { interval: 10 }`; `HISTOGRAM ts BY "1h"` lowers to `date_histogram { fixed_interval: "1h" }` — calendar units (`M`, `y`, `w`) lower to `calendar_interval`.
- **LQL-082** All SPEC-007 execution rules apply unchanged: full-docset semantics (AGG-001), fast-field requirement (AGG-060), bucket limits (§5), unsupported types (AGG-070). LQL MUST NOT expose an aggregation the ES/facet doors cannot express.
- **LQL-083** Aggregation-returning responses render buckets as rows: each row is the bucket key column(s) plus one column per aggregate alias — the tabular projection of the SPEC-007 response, with the underlying `aggregations` object available via `"raw": true` in the request body.

## 10. Future extensions (non-normative)

Documented direction, reserved by LQL-060, rejected today per LQL-003/LQL-072:

- **`STATS ... BY`** — ES|QL-inspired alias for `AGGREGATE ... BY` (F-030: `FROM logs | WHERE status >= 500 | STATS count = COUNT(*) BY host | SORT count DESC | LIMIT 10`). Planned as sugar over §9 with ES|QL's `alias = FUNC(...)` assignment spelling.
- **`JOIN` / `LEFT JOIN`** — cross-index joins; blocked on a bounded-cost execution design (ES|QL's `LOOKUP JOIN` is the model).
- **Subqueries in `WHERE ... IN (FROM ...)`**, **window functions (`OVER`)**, **`UNNEST`**, **computed `SELECT` expressions and scalar functions** (string/date/math), **`CAST`**, **query hints (`/*+ ... */`)** — described in [QUERY_LANGUAGE.md](../api/QUERY_LANGUAGE.md); none are part of the conformance surface until specified in a revision of this document.

## 11. Acceptance criteria

1. **Parser conformance**: fixture suite covering every §3.2 production (accept) and a rejection corpus where every error carries correct line/column/offset (LQL-071).
2. **Lowering equivalence**: for a table of paired requests (LQL vs SPEC-004 flat parameters / ES DSL vs SPEC-007 aggs), identical hits, order, totals, scores, and aggregation values (LQL-001).
3. **Filter-context proof**: adding a `WHERE` stage leaves `_score` values byte-identical (LQL-040, mirrors SRCH-041).
4. **Typo/matchingStrategy parity**: `MATCH` with no `IN` behaves identically to the same text in `q` — verified against the SRCH-070 budget table and all three strategies (LQL-050).
5. **Params safety**: `$param` bound to a string containing LQL syntax (`"x\" OR 1=1"`) is matched as a literal value, never parsed (LQL-010).
6. **Future rejection**: `JOIN`, `OVER`, subquery, and `STATS` inputs each return `lql_unsupported_feature` naming the construct with position (LQL-072).
7. **Aggregation rows**: `AGGREGATE ... BY` over the SPEC-007 correctness fixture returns full-docset numbers identical to the equivalent ES-shaped request (LQL-082).
