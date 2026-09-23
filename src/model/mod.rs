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

    #[test]
    fn dates_are_calendar_dates() {
        assert!(validate_date("2024-02-29").is_ok());
        assert!(validate_date("2026-02-29").is_err());
        assert!(validate_date("2026-09-24").is_ok());
    }
}
