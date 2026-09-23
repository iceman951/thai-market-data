use super::provider::MarketDataProvider;
use crate::model::{
    corporate_action::CorporateAction, index::IndexStat, investor::InvestorStat,
    market::MarketStat, security::SecurityStat, security_update::SecurityUpdate, Dataset,
};
use serde::de::DeserializeOwned;

/// Synthetic source. Its raw payloads happen to use canonical field names, so
/// its normalizers are plain deserialization, but they still run through the
/// provider normalization path like any real source.
pub struct MockProvider;

/// The exact response the mock "source" returns. Deterministic per dataset and
/// date, so a same-day retry matches the archived R2 object.
pub fn raw_payload(dataset: Dataset, date: &str) -> String {
    match dataset {
        Dataset::SecurityStat => format!(
            r#"[{{"trade_date":"{date}","market_code":"MOCK-MARKET","orderbook_id":1,"symbol":"MOCK","market_segment":"SET","trading_currency":"THB","previous_close":10.0,"open":10.0,"high":11.0,"low":9.0,"last_traded_price":10.5,"average_price":10.2,"turnover_quantity":100.0,"turnover_value":1020.0,"total_trade":5,"par":1.0,"lot_size":100,"listed_shares":1000000,"eps":0.8,"pe":13.1,"pbv":1.2,"market_cap":10500000.0}}]"#
        ),
        Dataset::IndexStat => format!(
            r#"[{{"trade_date":"{date}","data_round":"EOD","market_segment":"SET","industry_code":null,"sector_code":null,"index_name":"MOCK-INDEX","index_value":100.5,"previous_close":100.0,"open":100.0,"high":101.0,"low":99.0,"change":0.5,"volume":1000.0,"value":100500.0,"pe":15.2,"pbv":1.4,"dividend_yield":3.1,"market_cap":1000000.0}}]"#
        ),
        Dataset::MarketStat => format!(
            r#"[{{"trade_date":"{date}","data_round":"EOD","market_stat_id":"MOCK-MARKET","trading_currency":"THB","total_trades":5,"total_quantity":1000,"total_value":100500,"up_quantity":1,"down_quantity":0,"no_change_quantity":0,"up_shares":1000,"down_shares":0,"no_change_shares":0}}]"#
        ),
        Dataset::InvestorStat => format!(
            r#"[{{"trade_date":"{date}","data_round":"EOD","period_type":"DAILY","market_segment":"MOCK-MARKET","trading_currency":"THB","buy_institute_value":40.0,"buy_foreign_value":30.0,"buy_customer_value":20.0,"buy_proprietary_value":10.0,"sell_institute_value":35.0,"sell_foreign_value":25.0,"sell_customer_value":30.0,"sell_proprietary_value":10.0,"total_value":100.0}}]"#
        ),
        Dataset::SecurityUpdate => format!(
            r#"[{{"report_date":"{date}","data_round":"EOD","market_code":"MOCK-MARKET","orderbook_id":1,"symbol":"MOCK","long_name":"Synthetic security","financial_product":"CS","par":1.0,"lot_size":100,"instrument_state":"ACTIVE"}}]"#
        ),
        Dataset::CorporateAction => format!(
            r#"[{{"symbol":"MOCK","orderbook_id":1,"event_type":"MOCK-ACTION","announcement_date":"{date}","ex_date":null,"cash_dividend":0.5,"currency":"THB","source":"mock"}}]"#
        ),
    }
}

fn parse<T: DeserializeOwned>(raw: &str) -> Result<Vec<T>, String> {
    serde_json::from_str(raw).map_err(|e| format!("mock payload: {e}"))
}

impl MarketDataProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn fetch(&self, dataset: Dataset, date: &str) -> Result<String, String> {
        Ok(raw_payload(dataset, date))
    }

    fn normalize_security_stat(&self, raw: &str, _date: &str) -> Result<Vec<SecurityStat>, String> {
        parse(raw)
    }
    fn normalize_index_stat(&self, raw: &str, _date: &str) -> Result<Vec<IndexStat>, String> {
        parse(raw)
    }
    fn normalize_market_stat(&self, raw: &str, _date: &str) -> Result<Vec<MarketStat>, String> {
        parse(raw)
    }
    fn normalize_investor_stat(&self, raw: &str, _date: &str) -> Result<Vec<InvestorStat>, String> {
        parse(raw)
    }
    fn normalize_security_update(
        &self,
        raw: &str,
        _date: &str,
    ) -> Result<Vec<SecurityUpdate>, String> {
        parse(raw)
    }
    fn normalize_corporate_action(
        &self,
        raw: &str,
        _date: &str,
    ) -> Result<Vec<CorporateAction>, String> {
        parse(raw)
    }
}
