use serde::{Deserialize, Serialize};

// supported networks
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Network {
    Polkadot,
    Kusama,
    Avail,
}

// ss58 address prefixes
pub static POLKADOT_ADDR_PREFIX: u8 = 0;
pub static KUSAMA_ADDR_PREFIX: u8 = 2;
pub static AVAIL_ADDR_PREFIX: u8 = 42;

// prometheus metrics prefix
pub static METRICS_PREFIX: &str = "substratheus";
