mod collector;
mod ingestion;
mod model;
mod scheduled;
mod storage;

use worker::{event, Context, Env, Method, Request, Response, Result};

#[event(fetch)]
pub async fn main(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    if req.method() == Method::Get && req.path() == "/health" {
        Response::from_json(&serde_json::json!({"status":"ok","service":"thai-market-data"}))
    } else {
        Response::error("Not Found", 404)
    }
}
