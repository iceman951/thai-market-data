use crate::{collector::mock::MockProvider, ingestion};
use worker::{wasm_bindgen::JsValue, Env, Result, ScheduleContext, ScheduledEvent};

#[worker::event(scheduled)]
pub async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let date = worker::js_sys::Date::new(&JsValue::from_f64(event.schedule())).to_iso_string();
    let date: String = date.into();
    let outcome: Result<()> = async {
        let db = env.d1("DB")?;
        let bucket = env.bucket("RAW_BUCKET")?;
        ingestion::run_all(&MockProvider, &db, &bucket, &date[..10]).await
    }
    .await;
    if let Err(error) = outcome {
        // workers-rs does not propagate a Result from its scheduled event wrapper.
        worker::console_error!(
            "{}",
            serde_json::json!({"event":"ingestion_failed","date":&date[..10],"error":error.to_string()})
        );
    }
}
