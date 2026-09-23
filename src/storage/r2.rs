use crate::model::{validate_date, Dataset};
use worker::{Bucket, Conditional, Error, HttpMetadata, Result};

pub fn raw_key(provider: &str, dataset: Dataset, date: &str) -> Result<String> {
    validate_date(date).map_err(Error::RustError)?;
    if provider.is_empty()
        || !provider
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(Error::RustError("invalid provider slug".into()));
    }
    Ok(format!(
        "raw/{}/{}/{}/{}/{}.json",
        provider,
        dataset.slug(),
        &date[..4],
        &date[5..7],
        date
    ))
}

pub async fn archive(bucket: &Bucket, key: &str, payload: &str) -> Result<()> {
    let condition = Conditional {
        etag_does_not_match: Some("*".into()),
        ..Default::default()
    };
    let metadata = HttpMetadata {
        content_type: Some("application/json".into()),
        ..Default::default()
    };
    if bucket
        .put(key, payload.to_owned())
        .only_if(condition)
        .http_metadata(metadata)
        .execute()
        .await?
        .is_some()
    {
        return Ok(());
    }
    let object = bucket
        .get(key)
        .execute()
        .await?
        .ok_or_else(|| Error::RustError("raw archive conflict with missing object".into()))?;
    let body = object
        .body()
        .ok_or_else(|| Error::RustError("raw archive conflict with missing body".into()))?;
    let existing = body.text().await?;
    if existing == payload {
        Ok(())
    } else {
        Err(Error::RustError(format!("raw archive conflict at {key}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_is_stable() {
        assert_eq!(
            raw_key("mock", Dataset::SecurityStat, "2026-09-24").unwrap(),
            "raw/mock/security-stat/2026/09/2026-09-24.json"
        );
        assert!(raw_key("../bad", Dataset::SecurityStat, "2026-09-24").is_err());
    }
}
