use super::{finite, non_empty, non_negative, safe_counts, validate_record, Validate};
use serde::{Deserialize, Serialize};

// Mirrors production security_stat; created_at is set by D1.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityStat {
    pub trade_date: String,
    pub market_code: String,
    pub orderbook_id: i64,
    pub symbol: String,
    pub market_segment: Option<String>,
    pub trading_currency: Option<String>,
    pub financial_product: Option<String>,
    pub sector_code: Option<String>,
    pub previous_close: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub last_traded_price: Option<f64>,
    pub average_price: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_offer: Option<f64>,
    pub turnover_quantity: Option<f64>,
    pub turnover_value: Option<f64>,
    pub total_trade: Option<i64>,
    pub trade_report_quantity: Option<f64>,
    pub trade_report_value: Option<f64>,
    pub short_sell_quantity: Option<f64>,
    pub short_sell_value: Option<f64>,
    pub par: Option<f64>,
    pub lot_size: Option<i64>,
    pub listed_shares: Option<i64>,
    pub book_value: Option<f64>,
    pub eps: Option<f64>,
    pub dps: Option<f64>,
    pub pe: Option<f64>,
    pub pbv: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub market_cap: Option<f64>,
    pub corporate_action_code: Option<String>,
    pub notification_sign: Option<String>,
    pub other_sign: Option<String>,
    pub isin: Option<String>,
    pub isin_nvdr: Option<String>,
}

impl Validate for SecurityStat {
    fn validate(&self, date: &str) -> Result<(), String> {
        validate_record(&self.trade_date, date, &self.symbol)?;
        non_empty(&[&self.market_code])?;
        if !safe_counts(&[
            Some(self.orderbook_id),
            self.total_trade,
            self.lot_size,
            self.listed_shares,
        ]) {
            return Err("invalid orderbook id, trade count, lot size or listed shares".into());
        }
        if !non_negative(&[
            self.previous_close,
            self.open,
            self.high,
            self.low,
            self.last_traded_price,
            self.average_price,
            self.best_bid,
            self.best_offer,
            self.turnover_quantity,
            self.turnover_value,
            self.trade_report_quantity,
            self.trade_report_value,
            self.short_sell_quantity,
            self.short_sell_value,
            self.par,
            self.dps,
            self.dividend_yield,
            self.market_cap,
        ]) {
            return Err("invalid security price, quantity or value".into());
        }
        // Book value, EPS, P/E and P/BV are negative for loss-making companies.
        if !finite(&[self.book_value, self.eps, self.pe, self.pbv]) {
            return Err("invalid book value, EPS or valuation ratio".into());
        }
        if let (Some(low), Some(high)) = (self.low, self.high) {
            if low > high {
                return Err("low exceeds high".into());
            }
        }
        Ok(())
    }
}
