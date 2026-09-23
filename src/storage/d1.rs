use crate::model::{
    corporate_action::CorporateAction, index::IndexStat, investor::InvestorStat,
    market::MarketStat, security::SecurityStat, security_update::SecurityUpdate, IngestionLog,
};
use worker::{wasm_bindgen::JsValue, D1Database, Error, Result};

fn text(value: &str) -> JsValue {
    JsValue::from_str(value)
}
fn maybe_text(value: &Option<String>) -> JsValue {
    value.as_deref().map(text).unwrap_or(JsValue::NULL)
}
fn number(value: Option<f64>) -> JsValue {
    value.map(JsValue::from_f64).unwrap_or(JsValue::NULL)
}
fn integer(value: Option<i64>) -> JsValue {
    value
        .map(|v| JsValue::from_f64(v as f64))
        .unwrap_or(JsValue::NULL)
}

// created_at is omitted everywhere so D1 keeps its first-insert default.
fn upsert_sql(table: &str, columns: &[&str], key: &[&str]) -> String {
    let updates = columns
        .iter()
        .filter(|c| !key.contains(c))
        .map(|c| format!("{c}=excluded.{c}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "INSERT INTO {table} ({}) VALUES ({}) ON CONFLICT({}) DO UPDATE SET {updates}",
        columns.join(","),
        vec!["?"; columns.len()].join(","),
        key.join(",")
    )
}

// For tables without a natural key: skip a row identical to an existing one.
// IS matches NULL to NULL, which = would not.
fn insert_absent_sql(table: &str, columns: &[&str]) -> String {
    let params = (1..=columns.len())
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>();
    let matches = columns
        .iter()
        .zip(&params)
        .map(|(c, p)| format!("{c} IS {p}"))
        .collect::<Vec<_>>()
        .join(" AND ");
    format!(
        "INSERT INTO {table} ({}) SELECT {} WHERE NOT EXISTS (SELECT 1 FROM {table} WHERE {matches})",
        columns.join(","),
        params.join(",")
    )
}

// N ties each column list to its bind array, so a missing value fails to compile.
async fn write_rows<T, const N: usize>(
    db: &D1Database,
    sql: &str,
    rows: &[T],
    bind: impl Fn(&T) -> [JsValue; N],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let statements = rows
        .iter()
        .map(|r| db.prepare(sql).bind(&bind(r)))
        .collect::<Result<Vec<_>>>()?;
    // One D1 batch keeps a dataset atomic if any statement fails.
    for result in db.batch(statements).await? {
        if !result.success() {
            return Err(Error::RustError(
                result.error().unwrap_or_else(|| "D1 batch failed".into()),
            ));
        }
    }
    Ok(())
}

async fn upsert<T, const N: usize>(
    db: &D1Database,
    table: &str,
    columns: [&str; N],
    key: &[&str],
    rows: &[T],
    bind: impl Fn(&T) -> [JsValue; N],
) -> Result<()> {
    write_rows(db, &upsert_sql(table, &columns, key), rows, bind).await
}

async fn insert_absent<T, const N: usize>(
    db: &D1Database,
    table: &str,
    columns: [&str; N],
    rows: &[T],
    bind: impl Fn(&T) -> [JsValue; N],
) -> Result<()> {
    write_rows(db, &insert_absent_sql(table, &columns), rows, bind).await
}

pub async fn upsert_security_stat(db: &D1Database, rows: &[SecurityStat]) -> Result<()> {
    upsert(
        db,
        "security_stat",
        [
            "trade_date",
            "market_code",
            "orderbook_id",
            "symbol",
            "market_segment",
            "trading_currency",
            "financial_product",
            "sector_code",
            "previous_close",
            "open",
            "high",
            "low",
            "last_traded_price",
            "average_price",
            "best_bid",
            "best_offer",
            "turnover_quantity",
            "turnover_value",
            "total_trade",
            "trade_report_quantity",
            "trade_report_value",
            "short_sell_quantity",
            "short_sell_value",
            "par",
            "lot_size",
            "listed_shares",
            "book_value",
            "eps",
            "dps",
            "pe",
            "pbv",
            "dividend_yield",
            "market_cap",
            "corporate_action_code",
            "notification_sign",
            "other_sign",
            "isin",
            "isin_nvdr",
        ],
        &["trade_date", "market_code", "orderbook_id"],
        rows,
        |r| {
            [
                text(&r.trade_date),
                text(&r.market_code),
                integer(Some(r.orderbook_id)),
                text(&r.symbol),
                maybe_text(&r.market_segment),
                maybe_text(&r.trading_currency),
                maybe_text(&r.financial_product),
                maybe_text(&r.sector_code),
                number(r.previous_close),
                number(r.open),
                number(r.high),
                number(r.low),
                number(r.last_traded_price),
                number(r.average_price),
                number(r.best_bid),
                number(r.best_offer),
                number(r.turnover_quantity),
                number(r.turnover_value),
                integer(r.total_trade),
                number(r.trade_report_quantity),
                number(r.trade_report_value),
                number(r.short_sell_quantity),
                number(r.short_sell_value),
                number(r.par),
                integer(r.lot_size),
                integer(r.listed_shares),
                number(r.book_value),
                number(r.eps),
                number(r.dps),
                number(r.pe),
                number(r.pbv),
                number(r.dividend_yield),
                number(r.market_cap),
                maybe_text(&r.corporate_action_code),
                maybe_text(&r.notification_sign),
                maybe_text(&r.other_sign),
                maybe_text(&r.isin),
                maybe_text(&r.isin_nvdr),
            ]
        },
    )
    .await
}

pub async fn upsert_index_stat(db: &D1Database, rows: &[IndexStat]) -> Result<()> {
    upsert(
        db,
        "index_stat",
        [
            "trade_date",
            "data_round",
            "market_segment",
            "industry_code",
            "sector_code",
            "index_name",
            "index_value",
            "previous_close",
            "open",
            "high",
            "low",
            "change",
            "volume",
            "value",
            "pe",
            "pbv",
            "dividend_yield",
            "market_cap",
        ],
        &["trade_date", "index_name"],
        rows,
        |r| {
            [
                text(&r.trade_date),
                maybe_text(&r.data_round),
                maybe_text(&r.market_segment),
                maybe_text(&r.industry_code),
                maybe_text(&r.sector_code),
                text(&r.index_name),
                number(r.index_value),
                number(r.previous_close),
                number(r.open),
                number(r.high),
                number(r.low),
                number(r.change),
                number(r.volume),
                number(r.value),
                number(r.pe),
                number(r.pbv),
                number(r.dividend_yield),
                number(r.market_cap),
            ]
        },
    )
    .await
}

pub async fn upsert_market_stat(db: &D1Database, rows: &[MarketStat]) -> Result<()> {
    upsert(
        db,
        "market_stat",
        [
            "trade_date",
            "data_round",
            "market_stat_id",
            "trading_currency",
            "total_trades",
            "total_quantity",
            "total_value",
            "up_quantity",
            "down_quantity",
            "no_change_quantity",
            "up_shares",
            "down_shares",
            "no_change_shares",
        ],
        &["trade_date", "market_stat_id", "trading_currency"],
        rows,
        |r| {
            [
                text(&r.trade_date),
                maybe_text(&r.data_round),
                text(&r.market_stat_id),
                text(&r.trading_currency),
                integer(r.total_trades),
                integer(r.total_quantity),
                number(r.total_value),
                integer(r.up_quantity),
                integer(r.down_quantity),
                integer(r.no_change_quantity),
                integer(r.up_shares),
                integer(r.down_shares),
                integer(r.no_change_shares),
            ]
        },
    )
    .await
}

pub async fn upsert_investor_stat(db: &D1Database, rows: &[InvestorStat]) -> Result<()> {
    upsert(
        db,
        "investor_stat",
        [
            "trade_date",
            "data_round",
            "period_type",
            "market_segment",
            "trading_currency",
            "buy_institute_value",
            "buy_foreign_value",
            "buy_customer_value",
            "buy_proprietary_value",
            "sell_institute_value",
            "sell_foreign_value",
            "sell_customer_value",
            "sell_proprietary_value",
            "total_value",
        ],
        &[
            "trade_date",
            "period_type",
            "market_segment",
            "trading_currency",
        ],
        rows,
        |r| {
            [
                text(&r.trade_date),
                maybe_text(&r.data_round),
                text(&r.period_type),
                text(&r.market_segment),
                text(&r.trading_currency),
                number(r.buy_institute_value),
                number(r.buy_foreign_value),
                number(r.buy_customer_value),
                number(r.buy_proprietary_value),
                number(r.sell_institute_value),
                number(r.sell_foreign_value),
                number(r.sell_customer_value),
                number(r.sell_proprietary_value),
                number(r.total_value),
            ]
        },
    )
    .await
}

pub async fn upsert_security_update(db: &D1Database, rows: &[SecurityUpdate]) -> Result<()> {
    upsert(
        db,
        "security_update",
        [
            "report_date",
            "data_round",
            "market_code",
            "market_segment",
            "trading_currency",
            "orderbook_id",
            "symbol",
            "long_name",
            "originates_from",
            "financial_product",
            "sector_code",
            "pqf",
            "par",
            "lot_size",
            "isin",
            "isin_nvdr",
            "instrument_state",
            "instrument_status",
        ],
        &["report_date", "market_code", "orderbook_id"],
        rows,
        |r| {
            [
                text(&r.report_date),
                maybe_text(&r.data_round),
                text(&r.market_code),
                maybe_text(&r.market_segment),
                maybe_text(&r.trading_currency),
                integer(Some(r.orderbook_id)),
                text(&r.symbol),
                maybe_text(&r.long_name),
                maybe_text(&r.originates_from),
                maybe_text(&r.financial_product),
                maybe_text(&r.sector_code),
                number(r.pqf),
                number(r.par),
                integer(r.lot_size),
                maybe_text(&r.isin),
                maybe_text(&r.isin_nvdr),
                maybe_text(&r.instrument_state),
                maybe_text(&r.instrument_status),
            ]
        },
    )
    .await
}

pub async fn insert_corporate_action(db: &D1Database, rows: &[CorporateAction]) -> Result<()> {
    insert_absent(
        db,
        "corporate_action",
        [
            "symbol",
            "orderbook_id",
            "event_type",
            "announcement_date",
            "ex_date",
            "record_date",
            "payment_date",
            "effective_date",
            "cash_dividend",
            "stock_dividend_ratio",
            "split_ratio",
            "rights_ratio",
            "rights_price",
            "old_par",
            "new_par",
            "old_symbol",
            "new_symbol",
            "currency",
            "source",
        ],
        rows,
        |r| {
            [
                text(&r.symbol),
                integer(r.orderbook_id),
                text(&r.event_type),
                maybe_text(&r.announcement_date),
                maybe_text(&r.ex_date),
                maybe_text(&r.record_date),
                maybe_text(&r.payment_date),
                maybe_text(&r.effective_date),
                number(r.cash_dividend),
                number(r.stock_dividend_ratio),
                number(r.split_ratio),
                number(r.rights_ratio),
                number(r.rights_price),
                number(r.old_par),
                number(r.new_par),
                maybe_text(&r.old_symbol),
                maybe_text(&r.new_symbol),
                maybe_text(&r.currency),
                maybe_text(&r.source),
            ]
        },
    )
    .await
}

// ingestion_log is append-only: every attempt gets its own row.
pub async fn insert_ingestion_log(db: &D1Database, log: &IngestionLog) -> Result<()> {
    let sql = "INSERT INTO ingestion_log (trade_date,dataset,source,status,row_count,started_at,finished_at,error_message) VALUES (?,?,?,?,?,?,?,?)";
    db.prepare(sql)
        .bind(&[
            text(&log.trade_date),
            text(&log.dataset),
            text(&log.source),
            text(log.status.as_str()),
            integer(Some(log.row_count)),
            text(&log.started_at),
            text(&log.finished_at),
            maybe_text(&log.error_message),
        ])?
        .run()
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_updates_only_non_key_columns() {
        assert_eq!(
            upsert_sql("t", &["d", "k", "v"], &["d", "k"]),
            "INSERT INTO t (d,k,v) VALUES (?,?,?) ON CONFLICT(d,k) DO UPDATE SET v=excluded.v"
        );
    }

    #[test]
    fn insert_absent_matches_every_column() {
        assert_eq!(
            insert_absent_sql("t", &["a", "b"]),
            "INSERT INTO t (a,b) SELECT ?1,?2 WHERE NOT EXISTS (SELECT 1 FROM t WHERE a IS ?1 AND b IS ?2)"
        );
    }
}
