use super::{non_empty, non_negative, validate_record, Validate};
use serde::{Deserialize, Serialize};

// Mirrors production investor_stat; created_at is set by D1.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InvestorStat {
    pub trade_date: String,
    pub data_round: Option<String>,
    pub period_type: String,
    pub market_segment: String,
    pub trading_currency: String,
    pub buy_institute_value: Option<f64>,
    pub buy_foreign_value: Option<f64>,
    pub buy_customer_value: Option<f64>,
    pub buy_proprietary_value: Option<f64>,
    pub sell_institute_value: Option<f64>,
    pub sell_foreign_value: Option<f64>,
    pub sell_customer_value: Option<f64>,
    pub sell_proprietary_value: Option<f64>,
    pub total_value: Option<f64>,
}

impl Validate for InvestorStat {
    fn validate(&self, date: &str) -> Result<(), String> {
        validate_record(&self.trade_date, date, &self.market_segment)?;
        non_empty(&[&self.period_type, &self.trading_currency])?;
        if !non_negative(&[
            self.buy_institute_value,
            self.buy_foreign_value,
            self.buy_customer_value,
            self.buy_proprietary_value,
            self.sell_institute_value,
            self.sell_foreign_value,
            self.sell_customer_value,
            self.sell_proprietary_value,
            self.total_value,
        ]) {
            return Err("invalid investor value".into());
        }
        Ok(())
    }
}
