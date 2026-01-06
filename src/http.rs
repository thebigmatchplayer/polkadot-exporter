use async_std::sync::RwLock;
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use std::sync::Arc;
use tide::{Request, Response};

use crate::prometheus::Metrics;
use crate::substrate::SubstrateRPC;
use crate::utils::Config;

#[derive(Clone)]
pub struct State {
    pub config: Arc<Config>,
    pub registry: Arc<Registry>,
    pub metrics: Arc<Metrics>,
    pub rpc: Arc<RwLock<Option<Arc<SubstrateRPC>>>>,
    pub shutdown: Arc<RwLock<bool>>,
}
// fetch all metrics
pub async fn handle_metrics(req: Request<State>) -> tide::Result {
    let state = req.state();
    let mut encoded = String::new();
    encode(&mut encoded, &state.registry).map_err(|e| tide::Error::from_str(500, e.to_string()))?;
    Ok(Response::builder(200)
        .body(encoded)
        .content_type(tide::http::mime::PLAIN)
        .build())
}
