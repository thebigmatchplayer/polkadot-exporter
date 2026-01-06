use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;

// prometheus metrics label format
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct Labels {
    pub network: String,
    pub chain: String,
    pub validator_name: Option<String>,
    pub validator_address: Option<String>,
}

// prometheus metrics
#[derive(Clone, Default)]
pub struct Metrics {
    pub era: Family<Labels, Gauge>,
    pub active: Family<Labels, Gauge>,
    pub era_points: Family<Labels, Gauge>,
    pub nominator_stake: Family<Labels, Gauge>,
    pub nominator_count: Family<Labels, Gauge>,
    pub minimum_active_stake: Family<Labels, Gauge>,
    pub average_stake: Family<Labels, Gauge>,
    pub asset_hub_rpc_health: Family<Labels, Gauge>,
}
