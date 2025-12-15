use prometheus::{Counter, Gauge, Registry, Encoder, TextEncoder};

pub struct Metrics {
    pub registry: Registry,
    pub block_height: Gauge,
    pub rpc_requests_total: Counter,
    pub rpc_errors_total: Counter,
}

impl Metrics {
    pub fn new() -> Self {
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
    
    pub fn get_metrics_string(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).expect("Failed to encode metrics");
        String::from_utf8(buffer).expect("Failed to convert metrics to string")
    }
}