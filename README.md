# Thai market EOD collector

Rust Cloudflare Worker foundation for collecting daily Thai capital market data. It currently uses **synthetic `MockProvider` records only**. No real market provider, historical backfill, public data API, analytics, or frontend is included.

## Architecture

The scheduled handler runs six dataset fetches. Each one archives the provider's exact JSON in R2, deserializes and validates it, writes normalized rows to D1, and records an ingestion result. `GET /health` is the only HTTP route. See [architecture](docs/architecture.md).

## Prerequisites and local setup

- Rust and the `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)
- Node.js 22.13+ and npm (the schema check uses built-in `node:sqlite`)
- `cargo install worker-build --version 0.8.6 --locked`
- `npm ci` for project-local Wrangler

On Windows PowerShell systems that block `.ps1` shims, use `npm.cmd` and `npx.cmd` in the commands below. If `worker-build` cannot download esbuild on Windows, run `$env:ESBUILD_BIN = (Resolve-Path 'node_modules\@esbuild\win32-x64\esbuild.exe').Path` before Wrangler.

Set the existing D1 `database_id` and an R2 `bucket_name` in `wrangler.jsonc`. The bindings used by the code are `DB` and `RAW_BUCKET`. Create a bucket if needed with `npx wrangler r2 bucket create <bucket-name>`. No provider secrets are used. The default config has **no cron**; add a reviewed UTC expression to `triggers.crons` only when a real provider and schedule are ready.

### Schema

`migrations/0001_initial.sql` defines the intended local tables, keys, and indexes. The live D1 DDL has not been verified in this repository. Compare the live columns, types, keys, and indexes against the migration, models in `src/model/`, and writes in `src/storage/d1.rs` before applying it remotely. `CREATE ... IF NOT EXISTS` does not detect differences in existing tables. To inspect the live database:

```sh
npx wrangler d1 execute thai-market-eod --remote --command "SELECT name, sql FROM sqlite_master WHERE type IN ('table','index') AND name NOT LIKE 'sqlite_%' ORDER BY name"
```

The configured D1 database ID must be the existing `thai-market-eod` database, not a new one.
Noninteractive remote Wrangler commands require Cloudflare credentials such as `CLOUDFLARE_API_TOKEN`; keep credentials outside this repository.

### Local commands

```sh
npx wrangler d1 migrations apply DB --local
npx wrangler dev
curl http://localhost:8787/health
curl http://localhost:8787/cdn-cgi/local/scheduled
```

The local scheduled request writes synthetic rows to **local** D1 and R2. Repeating it on the same UTC date keeps one normalized row per mock identity, reuses identical raw archives, and appends new `ingestion_log` rows. If local D1 was created from an older migration, delete `.wrangler/state/v3/d1` and apply the migration again. Use local bindings when testing; do not set remote binding options for the mock run.
Inspect `ingestion_log` after a scheduled run: the `workers-rs` scheduled wrapper can return `ok` even when ingestion records `FAILED` or `PARTIAL`.

```sh
cargo fmt --all
cargo check
cargo test
node tests/schema_idempotency.mjs
```

### Deployment, after schema and provider review

```sh
npx wrangler d1 migrations apply DB --remote
npx wrangler deploy --dry-run
npx wrangler deploy
```

The remote migration command is deliberately separate from local setup. Deployment with the current empty cron does not run ingestion automatically. `MockProvider` writes synthetic data if the scheduled handler is manually invoked; replace it with a real provider before scheduling production collection.
