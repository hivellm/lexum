# SPEC-003 — Uniform Error Contract

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | phase1_write-path-task-queue · [tasks §5](../../.rulebook/tasks/phase1_write-path-task-queue/tasks.md) |
| **Planning source** | [phase1 proposal](../../.rulebook/tasks/phase1_write-path-task-queue/proposal.md) §What Changes #6; [Meilisearch execution plan](../analysis/meilisearch/08-execution-plan.md) R-02 (motivated by F-027) |

Requirement IDs `ERR-xxx`. RFC 2119 keywords are normative. The task queue that embeds this object in task records is [SPEC-002](SPEC-002-write-path-task-queue.md); per-item bulk errors that reuse it are [SPEC-005](SPEC-005-documents-and-bulk.md) §4; the architectural rule that all protocols share it is [SPEC-001](SPEC-001-architecture.md) ARC-012.

## 1. The error object

Every error response body, on every endpoint and every protocol adapter, is exactly:

```json
{
  "message": "Index `products` not found.",
  "code": "index_not_found",
  "type": "invalid_request",
  "link": "https://docs.lexum.dev/errors#index_not_found"
}
```

- **ERR-001** All four fields are REQUIRED and non-null on every error body. No other top-level fields are permitted (forward-compatible additions require a spec revision; clients MAY ignore unknown fields defensively but the server MUST NOT emit any today).
- **ERR-002** `message` — human-readable, English, sentence-cased, ends with a period, safe to display. It MUST NOT contain stack traces, internal file paths, or secrets. It MAY interpolate user-supplied identifiers (index names, document ids) in backticks.
- **ERR-003** `code` — machine-readable identifier from the registry (§4). Clients branch on `code`; `message` wording MAY change between releases, `code` MUST NOT.
- **ERR-004** `type` — coarse classification, exactly one of the §3 enum. Clients that don't know a `code` fall back to `type`.
- **ERR-005** `link` — `https://docs.lexum.dev/errors#<code>` (the anchor is the `code`, guaranteeing every code has a stable docs anchor).
- **ERR-006** The same object shape is embedded verbatim as: the task `error` field (SPEC-002 TSK-015), bulk per-item `error` values (SPEC-005 DOC-043 — there rendered in the ES envelope alongside `status`), and multi-search per-query errors (SPEC-001 ARC-033).

## 2. Scope

- **ERR-010** New endpoints MUST NOT invent new error shapes. An endpoint that cannot express its failure in this object gets a new registered `code`, never a new body layout.
- **ERR-011** The contract applies to every non-2xx JSON response produced by `lexum-server`, including middleware rejections (auth, rate limit, request size, content type) and axum extractor rejections (malformed JSON). Only responses that never carry a body (e.g. `304`) are exempt.
- **ERR-012** Acceptance (phase1 gate 5.5): an endpoint-walking contract test triggers at least one error on every registered route and asserts the body deserializes exactly to `{ message, code, type, link }`; zero endpoints emit the legacy shape.

## 3. `type` enum

| `type` | Meaning | Typical HTTP classes |
|---|---|---|
| `invalid_request` | The caller can fix it: bad syntax, unknown resource, conflict, unsupported parameter | 400, 404, 405, 409, 411, 413, 415 |
| `auth` | Authentication or authorization failure | 401, 403 |
| `system` | The system refuses or cannot serve right now: resource limits, timeouts, overload, unavailability | 408, 429, 503, 504 |
| `internal` | A bug or unexpected engine failure; the caller cannot fix it | 500, 502 |

- **ERR-020** The enum is closed at these four values; adding a value is a breaking spec revision. Resource-limit conditions (queue full, rate limited, payload too large at the limiter) classify as `system` except statically-checkable request-size violations, which are `invalid_request`.

## 4. Code registry

- **ERR-030** Naming rules: `snake_case` ASCII, singular nouns/adjectives, no abbreviations (`document`, not `doc`), pattern `[a-z][a-z0-9_]*`. Prefer `<subject>_<problem>` (`index_not_found`, `invalid_document_id`). A code, once released, is permanent: never renamed, never reused for a different meaning, never removed (only deprecated in docs).
- **ERR-031** Every code MUST be registered in one place in the codebase (single source for `code` → `type` + default HTTP status), and every registered code MUST have a docs section at its `link` anchor. Ad-hoc `ApiError::Internal(format!(...))` call sites MUST be mapped to registered codes (phase1 tasks.md 5.2).
- **ERR-032** Initial registry (append-only; statuses per §5):

| `code` | `type` | HTTP |
|---|---|---|
| `index_not_found` | invalid_request | 404 |
| `index_already_exists` | invalid_request | 409 |
| `invalid_index_uid` | invalid_request | 400 |
| `document_not_found` | invalid_request | 404 |
| `invalid_document_id` | invalid_request | 400 |
| `invalid_document_fields` | invalid_request | 400 |
| `document_too_large` | invalid_request | 413 |
| `template_not_found` | invalid_request | 404 |
| `alias_not_found` | invalid_request | 404 |
| `task_not_found` | invalid_request | 404 |
| `batch_not_found` | invalid_request | 404 |
| `invalid_task_filter` | invalid_request | 400 |
| `missing_task_filters` | invalid_request | 400 |
| `task_canceled` | system | — (task-embedded only) |
| `task_queue_full` | system | 429 |
| `version_conflict` | invalid_request | 409 |
| `invalid_search_query` | invalid_request | 400 |
| `invalid_query_parameter` | invalid_request | 400 |
| `bad_request` | invalid_request | 400 |
| `payload_too_large` | invalid_request | 413 |
| `unsupported_media_type` | invalid_request | 415 |
| `malformed_payload` | invalid_request | 400 |
| `missing_authorization_header` | auth | 401 |
| `invalid_api_key` | auth | 403 |
| `insufficient_permissions` | auth | 403 |
| `rate_limit_exceeded` | system | 429 |
| `request_timeout` | system | 408 |
| `timeout` | system | 504 |
| `service_unavailable` | system | 503 |
| `network_error` | internal | 502 |
| `internal` | internal | 500 |

- **ERR-033** `internal` (the code) is the last-resort catch-all for unmapped engine errors; emitting it MUST also log the underlying error with a correlation id at `error` level. Shipping a feature whose failures routinely surface as `internal` is a spec violation — register a real code.

## 5. HTTP status mapping

- **ERR-040** Each `code` has exactly one canonical HTTP status (ERR-032 table). Handlers MUST use the registry's status; per-call-site overrides are forbidden (a different status means a different code).
- **ERR-041** Consistency rule between `type` and status: `invalid_request` → 4xx; `auth` → 401/403; `system` → 408/429/5xx; `internal` → 500/502. A registered code violating this mapping fails CI (registry unit test).
- **ERR-042** Errors embedded in task records or bulk items carry no top-level HTTP status of their own; SPEC-005 items expose the canonical status in the item's `status` field, and task fetches return `200` with the error inside the task.

## 6. Migration from `ErrorResponse { error, details }`

Current shape (`crates/lexum-server/src/error.rs`): `{ "error": "<Display string>", "details": "<optional>" }`, produced by `ApiError::to_response()`, with JSON-rejection details string-concatenated into `message`.

- **ERR-050** `ErrorResponse { error, details }` is REMOVED, not aliased — no endpoint may emit it after phase1 (single hard break while pre-1.0, per the phase1 proposal). Mapping: old `error` string → `message`; old `details` content is folded into `message` (line/column/field info from JSON rejections becomes part of the message text); each `ApiError` variant maps to a registered `code` (`IndexNotFound`→`index_not_found`, `Validation`/`InvalidRequest`→`invalid_document_fields`/`bad_request`/`invalid_query_parameter` as appropriate per call site, `Serialization`→`malformed_payload`, `RateLimitExceeded`→`rate_limit_exceeded`, `Authentication`→`missing_authorization_header`/`invalid_api_key`, `Authorization`→`insufficient_permissions`, `Timeout`→`request_timeout`, `Network`→`network_error`, `Core`/`Internal`/`Configuration`→`internal` unless a finer code applies).
- **ERR-051** `src/openapi.rs` error schemas MUST be regenerated to the new object; the CHANGELOG MUST document the break with before/after examples.
- **ERR-052** The status-code behavior of existing endpoints is preserved except where the ERR-032 table corrects an inconsistency; any such status change MUST be listed in the CHANGELOG.
