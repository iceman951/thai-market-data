# Ingestion architecture

```text
MarketDataProvider -> Worker scheduled handler
                   -> R2 raw/<provider>/<dataset>/YYYY/MM/YYYY-MM-DD.json
                   -> deserialize, validate, normalize
                   -> D1 EOD tables + ingestion_log
```

`MarketDataProvider` has one fetch method per dataset and returns the original JSON text. The mock implementation makes the pipeline runnable without choosing a source. A future Yahoo/public or SET OAQ implementation belongs under `src/collector/` and must map its source format into the existing normalized models; D1 and R2 binding code stays separate.

D1 holds normalized daily records keyed by trade date (`report_date` for `security_update`) and natural identity. SQLite stores dates as ISO `TEXT`; nullable source fields use Rust `Option<T>`. Prepared upserts on each table's primary key make same date retries idempotent and leave `created_at` at its first-insert value. `corporate_action` has only a surrogate `id`, so the writer skips a row that exactly matches an existing one (NULLs included); a revised event is stored as a new row. Corporate action dates are not required to equal the run date. A dataset's D1 statements are sent as one batch so a failed statement does not leave half a dataset.

`ingestion_log` is append-only: every attempt adds one row per dataset, labelled with its table name, plus an `all` row with `SUCCESS`, `FAILED`, or `PARTIAL` when some datasets succeeded. Rows record the provider as `source`, UTC `started_at` and `finished_at`, and parsed `row_count`. The latest attempt is the highest `id` for a `trade_date` and `dataset`. The raw archive key is not logged because it is derived from source, dataset, and date.

R2 holds exact source JSON for replay and auditing. A conditional write preserves the first object at each provider, dataset, and date key. A rerun accepts an identical payload; a different payload produces an explicit conflict and stops before normalized writes. Intentional corrections will need a new versioned archive key and an explicit replay procedure, so no source revision is silently overwritten.

The archive happens before deserialization. Invalid JSON or failed validation is still available in R2, and the error is logged in D1 when D1 is available. If D1 itself is unavailable, Cloudflare Worker logs and traces are the remaining error signal. The `workers-rs` scheduled wrapper reports a successful trigger even when ingestion reports `FAILED` or `PARTIAL`; monitor `ingestion_log` and structured error logs for the actual result. `GET /health` checks only that the Worker is serving requests, not D1 or R2 availability.

This service does not calculate returns, volatility, beta, or other analytics. Keeping derived values outside the collector makes the source data reproducible and permits later analytics logic to change without rewriting source history.

## Current schema contract

The migration mirrors the manually created production schema. The models in `src/model/` and the column lists in `src/storage/d1.rs` must match it; each column list and its bind values share a compile-time length. `tests/schema_idempotency.mjs` checks the primary keys the upserts depend on.
