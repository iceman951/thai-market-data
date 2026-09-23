use crate::model::{
    corporate_action::CorporateAction, index::IndexStat, investor::InvestorStat,
    market::MarketStat, security::SecurityStat, security_update::SecurityUpdate, Dataset,
};

/// A market data source.
///
/// `fetch` returns the source response exactly as received; ingestion archives
/// that string to R2 unchanged. The `normalize_*` methods map the same string
/// into the canonical D1 models, so source field names and shapes never leave
/// the provider's module. Validation of the canonical rows happens afterwards,
/// in ingestion, and is the same for every provider.
pub trait MarketDataProvider {
    /// Lowercase slug used in R2 keys and as `ingestion_log.source`.
    fn name(&self) -> &'static str;

    async fn fetch(&self, dataset: Dataset, date: &str) -> Result<String, String>;

    fn normalize_security_stat(&self, raw: &str, date: &str) -> Result<Vec<SecurityStat>, String>;
    fn normalize_index_stat(&self, raw: &str, date: &str) -> Result<Vec<IndexStat>, String>;
    fn normalize_market_stat(&self, raw: &str, date: &str) -> Result<Vec<MarketStat>, String>;
    fn normalize_investor_stat(&self, raw: &str, date: &str) -> Result<Vec<InvestorStat>, String>;
    fn normalize_security_update(
        &self,
        raw: &str,
        date: &str,
    ) -> Result<Vec<SecurityUpdate>, String>;
    fn normalize_corporate_action(
        &self,
        raw: &str,
        date: &str,
    ) -> Result<Vec<CorporateAction>, String>;
}
