use warp::Filter;
use log::{info};

use crate::models::Metrics;

pub async fn start_metrics_server(metrics: std::sync::Arc<Metrics>, port: u16) {
    let metrics_filter = warp::path("metrics")
        .map(move || {
            let metrics_string = metrics.get_metrics_string();
            info!("Metrics endpoint accessed");
            // println!("📊 Metrics endpoint accessed");
            warp::reply::with_header(metrics_string, "content-type", "text/plain")
        });
    
    //info!("Starting Prometheus metrics server on port {}", port);
    info!("Access metrics over http://localhost:{}/metrics", port);
    //println!("🚀 Starting Prometheus metrics server on port {}", port);
    
    warp::serve(metrics_filter)
        .run(([0, 0, 0, 0], port))
        .await;
}