use crate::{constants, constants::Network};

use log::info;
use std::fs;

use base58::ToBase58;
use blake2::{Blake2b512, Digest};

use clap::Parser;
use serde::Deserialize;
use subxt::utils::AccountId32;

// argument parser format
#[derive(Parser, Debug, Clone)]
#[command(version)]
pub struct Args {
    #[arg(short, long)]
    pub config: String,

    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,

    #[arg(long, default_value_t = 8000)]
    pub port: u32,
}

// config parser format
#[derive(Deserialize, Debug, Clone)]
pub struct Validator {
    pub name: String,
    pub address: String,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub network: Network,
    pub chain: String,
    pub rpc_url: String,
    pub backup_rpc_url: String,
    pub validators: Vec<Validator>,
}
impl Config {
    // load config from file
    pub fn load(config_file: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_str: String =
            fs::read_to_string(config_file).expect("Unable to read config file!");
        let config: Config = serde_yaml::from_str(&config_str)?;
        info!("Loaded config: {:?}", config.clone());
        Ok(config)
    }
}

// serialize AccountId32 into a Substrate Address
pub fn serialize_address(network: &Network, account_id: &AccountId32) -> String {
    let prefix: u8 = match network {
        Network::Polkadot => constants::POLKADOT_ADDR_PREFIX,
        Network::Kusama => constants::KUSAMA_ADDR_PREFIX,
        Network::Avail => constants::AVAIL_ADDR_PREFIX,
    };

    let mut v = vec![prefix];
    v.extend(account_id.0);
    const PREFIX: &[u8] = b"SS58PRE";
    let mut ctx = Blake2b512::new();
    ctx.update(PREFIX);
    ctx.update(&v);
    let r = ctx.finalize().to_vec();
    v.extend(&r[0..2]);
    v.to_base58()
}
