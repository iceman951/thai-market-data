use super::{finite, non_negative, validate_record, Validate};
use serde::{Deserialize, Serialize};

// Mirrors production index_stat; created_at is set by D1.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStat {
    pub trade_date: String,
    pub data_round: Option<String>,
    pub market_segment: Option<String>,
    pub industry_code: Option<String>,
    pub sector_code: Option<String>,
    pub index_name: String,
    pub index_value: Option<f64>,
    pub previous_close: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub change: Option<f64>,
    pub volume: Option<f64>,
    pub value: Option<f64>,
    pub pe: Option<f64>,
    pub pbv: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub market_cap: Option<f64>,
}

impl Validate for IndexStat {
    fn validate(&self, date: &str) -> Result<(), String> {
        validate_record(&self.trade_date, date, &self.index_name)?;
        if !non_negative(&[
            self.index_value,
            self.previous_close,
            self.open,
            self.high,
            self.low,
            self.volume,
            self.value,
            self.dividend_yield,
            self.market_cap,
        ]) {
            return Err("invalid index level, volume, value or market cap".into());
        }
        // Change, P/E and P/BV may legitimately be negative.
        if !finite(&[self.change, self.pe, self.pbv]) {
            return Err("invalid change or valuation ratio".into());
        }
        if let (Some(low), Some(high)) = (self.low, self.high) {
            if low > high {
                return Err("low exceeds high".into());
            }
        }
        Ok(())
    }
}
