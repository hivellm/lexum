## 1. Audit + structured key model
- [ ] 1.1 Audit the current auth surface: enumerate every route in `crates/lexum-server/src/router.rs` and record which are anonymous, which are protected, and what the natural (action, index) pair is for each — commit the table as the route→privilege spec in `specs/`
- [ ] 1.2 Define `ApiKeyRecord { uid, name, description, key_hash, actions, indexes, expires_at, created_at }` and a persistent `KeyStore` (JSON file under the data dir, loaded at boot, atomic rewrite on change) replacing the `HashSet<String>` in `crates/lexum-server/src/middleware/auth.rs`
- [ ] 1.3 Hash keys at rest (SHA-256) and compare via constant-time equality; raw key material appears only in the creation response
- [ ] 1.4 Map legacy `LEXUM_API_KEYS` env keys to all-access records (`actions: ["*"], indexes: ["*"]`) at boot; add a regression test proving an existing env-key deployment works unchanged
- [ ] 1.5 Remove `dev-key-12345`/`admin-key-67890` from `AuthConfig::default()`; when auth is enabled with zero keys, generate a master key on first boot and log it once

## 2. RBAC enforcement (actions x index patterns)
- [ ] 2.1 Implement the action taxonomy (`search`, `documents.*`, `indexes.*`, `settings.*`, `tasks.*`, `snapshots.*`, `keys.*`, `stats.get`, `*`) and glob index-pattern matching (`logs-*`), with unit tests for pattern precedence and `*` wildcards
- [ ] 2.2 Build the declarative route→(action, index-extractor) table from 1.1 and enforce it in `auth_middleware`; unauthorized requests return 403 with the uniform error object (code `insufficient_permissions`)
- [ ] 2.3 Enforce per-sub-target checks on multi-index paths: `_msearch` and `_mget` in `crates/lexum-server/src/handlers/query_ops.rs` reject only the offending sub-queries (per-item error objects), not the whole request
- [ ] 2.4 Insert `AuthContext { key_uid, actions, index_patterns, tenant_rules }` into request extensions; refactor search handlers to read it instead of re-parsing headers
- [ ] 2.5 Integration tests: a `search`-only/`products`-only key is rejected on document write, index create, key management, and search of `orders` — and accepted on search of `products`

## 3. Tenant tokens (stateless JWT search rules)
- [ ] 3.1 Add `jsonwebtoken` and implement HS256 verification: resolve `apiKeyUid` claim → key record, verify signature with that key's secret, validate `exp`, require the parent key to be alive and hold `search`
- [ ] 3.2 Implement `searchRules` claim parsing: map of index pattern → forced filter object (or null = no filter); reject tokens whose rule patterns exceed the parent key's `indexes` scope
- [ ] 3.3 AND-combine the forced filter with the request `filter` in `crates/lexum-server/src/handlers/search.rs` (and `_msearch`) at the query-tree level, so a client-supplied filter can never widen the result set
- [ ] 3.4 Restrict tenant tokens to search endpoints only: any non-search route presented with a tenant token returns 403 with a distinct error code
- [ ] 3.5 Property-style test: for a token forcing `user_id = 123`, no combination of adversarial `q`/`filter`/`sort`/pagination returns a document with another `user_id`
- [ ] 3.6 Negative tests: expired token, token signed by a revoked key, token signed by an expired key, tampered signature, rule pattern outside parent scope — all 403 with machine-readable codes
- [ ] 3.7 Document (with runnable examples) how a backend mints a token in Node/Python without any Lexum SDK — plain `jsonwebtoken`/`pyjwt` snippets, proving the zero-server-state contract

## 4. Key management API
- [ ] 4.1 Extend `generate_api_key`/`list_api_keys` in `crates/lexum-server/src/handlers/auth.rs` to accept and return `actions`, `indexes`, `expiresAt`; validate action names and patterns at creation
- [ ] 4.2 Add key update (name/description only) and keep revoke; scopes are immutable — attempts to change them return a clear error directing to key rotation
- [ ] 4.3 Guard all key-management routes behind the `keys.*` action; update the OpenAPI spec in `crates/lexum-server/src/openapi.rs`

## 5. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 5.1 Update or create documentation covering the implementation
- [ ] 5.2 Write tests covering the new behavior
- [ ] 5.3 Run tests and confirm they pass
