use clap::Parser;
use reqwest::Client;
use serde_json::{Value, json};
use std::i64;
use std::{thread, time::Duration};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]

struct Cli {
    #[arg(short, long)]
    url: Option<String>,

    #[arg(short, long)]
    port: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let client = Client::new();

    let rpc_url = cli
        .url
        .unwrap_or_else(|| "https://polkadot-rpc.publicnode.com".to_string());
    
    let rpc_port = cli.port.unwrap_or_else(|| "".to_string());

    let payload = json!({
        "jsonrpc": "2.0",
        "method": "chain_getHeader",
        "params": [],
        "id": 1
    });
    loop {
        let formatted_url = format!("{}{}", rpc_url, rpc_port);
        let response = client
            .post(formatted_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .unwrap();

        let response_text = response.text().await;

        match response_text {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(json) => match json["result"]["number"].as_str() {
                    Some(hex_str) => match i64::from_str_radix(&hex_str[2..], 16) {
                        Ok(height) => {
                            println!("Height: {}", height);
                        }
                        Err(e) => {
                            println!("Failed to parse hex string '{}': {}", hex_str, e);
                        }
                    },
                    None => {
                        println!("Field 'number' doesn't exist");
                    }
                },
                Err(e) => {
                    println!("Failed to parse JSON: {}", e);
                    println!("Raw response: {}", text);
                }
            },
            Err(error) => {
                println!("Error getting response text: {}", error);
            }
        }
        thread::sleep(Duration::from_millis(6000));
    }
}
