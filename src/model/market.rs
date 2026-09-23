use super::{non_empty, non_negative, safe_counts, validate_record, Validate};
use serde::{Deserialize, Serialize};

// Mirrors production market_stat; created_at is set by D1.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketStat {
    pub trade_date: String,
    pub data_round: Option<String>,
    pub market_stat_id: String,
    pub trading_currency: String,
    pub total_trades: Option<i64>,
    pub total_quantity: Option<i64>,
    // Production declares this column INTEGER. f64 is deliberate: SQLite stores
    // a whole number as INTEGER and a fractional one as REAL, so satang are
    // never truncated, whereas i64 would reject or round a fractional source.
    // TODO: once a real provider exists, check its actual type for this field,
    // then either make this i64 (always whole baht) or migrate the D1 column
    // to REAL (fractional). Do not truncate in the meantime.
    pub total_value: Option<f64>,
    pub up_quantity: Option<i64>,
    pub down_quantity: Option<i64>,
    pub no_change_quantity: Option<i64>,
    pub up_shares: Option<i64>,
    pub down_shares: Option<i64>,
    pub no_change_shares: Option<i64>,
}

impl Validate for MarketStat {
    fn validate(&self, date: &str) -> Result<(), String> {
        validate_record(&self.trade_date, date, &self.market_stat_id)?;
        non_empty(&[&self.trading_currency])?;
        if !safe_counts(&[
            self.total_trades,
            self.total_quantity,
            self.up_quantity,
            self.down_quantity,
            self.no_change_quantity,
            self.up_shares,
            self.down_shares,
            self.no_change_shares,
        ]) || !non_negative(&[self.total_value])
        {
            return Err("invalid market count or value".into());
        }
        Ok(())
    }
}
