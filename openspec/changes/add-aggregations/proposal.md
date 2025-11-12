## Why

Lexum needs a rich aggregations framework to perform analytics on search results. This enables users to compute statistics, create histograms, group data, and perform complex analytics operations similar to ElasticSearch aggregations.

## What Changes

- Implement terms aggregation (top values)
- Add stats aggregation (min, max, avg, sum, count)
- Implement histogram aggregation (numeric buckets)
- Add date histogram (time-based buckets)
- Implement percentile aggregation
- Add cardinality aggregation (unique count)
- Implement nested aggregations
- Add pipeline aggregations
- Implement aggregation result merging in distributed mode

## Impact

- Affected specs: `aggregations`
- Affected code: Creates `lexum-core/src/aggregation/`:
  - `terms.rs` - Terms aggregation
  - `stats.rs` - Statistics
  - `histogram.rs` - Histograms
  - `percentile.rs` - Percentiles
  - `cardinality.rs` - Cardinality
  - `pipeline.rs` - Pipeline aggs
- Dependencies: Tantivy aggregations
- Performance target: <100ms for typical aggregations

