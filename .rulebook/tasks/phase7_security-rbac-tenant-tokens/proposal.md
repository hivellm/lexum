# Proposal: phase7_security-rbac-tenant-tokens

## Why

Lexum's authentication today is a flat, all-or-nothing API-key set:
`crates/lexum-server/src/middleware/auth.rs` holds a `HashSet<String>` of
plaintext keys — any valid key can call any of the 39 endpoints on any
index, keys are compared in plaintext (no hashing at rest, no
constant-time compare), two dev keys (`dev-key-12345`, `admin-key-67890`)
ship in `AuthConfig::default()`, and auth is disabled by default. The
handlers in `crates/lexum-server/src/handlers/auth.rs` can generate,
revoke and list keys, but a key carries no scope whatsoever.

Both analyses flag this as the gap between "has auth" and "is a
production platform":

- **Elastic P1 item 8 (F-023)** — roles/RBAC over existing API keys with
  index-pattern privileges is the first production-platform item; ES
  itself made security free in 6.8/7.1 because unsecured clusters were
  an ecosystem-wide liability.
- **Meilisearch R-08 (F-019)** — tenant tokens: backend-mintable JWTs
  signed with an API key, embedding per-index forced filters + expiry,
  honored by the search endpoints only, with **zero server-side state**.
  This is the proven multi-tenant SaaS pattern (row-level security
  without a round trip to the search engine) and it composes directly
  with the scoped keys above.

Sequencing: this must land **before phase9_distributed-clustering**
(inter-node surfaces need a real privilege model) and before phase8's
`remote` federation ships to production (node-to-node search proxying
authenticates with API keys). Phase8's `/multi-search` must enforce
per-index privileges per sub-query, so the `AuthContext` built here is a
direct input to that task.

## What Changes

1. **Structured API keys.** Replace the flat `HashSet<String>` with
   persistent key records: `{ uid, name, description, keyHash, actions[],
   indexes[] (glob patterns), expiresAt, createdAt }`. Keys are stored
   hashed (SHA-256 minimum) and compared in constant time. Legacy flat
   keys from `LEXUM_API_KEYS` keep working, mapped to an all-access
   record (`actions: ["*"], indexes: ["*"]`) — no breaking change.
2. **Action + index-pattern privileges (RBAC, F-023).** A fixed action
   taxonomy (`search`, `documents.get/add/delete`,
   `indexes.create/get/update/delete`, `settings.get/update`,
   `tasks.get/cancel`, `snapshots.*`, `keys.*`, `stats.get`, `*`) and
   glob index patterns (`logs-*`). The auth middleware resolves each
   route to (required action, target index/indices) via a declarative
   route→privilege table and rejects with 403 plus the uniform error
   object. Multi-index paths (`_msearch`, `_mget`, phase8
   `/multi-search`) check every sub-target independently.
3. **`AuthContext` in request extensions.** The middleware inserts a
   resolved `AuthContext { key_uid, actions, index_patterns,
   tenant_rules }` so handlers (search, multi-search) can apply forced
   filters without duplicating auth logic per handler.
4. **Tenant tokens (JWT search rules, R-08).** HS256 JWTs minted by the
   customer's backend — never by Lexum — signed with an API key's
   secret. Claims: `apiKeyUid`, `searchRules` (map of index pattern →
   forced filter object or null), `exp`. Verification is fully
   stateless: resolve the key record by `apiKeyUid`, verify the
   signature against that key, check `exp`, check the parent key is
   alive and has `search`. Honored by **search endpoints only** (search,
   `_msearch`, `/multi-search`, later `/similar`); all other endpoints
   reject tenant tokens. The forced filter is AND-combined server-side
   with any request filter — a client can never widen it.
5. **Key management API upgrade.** Extend the key handlers to
   accept/return `actions`, `indexes`, `expiresAt`; return the raw key
   only once at creation; allow updating name/description only (scopes
   are immutable — rotate instead). Remove hardcoded dev keys from
   defaults (a key is generated and logged on first boot when auth is
   enabled with no keys configured).

## Impact

- Affected specs: `.rulebook/tasks/phase7_security-rbac-tenant-tokens/specs/`
  (RBAC privilege model, tenant-token contract)
- Affected code:
  - `crates/lexum-server/src/middleware/auth.rs` (key model, RBAC
    enforcement, AuthContext, JWT verification)
  - `crates/lexum-server/src/handlers/auth.rs` (key CRUD with scopes)
  - `crates/lexum-server/src/handlers/search.rs` and
    `crates/lexum-server/src/handlers/query_ops.rs` (forced-filter
    application, per-sub-query index checks)
  - `crates/lexum-server/src/router.rs` (route→privilege table wiring)
  - `crates/lexum-server/Cargo.toml` (add `jsonwebtoken`, `sha2`,
    `subtle` or equivalents)
- Breaking change: NO (auth stays disabled by default; legacy env keys
  map to all-access; existing key endpoints extended additively)
- User benefit: production-grade least-privilege keys plus zero-state
  multi-tenant search — SaaS builders can hand browsers a token that can
  only ever see its own tenant's rows, without proxying every search.

## Success criteria

- A key with `actions: ["search"], indexes: ["products"]` gets 403 on
  document writes, index admin, and on searching any non-matching index
  — including as a sub-query of `_msearch` (integration tests).
- A tenant token embedding `user_id = 123` never returns a document with
  a different `user_id`, even when the request supplies adversarial
  `filter`/`q` parameters (property-style test over combined filters).
- Expired tokens, tokens signed by a revoked or expired parent key, and
  tenant tokens presented to non-search endpoints are all rejected with
  403 and a machine-readable error code.
- Token verification performs zero storage lookups beyond the in-memory
  key registry (no per-token server state exists).
- Hardcoded dev keys are gone from `AuthConfig::default()`; keys are
  stored hashed; key comparison is constant-time.
- Existing deployments using `LEXUM_API_KEYS` upgrade with zero config
  changes (regression test), and the full existing test suite passes.
