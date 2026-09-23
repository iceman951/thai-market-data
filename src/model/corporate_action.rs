use super::{non_empty, non_negative, safe_counts, validate_date, Validate};
use serde::{Deserialize, Serialize};

// Mirrors production corporate_action; id and created_at are set by D1.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorporateAction {
    pub symbol: String,
    pub orderbook_id: Option<i64>,
    pub event_type: String,
    pub announcement_date: Option<String>,
    pub ex_date: Option<String>,
    pub record_date: Option<String>,
    pub payment_date: Option<String>,
    pub effective_date: Option<String>,
    pub cash_dividend: Option<f64>,
    pub stock_dividend_ratio: Option<f64>,
    pub split_ratio: Option<f64>,
    pub rights_ratio: Option<f64>,
    pub rights_price: Option<f64>,
    pub old_par: Option<f64>,
    pub new_par: Option<f64>,
    pub old_symbol: Option<String>,
    pub new_symbol: Option<String>,
    pub currency: Option<String>,
    pub source: Option<String>,
}

impl Validate for CorporateAction {
    // Events are announced ahead of their dates, so none has to equal the run date.
    fn validate(&self, _date: &str) -> Result<(), String> {
        non_empty(&[&self.symbol, &self.event_type])?;
        for d in [
            &self.announcement_date,
            &self.ex_date,
            &self.record_date,
            &self.payment_date,
            &self.effective_date,
        ]
        .into_iter()
        .flatten()
        {
            validate_date(d)?;
        }
        if !safe_counts(&[self.orderbook_id])
            || !non_negative(&[
                self.cash_dividend,
                self.stock_dividend_ratio,
                self.split_ratio,
                self.rights_ratio,
                self.rights_price,
                self.old_par,
                self.new_par,
            ])
        {
            return Err("invalid corporate action amount or ratio".into());
        }
        Ok(())
    }
}
