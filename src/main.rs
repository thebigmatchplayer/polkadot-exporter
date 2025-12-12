use reqwest::Client;
use serde_json::{Value, json};
use std::i64;
use std::{thread, time::Duration};


#[tokio::main]
async fn main() {
    let client = Client::new();
    const RPC_URL: &str = "https://polkadot-rpc.publicnode.com";
    const RPC_PORT: &str = "";

    let payload = json!({
        "jsonrpc": "2.0",
        "method": "chain_getHeader",
        "params": [],
        "id": 1
    });
    loop {
        let formatted_url = format!("{}{}", RPC_URL, RPC_PORT);
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
        thread::sleep(Duration::from_millis(4000));
    }
}
