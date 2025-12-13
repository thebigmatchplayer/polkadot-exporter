use clap::Parser;
use reqwest::Client;
use serde_json::{Value, json};
use std::i64;
use std::{thread, time::Duration};
use prometheus::{Counter, Gauge, Registry, Encoder, TextEncoder};
use warp::Filter;
use log::{info, error, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    url: Option<String>,
    #[arg(short, long)]
    port: Option<String>,
    #[arg(short = 'm', long, default_value = "9090")]
    metrics_port: u16,
}

struct Metrics {
    registry: Registry,
    block_height: Gauge,
    rpc_requests_total: Counter,
    rpc_errors_total: Counter,
}

impl Metrics {
    fn new() -> Self {
        let registry = Registry::new();
        
        let block_height = Gauge::new(
            "polkadot_block_height",
            "Current block height of the Polkadot chain"
        ).expect("Failed to create block_height gauge");
        
        let rpc_requests_total = Counter::new(
            "polkadot_rpc_requests_total",
            "Total number of RPC requests made"
        ).expect("Failed to create rpc_requests_total counter");
        
        let rpc_errors_total = Counter::new(
            "polkadot_rpc_errors_total", 
            "Total number of RPC errors"
        ).expect("Failed to create rpc_errors_total counter");
        
        registry.register(Box::new(block_height.clone())).expect("Failed to register block_height");
        registry.register(Box::new(rpc_requests_total.clone())).expect("Failed to register rpc_requests_total");
        registry.register(Box::new(rpc_errors_total.clone())).expect("Failed to register rpc_errors_total");
        
        Self {
            registry,
            block_height,
            rpc_requests_total,
            rpc_errors_total,
        }
    }
    
    fn get_metrics_string(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).expect("Failed to encode metrics");
        String::from_utf8(buffer).expect("Failed to convert metrics to string")
    }
}

async fn start_metrics_server(metrics: std::sync::Arc<Metrics>, port: u16) {
    let metrics_filter = warp::path("metrics")
        .map(move || {
            let metrics_string = metrics.get_metrics_string();
            info!("Metrics endpoint accessed");
            println!("📊 Metrics endpoint accessed");
            warp::reply::with_header(metrics_string, "content-type", "text/plain")
        });
    
    info!("Starting Prometheus metrics server on port {}", port);
    println!("🚀 Starting Prometheus metrics server on port {}", port);
    
    warp::serve(metrics_filter)
        .run(([0, 0, 0, 0], port))
        .await;
}

#[tokio::main]
async fn main() {
    // Initialize logger
    env_logger::init();
    
    let cli = Cli::parse();
    let client = Client::new();
    let metrics = std::sync::Arc::new(Metrics::new());
    
    let rpc_url = cli
        .url
        .unwrap_or_else(|| "https://polkadot-rpc.publicnode.com".to_string());
    
    let rpc_port = cli.port.unwrap_or_else(|| "".to_string());
    
    info!("Starting Polkadot block height monitor");
    info!("RPC URL: {}{}", rpc_url, rpc_port);
    info!("Metrics port: {}", cli.metrics_port);
    
    println!("Starting Polkadot block height monitor");
    println!("RPC URL: {}{}", rpc_url, rpc_port);
    println!("Metrics available at: http://localhost:{}/metrics", cli.metrics_port);
    
    // Start metrics server in background
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        start_metrics_server(metrics_clone, cli.metrics_port).await;
    });
    
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "chain_getHeader",
        "params": [],
        "id": 1
    });
    
    loop {
        let formatted_url = format!("{}{}", rpc_url, rpc_port);
        
        info!("Making RPC request to {}", formatted_url);
        metrics.rpc_requests_total.inc();
        
        let response = client
            .post(formatted_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                let response_text = resp.text().await;
                match response_text {
                    Ok(text) => match serde_json::from_str::<Value>(&text) {
                        Ok(json) => match json["result"]["number"].as_str() {
                            Some(hex_str) => match i64::from_str_radix(&hex_str[2..], 16) {
                                Ok(height) => {
                                    metrics.block_height.set(height as f64);
                                    info!("Block height updated: {}", height);
                                    println!("📦 Height: {} | RPC Requests: {} | Errors: {}", 
                                        height, 
                                        metrics.rpc_requests_total.get(),
                                        metrics.rpc_errors_total.get()
                                    );
                                }
                                Err(e) => {
                                    metrics.rpc_errors_total.inc();
                                    error!("Failed to parse hex string '{}': {}", hex_str, e);
                                    println!("❌ Failed to parse hex string '{}': {}", hex_str, e);
                                }
                            },
                            None => {
                                metrics.rpc_errors_total.inc();
                                warn!("Field 'number' doesn't exist in response");
                                println!("⚠️  Field 'number' doesn't exist in response");
                            }
                        },
                        Err(e) => {
                            metrics.rpc_errors_total.inc();
                            error!("Failed to parse JSON: {}", e);
                            println!("❌ Failed to parse JSON: {}", e);
                            println!("Raw response: {}", text);
                        }
                    },
                    Err(error) => {
                        metrics.rpc_errors_total.inc();
                        error!("Error getting response text: {}", error);
                        println!("❌ Error getting response text: {}", error);
                    }
                }
            }
            Err(error) => {
                metrics.rpc_errors_total.inc();
                error!("HTTP request failed: {}", error);
                println!("❌ HTTP request failed: {}", error);
            }
        }
        
        thread::sleep(Duration::from_millis(6000));
    }
}