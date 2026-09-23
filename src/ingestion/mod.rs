use crate::{
    collector::provider::MarketDataProvider,
    model::{
        corporate_action::CorporateAction, index::IndexStat, investor::InvestorStat,
        market::MarketStat, security::SecurityStat, security_update::SecurityUpdate, Dataset,
        IngestionLog, IngestionStatus, Validate,
    },
    storage::{d1, r2},
};
use serde::de::DeserializeOwned;
use worker::{Bucket, D1Database, Error, Result};

fn now_iso() -> String {
    worker::js_sys::Date::new_0().to_iso_string().into()
}

fn decode<T: DeserializeOwned + Validate>(payload: &str, date: &str) -> Result<Vec<T>> {
    let rows: Vec<T> =
        serde_json::from_str(payload).map_err(|e| Error::RustError(e.to_string()))?;
    for row in &rows {
        row.validate(date).map_err(Error::RustError)?;
    }
    Ok(rows)
}

async fn ingest_payload(
    db: &D1Database,
    dataset: Dataset,
    payload: &str,
    date: &str,
) -> Result<usize> {
    match dataset {
        Dataset::SecurityStat => {
            let rows = decode::<SecurityStat>(payload, date)?;
            d1::upsert_security_stat(db, &rows).await?;
            Ok(rows.len())
        }
        Dataset::IndexStat => {
            let rows = decode::<IndexStat>(payload, date)?;
            d1::upsert_index_stat(db, &rows).await?;
            Ok(rows.len())
        }
        Dataset::MarketStat => {
            let rows = decode::<MarketStat>(payload, date)?;
            d1::upsert_market_stat(db, &rows).await?;
            Ok(rows.len())
        }
        Dataset::InvestorStat => {
            let rows = decode::<InvestorStat>(payload, date)?;
            d1::upsert_investor_stat(db, &rows).await?;
            Ok(rows.len())
        }
        Dataset::SecurityUpdate => {
            let rows = decode::<SecurityUpdate>(payload, date)?;
            d1::upsert_security_update(db, &rows).await?;
            Ok(rows.len())
        }
        Dataset::CorporateAction => {
            let rows = decode::<CorporateAction>(payload, date)?;
            d1::insert_corporate_action(db, &rows).await?;
            Ok(rows.len())
        }
    }
}

pub async fn run_dataset<P: MarketDataProvider>(
    provider: &P,
    db: &D1Database,
    bucket: &Bucket,
    dataset: Dataset,
    date: &str,
) -> Result<usize> {
    let started_at = now_iso();
    let outcome: Result<usize> = async {
        let payload = provider
            .fetch(dataset, date)
            .await
            .map_err(Error::RustError)?;
        let key = r2::raw_key(provider.name(), dataset, date)?;
        r2::archive(bucket, &key, &payload).await?;
        ingest_payload(db, dataset, &payload, date).await
    }
    .await;

    // The raw archive key is derivable from source, dataset and date.
    let log = IngestionLog {
        trade_date: date.into(),
        dataset: dataset.table().into(),
        source: provider.name().into(),
        status: if outcome.is_ok() {
            IngestionStatus::Success
        } else {
            IngestionStatus::Failed
        },
        row_count: outcome.as_ref().map(|n| *n as i64).unwrap_or(0),
        error_message: outcome.as_ref().err().map(ToString::to_string),
        started_at,
        finished_at: now_iso(),
    };
    d1::insert_ingestion_log(db, &log).await?;
    outcome
}

pub async fn run_all<P: MarketDataProvider>(
    provider: &P,
    db: &D1Database,
    bucket: &Bucket,
    date: &str,
) -> Result<()> {
    let started_at = now_iso();
    let mut successes = 0;
    let mut total_rows = 0;
    let mut errors = Vec::new();
    for dataset in Dataset::ALL {
        match run_dataset(provider, db, bucket, dataset, date).await {
            Ok(count) => {
                successes += 1;
                total_rows += count as i64;
            }
            Err(error) => errors.push(format!("{}: {error}", dataset.table())),
        }
    }
    let status = match successes {
        n if n == Dataset::ALL.len() => IngestionStatus::Success,
        0 => IngestionStatus::Failed,
        _ => IngestionStatus::Partial,
    };
    d1::insert_ingestion_log(
        db,
        &IngestionLog {
            trade_date: date.into(),
            dataset: "all".into(),
            source: provider.name().into(),
            status,
            row_count: total_rows,
            error_message: if errors.is_empty() {
                None
            } else {
                Some(errors.join("; "))
            },
            started_at,
            finished_at: now_iso(),
        },
    )
    .await?;
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::RustError(errors.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATE: &str = "2026-09-24";

    #[test]
    fn security_stat_requires_key_and_valid_ranges() {
        let row = r#"{"trade_date":"2026-09-24","market_code":"XBKK","orderbook_id":1,"symbol":"ABC","low":1.0,"high":2.0,"eps":-0.5,"listed_shares":1000}"#;
        assert_eq!(
            decode::<SecurityStat>(&format!("[{row}]"), DATE)
                .unwrap()
                .len(),
            1
        );
        for bad in [
            r#"{"trade_date":"2026-09-24","market_code":"XBKK","orderbook_id":1,"symbol":"ABC","low":2.0,"high":1.0}"#,
            r#"{"trade_date":"2026-09-25","market_code":"XBKK","orderbook_id":1,"symbol":"ABC"}"#,
            r#"{"trade_date":"2026-09-24","market_code":" ","orderbook_id":1,"symbol":"ABC"}"#,
            r#"{"trade_date":"2026-09-24","market_code":"XBKK","symbol":"ABC"}"#,
            r#"{"trade_date":"2026-09-24","market_code":"XBKK","orderbook_id":9007199254740992,"symbol":"ABC"}"#,
        ] {
            assert!(
                decode::<SecurityStat>(&format!("[{bad}]"), DATE).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn index_stat_allows_negative_change() {
        let rows = decode::<IndexStat>(
            r#"[{"trade_date":"2026-09-24","index_name":"SET","index_value":1400.5,"change":-3.2,"pe":-1.0}]"#,
            DATE,
        )
        .unwrap();
        assert_eq!(rows[0].index_name, "SET");
        assert!(decode::<IndexStat>(
            r#"[{"trade_date":"2026-09-24","index_name":"SET","low":2.0,"high":1.0}]"#,
            DATE
        )
        .is_err());
    }

    #[test]
    fn market_and_investor_keys_are_required() {
        assert!(decode::<MarketStat>(
            r#"[{"trade_date":"2026-09-24","market_stat_id":"SET","trading_currency":"THB","total_value":1.5}]"#,
            DATE
        )
        .is_ok());
        assert!(decode::<MarketStat>(
            r#"[{"trade_date":"2026-09-24","market_stat_id":"SET"}]"#,
            DATE
        )
        .is_err());
        assert!(decode::<InvestorStat>(
            r#"[{"trade_date":"2026-09-24","period_type":"DAILY","market_segment":"SET","trading_currency":"THB"}]"#,
            DATE
        )
        .is_ok());
        assert!(decode::<InvestorStat>(
            r#"[{"trade_date":"2026-09-24","period_type":"","market_segment":"SET","trading_currency":"THB"}]"#,
            DATE
        )
        .is_err());
    }

    #[test]
    fn security_update_uses_report_date() {
        let row =
            r#"{"report_date":"2026-09-24","market_code":"XBKK","orderbook_id":1,"symbol":"ABC"}"#;
        assert!(decode::<SecurityUpdate>(&format!("[{row}]"), DATE).is_ok());
        assert!(decode::<SecurityUpdate>(&format!("[{row}]"), "2026-09-23").is_err());
    }

    #[test]
    fn corporate_action_dates_are_independent_of_run_date() {
        assert!(decode::<CorporateAction>(
            r#"[{"symbol":"ABC","event_type":"XD","ex_date":"2026-10-15","cash_dividend":0.5}]"#,
            DATE
        )
        .is_ok());
        assert!(decode::<CorporateAction>(
            r#"[{"symbol":"ABC","event_type":"XD","ex_date":"2026-02-30"}]"#,
            DATE
        )
        .is_err());
    }
}
