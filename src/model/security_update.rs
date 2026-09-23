use super::{non_empty, non_negative, safe_counts, validate_record, Validate};
use serde::{Deserialize, Serialize};

// Mirrors production security_update; created_at is set by D1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityUpdate {
    pub report_date: String,
    pub data_round: Option<String>,
    pub market_code: String,
    pub market_segment: Option<String>,
    pub trading_currency: Option<String>,
    pub orderbook_id: i64,
    pub symbol: String,
    pub long_name: Option<String>,
    pub originates_from: Option<String>,
    pub financial_product: Option<String>,
    pub sector_code: Option<String>,
    pub pqf: Option<f64>,
    pub par: Option<f64>,
    pub lot_size: Option<i64>,
    pub isin: Option<String>,
    pub isin_nvdr: Option<String>,
    pub instrument_state: Option<String>,
    pub instrument_status: Option<String>,
}

impl Validate for SecurityUpdate {
    fn validate(&self, date: &str) -> Result<(), String> {
        validate_record(&self.report_date, date, &self.symbol)?;
        non_empty(&[&self.market_code])?;
        if !safe_counts(&[Some(self.orderbook_id), self.lot_size])
            || !non_negative(&[self.pqf, self.par])
        {
            return Err("invalid orderbook id, lot size, PQF or par".into());
        }
        Ok(())
    }
}
