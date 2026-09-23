# Ingestion architecture

```text
provider.fetch(dataset, date) -> raw source string
    |
    +--> R2 raw/<provider>/<dataset>/YYYY/MM/YYYY-MM-DD.json   (exact bytes)
    |
    v
provider.normalize_<dataset>(raw) -> canonical models    (source-specific)
    |
    v
validate canonical rows                                  (same for every provider)
    |
    v
D1 EOD tables, in batches of 100 -> ingestion_log
```

`MarketDataProvider` (`src/collector/provider.rs`) has two jobs. `fetch` returns the source response exactly as received, and ingestion archives that string unchanged. One `normalize_*` method per dataset maps the same string into a canonical model from `src/model/`. The canonical models are the D1 contract and carry no source field names; each source's own structs and field mapping (for example `tradeDate` to `trade_date`, or `last` to `last_traded_price`) live in its module, such as a future `src/collector/set.rs`. The ingestion layer never parses a payload itself: it calls the provider's normalizer, then validates the canonical rows with the same rules for every source. `MockProvider` (`src/collector/mock.rs`) makes the pipeline runnable without a real source; its payloads happen to use canonical field names, but they still go through its normalizers.

D1 holds normalized daily records keyed by trade date (`report_date` for `security_update`) and natural identity. SQLite stores dates as ISO `TEXT`; nullable source fields use Rust `Option<T>`. Prepared upserts on each table's primary key make same date retries idempotent and leave `created_at` at its first-insert value. `corporate_action` has only a surrogate `id`, so the writer skips a row that exactly matches an existing one (NULLs included); a revised event is stored as a new row. Corporate action dates are not required to equal the run date. Rows are written in D1 batches of 100 statements, with statements prepared one batch at a time. Each batch is atomic, but a dataset larger than one batch is not: if a later batch fails, earlier batches stay written, the error stops the dataset at once, and `ingestion_log` records it as `FAILED`. Rerunning the date is safe because every EOD write is an upsert or an insert-if-absent, so the rerun converges on one complete copy.

`ingestion_log` is append-only: every attempt adds one row per dataset, labelled with its table name, plus an `all` row with `SUCCESS`, `FAILED`, or `PARTIAL` when some datasets succeeded. Rows record the provider as `source`, UTC `started_at` and `finished_at`, and parsed `row_count`. The latest attempt is the highest `id` for a `trade_date` and `dataset`. The raw archive key is not logged because it is derived from source, dataset, and date.

R2 holds exact source JSON for replay and auditing. A conditional write preserves the first object at each provider, dataset, and date key. A rerun accepts an identical payload; a different payload produces an explicit conflict and stops before normalized writes. Intentional corrections will need a new versioned archive key and an explicit replay procedure, so no source revision is silently overwritten.

The archive happens before normalization. A payload that fails to parse or validate is still available in R2, and the error is logged in D1 when D1 is available. If D1 itself is unavailable, Cloudflare Worker logs and traces are the remaining error signal. The `workers-rs` scheduled wrapper reports a successful trigger even when ingestion reports `FAILED` or `PARTIAL`; monitor `ingestion_log` and structured error logs for the actual result. `GET /health` checks only that the Worker is serving requests, not D1 or R2 availability.

This service does not calculate returns, volatility, beta, or other analytics. Keeping derived values outside the collector makes the source data reproducible and permits later analytics logic to change without rewriting source history.

## Current schema contract

The migration defines the intended local schema; the manually created production schema still needs comparison before remote migration or deployment. The models in `src/model/` and the column lists in `src/storage/d1.rs` must match the live schema; each column list and its bind values share a compile-time length. `tests/schema_idempotency.mjs` checks the local primary keys the upserts depend on, and a Rust unit test in `src/storage/d1.rs` fails if a writer's columns or key drift from the migration.
