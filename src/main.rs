use clap::Parser;
use env_logger::Env;
use log::{LevelFilter, error, info, warn};
use reqwest::Client;
use serde_json::{Value, json};
use std::i64;
use std::{thread, time::Duration};

mod models;
use models::Metrics;

mod http;
use http::start_metrics_server;

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

#[tokio::main]
async fn main() {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .filter_module("warp", LevelFilter::Warn)
        .init();

    let cli = Cli::parse();
    let client = Client::new();
    let metrics = std::sync::Arc::new(Metrics::new());

    let rpc_url = cli.url.unwrap_or_else(|| "".to_string());

    let rpc_port = cli.port.unwrap_or_else(|| "".to_string());

    info!("Starting Polkadot exporter service...");
    info!("RPC URL: {}{}", rpc_url, rpc_port);
    info!("Metrics port: {}", cli.metrics_port);

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

        let resp = match response {
            Ok(resp) => resp,
            Err(error) => {
                metrics.rpc_errors_total.inc();
                error!("HTTP request failed: {}", error);
                return; // ← IMPORTANT
            }
        };

        let response_text = resp.text().await;

        let text = match response_text {
            Ok(text) => text,
            Err(error) => {
                metrics.rpc_errors_total.inc();
                error!("Error getting response text: {}", error);
                return; // ← IMPORTANT
            }
        };
        
        let json = match serde_json::from_str::<Value>(&text) {
            Ok(json) => json,
            Err(e) => {
                metrics.rpc_errors_total.inc();
                error!("Failed to parse JSON: {}", e);
                return; // ← IMPORTANT
            }
        };

        let hex_str = match json["result"]["number"].as_str(){
            Some(hex_str) => hex_str,
            None => {
                                metrics.rpc_errors_total.inc();
                                warn!("Field 'number' doesn't exist in response");
                                return ; // ← IMPORTANT
            }
        };

        let height = match i64::from_str_radix(&hex_str[2..], 16) {
            Ok(height) => height,
            Err(e) => {
                metrics.rpc_errors_total.inc();
                error!("Failed to parse hex string '{}': {}", hex_str, e);
                return; // ← IMPORTANT
            }
        };
        metrics.block_height.set(height as f64);
        info!("Block height updated: {}", height);
        

        thread::sleep(Duration::from_millis(6000));
    }
}
