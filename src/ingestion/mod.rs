use crate::{
    collector::provider::MarketDataProvider,
    model::{CanonicalRows, Dataset, IngestionLog, IngestionStatus},
    storage::{d1, r2},
};
use worker::{Bucket, D1Database, Error, Result};

fn now_iso() -> String {
    worker::js_sys::Date::new_0().to_iso_string().into()
}

/// Maps a raw payload to canonical rows with the provider's own normalizer,
/// then applies the provider-independent validation. Ingestion never parses a
/// source payload itself.
fn normalize<P: MarketDataProvider>(
    provider: &P,
    dataset: Dataset,
    raw: &str,
    date: &str,
) -> std::result::Result<CanonicalRows, String> {
    let rows = match dataset {
        Dataset::SecurityStat => {
            CanonicalRows::SecurityStat(provider.normalize_security_stat(raw, date)?)
        }
        Dataset::IndexStat => CanonicalRows::IndexStat(provider.normalize_index_stat(raw, date)?),
        Dataset::MarketStat => {
            CanonicalRows::MarketStat(provider.normalize_market_stat(raw, date)?)
        }
        Dataset::InvestorStat => {
            CanonicalRows::InvestorStat(provider.normalize_investor_stat(raw, date)?)
        }
        Dataset::SecurityUpdate => {
            CanonicalRows::SecurityUpdate(provider.normalize_security_update(raw, date)?)
        }
        Dataset::CorporateAction => {
            CanonicalRows::CorporateAction(provider.normalize_corporate_action(raw, date)?)
        }
    };
    rows.validate(date)?;
    Ok(rows)
}

/// fetch -> R2 key -> archive exact raw -> normalize -> validate -> D1 -> log.
pub async fn run_dataset<P: MarketDataProvider>(
    provider: &P,
    db: &D1Database,
    bucket: &Bucket,
    dataset: Dataset,
    date: &str,
) -> Result<usize> {
    let started_at = now_iso();
    let outcome: Result<usize> = async {
        let raw = provider
            .fetch(dataset, date)
            .await
            .map_err(Error::RustError)?;
        let key = r2::raw_key(provider.name(), dataset, date)?;
        // Archived before normalizing, so an unparseable payload is still kept.
        r2::archive(bucket, &key, &raw).await?;
        let rows = normalize(provider, dataset, &raw, date).map_err(Error::RustError)?;
        // Written in batches; a failure can leave earlier batches in D1 (see d1::write_rows).
        d1::write_dataset(db, &rows).await?;
        Ok(rows.row_count())
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
    use crate::{
        collector::mock::{self, MockProvider},
        model::{
            corporate_action::CorporateAction, index::IndexStat, investor::InvestorStat,
            market::MarketStat, security::SecurityStat, security_update::SecurityUpdate,
        },
    };
    use serde::Deserialize;

    const DATE: &str = "2026-09-24";

    /// A source whose payload uses its own field names, like a real vendor.
    struct VendorProvider;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct VendorQuote {
        trade_date: String,
        order_book_id: i64,
        symbol: String,
        high: Option<f64>,
        low: Option<f64>,
        last: Option<f64>,
    }

    impl MarketDataProvider for VendorProvider {
        fn name(&self) -> &'static str {
            "vendor"
        }
        async fn fetch(&self, _: Dataset, _: &str) -> std::result::Result<String, String> {
            Err("not used".into())
        }
        fn normalize_security_stat(
            &self,
            raw: &str,
            _: &str,
        ) -> std::result::Result<Vec<SecurityStat>, String> {
            let quotes: Vec<VendorQuote> = serde_json::from_str(raw).map_err(|e| e.to_string())?;
            Ok(quotes
                .into_iter()
                .map(|q| SecurityStat {
                    trade_date: q.trade_date,
                    market_code: "XBKK".into(),
                    orderbook_id: q.order_book_id,
                    symbol: q.symbol,
                    high: q.high,
                    low: q.low,
                    last_traded_price: q.last,
                    ..Default::default()
                })
                .collect())
        }
        fn normalize_index_stat(
            &self,
            _: &str,
            _: &str,
        ) -> std::result::Result<Vec<IndexStat>, String> {
            Err("unsupported".into())
        }
        fn normalize_market_stat(
            &self,
            _: &str,
            _: &str,
        ) -> std::result::Result<Vec<MarketStat>, String> {
            Err("unsupported".into())
        }
        fn normalize_investor_stat(
            &self,
            _: &str,
            _: &str,
        ) -> std::result::Result<Vec<InvestorStat>, String> {
            Err("unsupported".into())
        }
        fn normalize_security_update(
            &self,
            _: &str,
            _: &str,
        ) -> std::result::Result<Vec<SecurityUpdate>, String> {
            Err("unsupported".into())
        }
        fn normalize_corporate_action(
            &self,
            _: &str,
            _: &str,
        ) -> std::result::Result<Vec<CorporateAction>, String> {
            Err("unsupported".into())
        }
    }

    const VENDOR_RAW: &str = r#"[{"tradeDate":"2026-09-24","orderBookId":42,"symbol":"PTT","high":35.0,"low":34.0,"last":34.5}]"#;

    #[test]
    fn vendor_payload_goes_through_provider_normalizer() {
        let rows = normalize(&VendorProvider, Dataset::SecurityStat, VENDOR_RAW, DATE).unwrap();
        let CanonicalRows::SecurityStat(rows) = rows else {
            panic!("wrong dataset variant");
        };
        assert_eq!(rows[0].orderbook_id, 42);
        assert_eq!(rows[0].last_traded_price, Some(34.5));
        // The raw payload is not in canonical shape; direct deserialization would fail.
        assert!(serde_json::from_str::<Vec<SecurityStat>>(VENDOR_RAW).is_err());
    }

    #[test]
    fn normalization_leaves_raw_payload_untouched() {
        let raw = VENDOR_RAW.to_string();
        normalize(&VendorProvider, Dataset::SecurityStat, &raw, DATE).unwrap();
        assert_eq!(raw, VENDOR_RAW);
    }

    #[test]
    fn validation_runs_after_normalization() {
        let bad = r#"[{"tradeDate":"2026-09-24","orderBookId":42,"symbol":"PTT","high":34.0,"low":35.0}]"#;
        assert_eq!(
            normalize(&VendorProvider, Dataset::SecurityStat, bad, DATE).unwrap_err(),
            "row 0: low exceeds high"
        );
        let wrong_day = VENDOR_RAW.replace("2026-09-24", "2026-09-23");
        assert!(normalize(&VendorProvider, Dataset::SecurityStat, &wrong_day, DATE).is_err());
    }

    #[test]
    fn normalizer_errors_are_reported() {
        assert!(normalize(&VendorProvider, Dataset::SecurityStat, "not json", DATE).is_err());
        assert_eq!(
            normalize(&VendorProvider, Dataset::IndexStat, "[]", DATE).unwrap_err(),
            "unsupported"
        );
    }

    #[test]
    fn mock_payloads_normalize_into_each_canonical_model() {
        for dataset in Dataset::ALL {
            let raw = mock::raw_payload(dataset, DATE);
            assert_eq!(
                raw,
                mock::raw_payload(dataset, DATE),
                "retry must match archive"
            );
            let rows = normalize(&MockProvider, dataset, &raw, DATE)
                .unwrap_or_else(|e| panic!("{}: {e}", dataset.table()));
            assert_eq!(rows.row_count(), 1, "{}", dataset.table());
            let variant_matches = matches!(
                (dataset, &rows),
                (Dataset::SecurityStat, CanonicalRows::SecurityStat(_))
                    | (Dataset::IndexStat, CanonicalRows::IndexStat(_))
                    | (Dataset::MarketStat, CanonicalRows::MarketStat(_))
                    | (Dataset::InvestorStat, CanonicalRows::InvestorStat(_))
                    | (Dataset::SecurityUpdate, CanonicalRows::SecurityUpdate(_))
                    | (Dataset::CorporateAction, CanonicalRows::CorporateAction(_))
            );
            assert!(variant_matches, "{}", dataset.table());
        }
    }
}
