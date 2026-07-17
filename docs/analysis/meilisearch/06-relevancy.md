# 6. Relevancy Model

> Part of the [Meilisearch analysis](README.md) · Previous: [§5 Indexing and Storage](05-indexing-storage.md) · Next: [§7 Parity Matrix](07-parity-matrix.md)

## Bucket sort over ranking rules

Meilisearch does **not** use a single scalar score formula (like BM25) as its primary mechanism. Instead it applies **ranking rules in order as successive tiebreakers** ("bucket sort"): rule 1 partitions candidates into ordered groups; rule 2 orders within each group; and so on. Once a rule separates two documents, later rules can never reorder them ([ranking rules docs](https://www.meilisearch.com/docs/learn/relevancy/ranking_rules)).

**F-034 — Meilisearch's primary relevance mechanism is not a scalar score formula (like BM25) but a bucket sort: ranking rules applied in order as successive tiebreakers, where once a rule separates two documents, later rules can never reorder them**
- Evidence: https://www.meilisearch.com/docs/learn/relevancy/ranking_rules
- Impact: Explains why Meilisearch relevance is zero-config and tunable without boosting arithmetic. Lexum (BM25 via Tantivy) need not adopt full bucket sort, but should adopt the explainability and normalization it enables (F-036).
- Confidence: high

## Default rules (current, in order)

Per the current settings reference ([settings API](https://www.meilisearch.com/docs/reference/api/settings)):

```json
["words", "typo", "proximity", "attributeRank", "sort", "wordPosition", "exactness"]
```

> Historical note: for most of 1.x the default was six rules — `["words", "typo", "proximity", "attribute", "sort", "exactness"]` — where `attribute` combined field importance and word position. Recent versions split it into `attributeRank` (field importance only) and `wordPosition` (position within the field), placing `wordPosition` after `sort`. Track this when reading older articles.

1. **`words`** — documents matching *more* of the query terms rank first (terms are dropped from the end per `matchingStrategy`). Special: results are always sorted as if `words` were first, regardless of its position.
2. **`typo`** — fewer typos ranks higher (exact spellings beat fuzzy matches).
3. **`proximity`** — smaller distance between matched terms ranks higher (phrase-like matches win).
4. **`attributeRank`** — matches in more important attributes rank higher; importance = the order of `searchableAttributes` (first = most important).
5. **`sort`** — applies query-time `sort` parameter; no-op otherwise. Its *position* controls whether it acts as a relevance tiebreaker (late) or dominates relevance (early).
6. **`wordPosition`** — matches earlier within an attribute rank higher.
7. **`exactness`** — exact word matches (no prefix/stem expansion) beat partial ones.
8. **Custom rules** — `field:asc` / `field:desc` entries (e.g., `popularity:desc`) can be inserted anywhere; recommended at the end as final tiebreakers.

**F-035 — The current default ranking-rule order is `["words", "typo", "proximity", "attributeRank", "sort", "wordPosition", "exactness"]`; recent versions split the historical `attribute` rule into `attributeRank` (field importance, from `searchableAttributes` order) and `wordPosition` (position within the field), and custom `field:asc/desc` rules can be inserted anywhere**
- Evidence: https://www.meilisearch.com/docs/reference/api/settings · https://www.meilisearch.com/docs/learn/relevancy/ranking_rules (historical six-rule default: `["words", "typo", "proximity", "attribute", "sort", "exactness"]` — track this when reading older articles)
- Impact: The default order encodes universally sensible priorities (completeness > spelling > cohesion > field importance > position > exactness) — a proven reference ordering if Lexum builds a rules-based layer, and the `searchableAttributes`-order-as-field-boost idea transfers directly.
- Confidence: high

## Why this model wins for site search

- **Explainable**: `showRankingScoreDetails: true` returns a per-rule score breakdown for every hit — relevance debugging is a query parameter, not a `_explain` expedition.
- **Zero-config**: the default order encodes universally sensible priorities (completeness > spelling > cohesion > field importance > position > exactness).
- **Tunable without math**: reorder/remove rules, reorder `searchableAttributes`, add `field:desc` — no boosting arithmetic, no function_score scripting.
- **Normalized score**: `showRankingScore` yields a 0.0–1.0 score usable across queries; `rankingScoreThreshold` filters low-quality tail results — this normalization is also what makes **federated merging and hybrid keyword+semantic fusion** possible (BM25 scores from different indexes are not comparable; Meilisearch's normalized rule-based scores are).

**F-036 — Meilisearch exposes a normalized 0.0–1.0 ranking score (`showRankingScore`) with per-rule breakdown (`showRankingScoreDetails`) and a `rankingScoreThreshold`; the normalization is what makes federated merging and hybrid keyword+semantic fusion possible, since raw BM25 scores from different indexes are not comparable**
- Evidence: https://www.meilisearch.com/docs/learn/relevancy/ranking_rules · https://www.meilisearch.com/docs/reference/api/search
- Impact: Foundational prerequisite for Lexum's federation, sharding, and hybrid-search plans: even with BM25 underneath, Lexum must normalize scores (e.g., against the theoretical max or top hit) before any cross-index merging works. Score explainability as a query flag is also a major DX differentiator vs Elasticsearch's `_explain`.
- Confidence: high

## Supporting relevancy machinery

- Synonyms (one-way and multi-way; v1.49 made large synonym sets 13× faster via lazy loading — [releases](https://github.com/meilisearch/meilisearch/releases)), stop words, `distinctAttribute` (dedup), `proximityPrecision` (`byWord` exact vs `byAttribute` cheaper), `separatorTokens`/`nonSeparatorTokens`/`dictionary` for tokenizer customization, `localizedAttributes` + `locales` for language pinning, `stopWords`.

**F-037 — Relevancy is supported by a settings-level machinery layer: one-way and multi-way synonyms (v1.49 made large synonym sets 13× faster via lazy loading), stop words, `distinctAttribute` dedup, `proximityPrecision` (`byWord` exact vs `byAttribute` cheaper), tokenizer customization (`separatorTokens`/`nonSeparatorTokens`/`dictionary`), and `localizedAttributes` + `locales` language pinning**
- Evidence: https://www.meilisearch.com/docs/reference/api/settings · https://github.com/meilisearch/meilisearch/releases (v1.49)
- Impact: Lexum currently lacks synonyms, stop words, and distinct-attribute dedup entirely (parity matrix row 32); these belong in the settings-as-resource work (see [execution plan](08-execution-plan.md), R-03).
- Confidence: high
