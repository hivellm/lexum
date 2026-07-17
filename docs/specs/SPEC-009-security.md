# SPEC-009 — Security: API Keys, RBAC & Tenant Tokens

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | Phase 7 · tasks 1–4 (`.rulebook/tasks/phase7_security-rbac-tenant-tokens/tasks.md`) |
| **Planning source** | [phase7 proposal](../../.rulebook/tasks/phase7_security-rbac-tenant-tokens/proposal.md); Elastic F-023 (RBAC over API keys, P1 item 8); Meilisearch R-08 / F-019 (tenant tokens); current code `crates/lexum-server/src/middleware/auth.rs`, `crates/lexum-server/src/handlers/auth.rs` |

Requirement IDs `SEC-xxx`. RFC 2119 keywords are normative. Error objects follow the SPEC-003 error contract (`{ message, code, type, link }`); auth is evaluated before the SPEC-002 task queue ever sees a write. The `AuthContext` defined here is a direct input to SPEC-010 (`/multi-search` per-sub-query checks) and SPEC-011 (inter-node authentication).

## 1. Model

The auth middleware runs one pipeline for every request:

```
extract credential → classify (API key | tenant token | none)
  → resolve key record (constant-time)
  → authorize: required (action, index target[s]) from the route→privilege table
  → inject AuthContext into request extensions
  → handler applies tenant forced filters (search paths only)
```

- **SEC-001** Credentials are presented via `Authorization: Bearer <credential>` or `X-API-Key: <credential>`; `X-API-Key` wins when both are present (current behavior, kept). A credential containing exactly two `.` separators is classified as a tenant token (JWT); anything else is classified as an API key.
- **SEC-002** With auth disabled (`LEXUM_AUTH_ENABLED=false`, the default) the pipeline short-circuits to allow-all and injects a synthetic all-access `AuthContext`. Anonymous endpoints (`/health`, `/_ready`, `/swagger-ui*`, `/api-docs*`, `/metrics`) bypass authorization in all modes.
- **SEC-003** Handlers MUST NOT re-parse auth headers. All privilege and tenant-rule information reaches handlers exclusively through `AuthContext` (SEC-040).

## 2. API key model

### 2.1 Key material

- **SEC-010** Raw key format: `lxk_<uid>_<secret>` where `uid` is 10 base62 chars (random, unique, public identifier) and `secret` is 32 base62 chars from a CSPRNG. The `uid` substring equals `ApiKeyRecord.uid`, so the record is addressable from the raw key without a table scan.
- **SEC-011** Keys are stored **hashed at rest**: `key_hash = SHA-256(raw_key)`, lowercase hex. The raw key is returned exactly once, in the creation response; it is never logged, never listable, and never persisted in cleartext.
- **SEC-012** Authentication compares `SHA-256(presented)` against `key_hash` using constant-time equality (`subtle::ConstantTimeEq` or equivalent). String/`HashSet` comparison of key material is forbidden.
- **SEC-013** `AuthConfig::default()` MUST NOT contain any key. The hardcoded `dev-key-12345` / `admin-key-67890` are removed; a release build containing any compiled-in key literal fails CI (grep-level check). When auth is enabled and the key store is empty at boot, the server generates one all-access master key, logs it once at `WARN`, and persists only its hash.
- **SEC-014** Legacy compatibility: each entry of `LEXUM_API_KEYS` is mapped at boot to a record with `actions: ["*"], indexes: ["*"], expires_at: null`, `uid` derived as the first 10 hex chars of its SHA-256. Existing deployments upgrade with zero config changes.

### 2.2 Key record and store

```json
{
  "uid": "a1B2c3D4e5",
  "name": "ingest-service",
  "description": "log shipper for logs-*",
  "keyHash": "<sha256 hex>",
  "actions": ["documents.add", "indexes.create"],
  "indexes": ["logs-*"],
  "expiresAt": "2027-01-01T00:00:00Z",
  "createdAt": "2026-07-17T12:00:00Z"
}
```

- **SEC-015** The `KeyStore` persists records as a JSON file under the data directory, loaded fully into memory at boot and rewritten atomically (temp file + rename) on every change. All verification decisions read only the in-memory registry — no per-request I/O.
- **SEC-016** An expired key (`expiresAt` in the past) behaves exactly like an unknown key (SEC-071); expiry is checked on every request against the server clock.

## 3. Action taxonomy

- **SEC-020** The action set is fixed (closed enumeration). Unknown action names in key creation are rejected with `invalid_api_key_actions`.

| Action | Covers |
|---|---|
| `search` | search, `_msearch`, `/multi-search`, scroll/PIT, suggest, `_count`, `/similar` (SPEC-012) |
| `documents.get` | doc GET, `_mget` |
| `documents.add` | doc index/update, `_bulk`, `_batch` write items, `_reindex` (destination) |
| `documents.delete` | doc delete, delete-by-query |
| `indexes.create` / `indexes.get` / `indexes.update` / `indexes.delete` | index lifecycle, open/close, refresh/flush, aliases, templates, rollover |
| `settings.get` / `settings.update` | index settings & mappings read/write |
| `tasks.get` / `tasks.cancel` | task queue (SPEC-002) read / cancel |
| `snapshots.*` | snapshot repo + snapshot CRUD/restore |
| `keys.*` | key management endpoints (§7) |
| `stats.get` | index/cluster/node stats, `_cat/*`, cluster health/state |
| `network.get` / `network.update` | `GET/PATCH /network` (SPEC-010 §6) |
| `*` | everything, including future actions |

- **SEC-021** Dotted families support the `.*` suffix wildcard (`documents.*`, `snapshots.*`, `keys.*`). `*` alone grants all actions. There is no deny rule — the model is purely additive grants.
- **SEC-022** Index patterns are glob patterns over index names: literal chars, `*` (any run, including empty), `?` (one char). A key matches an index iff **any** pattern in `indexes` matches. `["*"]` matches everything. Patterns never match across the `.`-prefixed internal namespace unless the pattern itself starts with `.`.

## 4. Route → privilege mapping

- **SEC-030** Every route registered in `crates/lexum-server/src/router.rs` MUST have exactly one entry in a declarative table `(method, path pattern) → (required action, index extractor)`. The table is data, not per-handler code, and is the single authorization source of truth. **Fail closed**: a request matching a route with no table entry is rejected 403 in release builds (debug builds panic at startup so the gap is caught in CI).
- **SEC-031** Index extractors are one of: `Path("{index}"|"{name}")` — target named in the path; `Body(...)` — target(s) named per item in the body (`_bulk`, `_mget`, `_msearch`, `/multi-search`, `_aliases`, `_reindex`); `Global` — no index target, action check only (`_cluster/*`, `_tasks`, `/network`, key management, snapshots).
- **SEC-032** Multi-target requests (`Body` extractor) authorize **each sub-target independently**. A failing sub-target produces a per-item SPEC-003 error object in that item's slot (code `insufficient_permissions`) while other items proceed — never all-or-nothing (this is the same contract SPEC-010 FED-032 mandates for query errors).
- **SEC-033** Representative table (normative for these routes; the full table is generated from the audit in phase7 task 1.1):

| Route | Action | Extractor |
|---|---|---|
| `POST /api/v1/indices/{index}/search` | `search` | Path |
| `POST /api/v1/_msearch`, `POST /multi-search` | `search` | Body (per sub-query) |
| `PUT/POST /api/v1/indices/{index}/documents*`, `POST /api/v1/bulk` | `documents.add` | Path / Body |
| `POST /api/v1/indices` | `indexes.create` | Body (index name) |
| `DELETE /api/v1/indices/{name}` | `indexes.delete` | Path |
| `GET/PUT /api/v1/indices/{index}/_mapping` | `settings.get` / `settings.update` | Path |
| `GET /_tasks`, `POST /_tasks/{id}/_cancel` | `tasks.get` / `tasks.cancel` | Global |
| `POST /api/v1/auth/keys` (all key routes) | `keys.*` | Global |
| `GET /_cluster/health`, `_cat/*` | `stats.get` | Global |
| `GET/PATCH /network` | `network.get` / `network.update` | Global |

## 5. AuthContext

- **SEC-040** On successful authentication the middleware inserts into axum request extensions:

```rust
pub struct AuthContext {
    pub key_uid: String,                     // "anonymous" when auth is disabled
    pub actions: Vec<Action>,                // resolved, wildcard-expanded lazily
    pub index_patterns: Vec<GlobPattern>,
    pub tenant_rules: Option<SearchRules>,   // Some(_) iff a tenant token authenticated
}
```

- **SEC-041** `AuthContext::is_allowed(action, index) -> bool` is the only authorization predicate; the middleware and every multi-target handler call it — logic is never duplicated.
- **SEC-042** For SPEC-011 inter-node calls, node credentials are ordinary API keys whose records carry the internal actions required by the transport; nothing in this pipeline is bypassed for cluster traffic.

## 6. Tenant tokens

Stateless, backend-minted HS256 JWTs that narrow a search-capable API key down to per-index forced filters. Lexum never mints tenant tokens; customers' backends do, with any JWT library.

### 6.1 Token format

- **SEC-050** Header `{ "alg": "HS256", "typ": "JWT" }` — `alg` MUST be exactly `HS256`; any other value (including `none`) is rejected before signature verification.
- **SEC-051** Claims:

```json
{
  "apiKeyUid": "a1B2c3D4e5",
  "searchRules": {
    "products": { "filter": "tenant_id = 123" },
    "logs-*":  { "filter": "org = \"acme\" AND region = \"eu\"" },
    "public-docs": null
  },
  "exp": 1789000000
}
```

  `searchRules` maps an index pattern (SEC-022 glob semantics) to a forced-filter object or `null` (access with no forced filter). `exp` (Unix seconds) is REQUIRED — tokens without expiry are rejected (`tenant_token_missing_exp`).
- **SEC-052** Signing secret: `SHA-256(raw_api_key)` as lowercase hex — i.e. exactly the `keyHash` Lexum already stores. The minting backend computes it in one line from the raw key it holds; the server verifies with the stored hash and therefore never needs (and never stores) the raw key. *Deliberate divergence from Meilisearch, which signs with the raw key itself: signing with the hash is the only construction that satisfies both hashed-at-rest (SEC-011) and stateless verification (SEC-054) simultaneously. Trade-off, documented: a leaked key store permits minting search-scoped tenant tokens, but never authenticating as the key itself.*

### 6.2 Verification state machine

- **SEC-053** Ordered checks; the first failure rejects with 403 and the listed code (§8):
  1. decode header, require `alg == HS256` → `tenant_token_invalid`
  2. read `apiKeyUid`, resolve the key record in the in-memory registry → `tenant_token_invalid` if absent
  3. verify HMAC-SHA256 signature against `keyHash` → `tenant_token_invalid_signature`
  4. `exp` present and in the future → `tenant_token_expired`
  5. parent key not expired/revoked and holds `search` (directly or via `*`) → `tenant_token_parent_invalid`
  6. every `searchRules` pattern is a subset of the parent key's `indexes` scope (a rule pattern MUST match only names the parent's patterns could match) → `tenant_token_rule_out_of_scope`
- **SEC-054** Verification is fully stateless: no token registry, no revocation list, no storage lookups beyond the in-memory key registry. Revoking the parent API key instantly invalidates all tokens it signed (step 5); that is the only revocation mechanism.
- **SEC-055** Tenant tokens are honored by **search endpoints only**: `search` (GET/POST), `_msearch`, `/multi-search`, scroll/PIT continuation, `_suggest`, `_count`, and `/similar` (SPEC-012). Every other route presented with a tenant token returns 403 `tenant_token_forbidden_endpoint`, regardless of the parent key's scopes.

### 6.3 Forced-filter application

- **SEC-056** For each queried index, the applicable rule is the **most specific matching pattern** (longest literal prefix wins; exact name beats glob). An index matched by no rule pattern is forbidden to the token (per-sub-query error in multi-search).
- **SEC-057** The forced filter is AND-combined with any client-supplied filter **at the query-tree level** in the executor: `final = forced AND client`. It is applied after parsing, before scoring, on every code path (plain search, `_msearch` sub-queries, federated queries, `/similar`, aggregations/facets over search). No combination of client `q`/`filter`/`sort`/pagination/aggregation parameters can widen the visible document set (property test, §9.2).
- **SEC-058** Forced filters use the same filter grammar as the public `filter` parameter and are parsed at verification time; an unparseable rule rejects the token (`tenant_token_invalid_rules`), never silently drops the restriction.

## 7. Key management API

- **SEC-060** `POST /api/v1/auth/keys` accepts `{ name, description?, actions, indexes, expiresAt? }`, validates actions (SEC-020) and patterns, and returns the record plus `"key": "<raw>"` — the only time the raw key exists in a response.
- **SEC-061** `GET /api/v1/auth/keys` lists records **without** key material (no hash, no raw key). `PATCH /api/v1/auth/keys/{uid}` may change `name`/`description` only; attempts to change `actions`/`indexes`/`expiresAt` return 400 `immutable_api_key_field` directing to rotation (create + revoke). `DELETE` revokes by `uid`, effective immediately for keys and (via SEC-053 step 5) for all tenant tokens they signed.
- **SEC-062** All key-management routes require the `keys.*` action family and are never accessible to tenant tokens.

## 8. Failure semantics (SPEC-003 error contract)

- **SEC-070** All auth failures return the uniform error object with `type: "auth"`. No auth error ever reveals whether a key exists, which check failed inside constant-time comparison, or any stored hash material.

| HTTP | `code` | When |
|---|---|---|
| 401 | `missing_authorization_header` | auth enabled, anonymous disallowed, no credential |
| 403 | `invalid_api_key` | unknown, malformed, or expired API key (SEC-071) |
| 403 | `insufficient_permissions` | key lacks (action, index); also per-item in multi-target bodies |
| 403 | `tenant_token_invalid` / `tenant_token_invalid_signature` / `tenant_token_expired` / `tenant_token_missing_exp` / `tenant_token_parent_invalid` / `tenant_token_rule_out_of_scope` / `tenant_token_invalid_rules` | SEC-053 / SEC-051 / SEC-058 |
| 403 | `tenant_token_forbidden_endpoint` | tenant token on a non-search route (SEC-055) |
| 400 | `invalid_api_key_actions` / `invalid_api_key_indexes` / `immutable_api_key_field` | key management validation (§7) |

- **SEC-071** Unknown and expired keys are indistinguishable to the caller (same code, same message shape, comparable timing).

## 9. Acceptance criteria

1. **Scope matrix test**: a key `actions: ["search"], indexes: ["products"]` → 403 on document write, index create, key listing, and search of `orders`; 200 on search of `products` — including as an `_msearch`/`/multi-search` sub-query where only the offending slot errors (SEC-032).
2. **Tenant isolation property test**: token forcing `user_id = 123` — no combination of adversarial `q`/`filter`/`sort`/pagination/facets returns a document with another `user_id` (SEC-057).
3. **Negative token matrix**: expired token, revoked parent, expired parent, tampered signature, `alg: none`, rule pattern outside parent scope, missing `exp` — each rejected 403 with its distinct code from §8.
4. **Statelessness**: token verification performs zero I/O beyond the in-memory registry (instrumented test); minting works from plain `jsonwebtoken` (Node) and `pyjwt` (Python) snippets with the SEC-052 secret, no Lexum SDK.
5. **Hygiene**: no key literal in release binaries (SEC-013); keys hashed at rest (SEC-011); constant-time compare (SEC-012); `LEXUM_API_KEYS` deployments upgrade unchanged (SEC-014 regression test); full existing suite passes with auth disabled.
