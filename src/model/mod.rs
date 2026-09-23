pub mod corporate_action;
pub mod index;
pub mod investor;
pub mod market;
pub mod security;
pub mod security_update;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dataset {
    SecurityStat,
    IndexStat,
    MarketStat,
    InvestorStat,
    SecurityUpdate,
    CorporateAction,
}

impl Dataset {
    pub const ALL: [Self; 6] = [
        Self::SecurityStat,
        Self::IndexStat,
        Self::MarketStat,
        Self::InvestorStat,
        Self::SecurityUpdate,
        Self::CorporateAction,
    ];

    /// D1 table name, also the dataset label in ingestion_log.
    pub fn table(self) -> &'static str {
        match self {
            Self::SecurityStat => "security_stat",
            Self::IndexStat => "index_stat",
            Self::MarketStat => "market_stat",
            Self::InvestorStat => "investor_stat",
            Self::SecurityUpdate => "security_update",
            Self::CorporateAction => "corporate_action",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::SecurityStat => "security-stat",
            Self::IndexStat => "index-stat",
            Self::MarketStat => "market-stat",
            Self::InvestorStat => "investor-stat",
            Self::SecurityUpdate => "security-update",
            Self::CorporateAction => "corporate-action",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IngestionStatus {
    Success,
    Failed,
    Partial,
}

impl IngestionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "SUCCESS",
            Self::Failed => "FAILED",
            Self::Partial => "PARTIAL",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionLog {
    pub trade_date: String,
    pub dataset: String,
    pub source: String,
    pub status: IngestionStatus,
    pub row_count: i64,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: String,
}

/// One dataset's rows in the canonical D1 shape, as returned by a provider's
/// normalizer. Nothing in this type depends on a source's field names.
#[derive(Debug, Clone)]
pub enum CanonicalRows {
    SecurityStat(Vec<security::SecurityStat>),
    IndexStat(Vec<index::IndexStat>),
    MarketStat(Vec<market::MarketStat>),
    InvestorStat(Vec<investor::InvestorStat>),
    SecurityUpdate(Vec<security_update::SecurityUpdate>),
    CorporateAction(Vec<corporate_action::CorporateAction>),
}

impl CanonicalRows {
    pub fn row_count(&self) -> usize {
        match self {
            Self::SecurityStat(rows) => rows.len(),
            Self::IndexStat(rows) => rows.len(),
            Self::MarketStat(rows) => rows.len(),
            Self::InvestorStat(rows) => rows.len(),
            Self::SecurityUpdate(rows) => rows.len(),
            Self::CorporateAction(rows) => rows.len(),
        }
    }

    pub fn validate(&self, date: &str) -> Result<(), String> {
        match self {
            Self::SecurityStat(rows) => validate_rows(rows, date),
            Self::IndexStat(rows) => validate_rows(rows, date),
            Self::MarketStat(rows) => validate_rows(rows, date),
            Self::InvestorStat(rows) => validate_rows(rows, date),
            Self::SecurityUpdate(rows) => validate_rows(rows, date),
            Self::CorporateAction(rows) => validate_rows(rows, date),
        }
    }
}

fn validate_rows<T: Validate>(rows: &[T], date: &str) -> Result<(), String> {
    rows.iter()
        .enumerate()
        .try_for_each(|(i, row)| row.validate(date).map_err(|e| format!("row {i}: {e}")))
}

// D1 bindings pass SQLite integers through JavaScript numbers.
pub const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

pub trait Validate {
    fn validate(&self, date: &str) -> Result<(), String>;
}

pub fn validate_date(date: &str) -> Result<(), String> {
    let b = date.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return Err(format!("invalid ISO date: {date}"));
    }
    let year: i32 = date[..4].parse().map_err(|_| "invalid year")?;
    let month: u32 = date[5..7].parse().map_err(|_| "invalid month")?;
    let day: u32 = date[8..].parse().map_err(|_| "invalid day")?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return Err(format!("invalid ISO date: {date}")),
    };
    if day == 0 || day > max_day {
        return Err(format!("invalid ISO date: {date}"));
    }
    Ok(())
}

pub fn non_negative(values: &[Option<f64>]) -> bool {
    values.iter().flatten().all(|v| v.is_finite() && *v >= 0.0)
}

pub fn finite(values: &[Option<f64>]) -> bool {
    values.iter().flatten().all(|v| v.is_finite())
}

pub fn safe_counts(values: &[Option<i64>]) -> bool {
    values
        .iter()
        .flatten()
        .all(|v| (0..=MAX_SAFE_INTEGER).contains(v))
}

pub fn non_empty(values: &[&str]) -> Result<(), String> {
    if values.iter().any(|v| v.trim().is_empty()) {
        return Err("empty record identity".into());
    }
    Ok(())
}

pub fn validate_record(date: &str, expected: &str, identity: &str) -> Result<(), String> {
    validate_date(date)?;
    if date != expected {
        return Err(format!(
            "record date {date} differs from requested {expected}"
        ));
    }
    if identity.trim().is_empty() {
        return Err("empty record identity".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;

    const DATE: &str = "2026-09-24";

    fn check<T: DeserializeOwned + Validate>(json: &str, date: &str) -> Result<(), String> {
        serde_json::from_str::<T>(json).unwrap().validate(date)
    }

    #[test]
    fn dates_are_calendar_dates() {
        assert!(validate_date("2024-02-29").is_ok());
        assert!(validate_date("2026-02-29").is_err());
        assert!(validate_date("2026-09-24").is_ok());
    }

    #[test]
    fn security_stat_requires_key_and_valid_ranges() {
        use security::SecurityStat;
        assert!(check::<SecurityStat>(
            r#"{"trade_date":"2026-09-24","market_code":"XBKK","orderbook_id":1,"symbol":"ABC","low":1.0,"high":2.0,"eps":-0.5,"listed_shares":1000}"#,
            DATE
        )
        .is_ok());
        for bad in [
            r#"{"trade_date":"2026-09-24","market_code":"XBKK","orderbook_id":1,"symbol":"ABC","low":2.0,"high":1.0}"#,
            r#"{"trade_date":"2026-09-25","market_code":"XBKK","orderbook_id":1,"symbol":"ABC"}"#,
            r#"{"trade_date":"2026-09-24","market_code":" ","orderbook_id":1,"symbol":"ABC"}"#,
            r#"{"trade_date":"2026-09-24","market_code":"XBKK","orderbook_id":9007199254740992,"symbol":"ABC"}"#,
        ] {
            assert!(check::<SecurityStat>(bad, DATE).is_err(), "{bad}");
        }
    }

    #[test]
    fn index_stat_allows_negative_change() {
        use index::IndexStat;
        assert!(check::<IndexStat>(
            r#"{"trade_date":"2026-09-24","index_name":"SET","index_value":1400.5,"change":-3.2,"pe":-1.0}"#,
            DATE
        )
        .is_ok());
        assert!(check::<IndexStat>(
            r#"{"trade_date":"2026-09-24","index_name":"SET","low":2.0,"high":1.0}"#,
            DATE
        )
        .is_err());
    }

    #[test]
    fn market_and_investor_keys_are_required() {
        assert!(check::<market::MarketStat>(
            r#"{"trade_date":"2026-09-24","market_stat_id":"SET","trading_currency":"THB","total_value":1.5}"#,
            DATE
        )
        .is_ok());
        assert!(check::<market::MarketStat>(
            r#"{"trade_date":"2026-09-24","market_stat_id":"SET","trading_currency":" "}"#,
            DATE
        )
        .is_err());
        assert!(check::<investor::InvestorStat>(
            r#"{"trade_date":"2026-09-24","period_type":"","market_segment":"SET","trading_currency":"THB"}"#,
            DATE
        )
        .is_err());
    }

    #[test]
    fn security_update_uses_report_date() {
        let row =
            r#"{"report_date":"2026-09-24","market_code":"XBKK","orderbook_id":1,"symbol":"ABC"}"#;
        assert!(check::<security_update::SecurityUpdate>(row, DATE).is_ok());
        assert!(check::<security_update::SecurityUpdate>(row, "2026-09-23").is_err());
    }

    #[test]
    fn corporate_action_dates_are_independent_of_run_date() {
        use corporate_action::CorporateAction;
        assert!(check::<CorporateAction>(
            r#"{"symbol":"ABC","event_type":"XD","ex_date":"2026-10-15","cash_dividend":0.5}"#,
            DATE
        )
        .is_ok());
        assert!(check::<CorporateAction>(
            r#"{"symbol":"ABC","event_type":"XD","ex_date":"2026-02-30"}"#,
            DATE
        )
        .is_err());
    }

    #[test]
    fn canonical_rows_report_the_failing_row() {
        let rows = CanonicalRows::IndexStat(vec![
            serde_json::from_str(r#"{"trade_date":"2026-09-24","index_name":"SET"}"#).unwrap(),
            serde_json::from_str(r#"{"trade_date":"2026-09-24","index_name":""}"#).unwrap(),
        ]);
        assert_eq!(rows.row_count(), 2);
        assert_eq!(
            rows.validate(DATE).unwrap_err(),
            "row 1: empty record identity"
        );
    }
}
