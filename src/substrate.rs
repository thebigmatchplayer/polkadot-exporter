use crate::constants::Network;
use crate::utils::serialize_address;
use log::{info, warn};
use subxt::{
    dynamic::{DecodedValueThunk, Value},
    storage::{DefaultAddress, StorageKey},
    utils::{AccountId32, Yes},
    OnlineClient, PolkadotConfig,
};

// substrate rpc return types
pub type Era = u32;
pub type EraPoints = i64;
pub type Tokens = i128;

// convert i128 to i64
pub fn tokens_to_i64(tokens: Tokens) -> i64 {
    (tokens / 1e10 as i128) as i64
}

#[derive(Debug, scale_decode::DecodeAsType, Default)]
pub struct EraPointsMap {
    pub individual: Vec<(AccountId32, EraPoints)>,
}

impl EraPointsMap {
    pub fn default() -> Self {
        Self { individual: vec![] }
    }
}

#[derive(Debug, Default, scale_decode::DecodeAsType)]
pub struct NominatorSummary {
    pub total: Tokens,
    pub nominator_count: u32,
}

//new stuff_________________________
#[derive(Debug, scale_decode::DecodeAsType)]
pub struct ActiveEraInfo {
    pub index: Era,
    pub start: Option<u64>,
}

// substrate rpc actions
#[derive(Debug)]
pub struct SubstrateRPC {
    network: Network,
    client: OnlineClient<PolkadotConfig>,
}
impl SubstrateRPC {
    // instantiate a new substrate rpc client
    pub async fn new(
        network: Network,
        rpc_url: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = OnlineClient::<PolkadotConfig>::from_url(rpc_url).await?;
        Ok(Self { network, client })
    }

    async fn query_storage<T: StorageKey>(
        &self,
        query: DefaultAddress<T, DecodedValueThunk, Yes, Yes, Yes>,
    ) -> Option<DecodedValueThunk> {
        let storage = match self.client.storage().at_latest().await {
            Ok(s) => s,
            Err(e) => {
                warn!("Storage RPC unavailable: {e}");
                return None;
            }
        };

        match storage.fetch(&query).await {
            Ok(res) => res,
            Err(e) => {
                warn!("Storage query failed: {e}");
                None
            }
        }
    }

    // get ongoing era number
    // get ongoing era number
    pub async fn get_current_era(&self) -> Option<Era> {
        // We query "ActiveEra" instead of "CurrentEra"
        let query = subxt::dynamic::storage("Staking", "ActiveEra", ());

        if let Some(active_era) = self.query_storage(query).await {
            // We decode the result into the new struct we defined
            let active_era_info: ActiveEraInfo = match active_era.as_type() {
                Ok(v) => v,
                Err(e) => {
                    warn!("Failed to decode ActiveEra: {e}");
                    return None;
                }
            };

            // The era number is now in the 'index' field of the struct
            info!("Fetched current era: {}", active_era_info.index);
            Some(active_era_info.index)
        } else {
            warn!("Unable to fetch current era!");
            None
        }
    }

    // get era points earned in the given era by all validators
    pub async fn get_all_era_points(&self, era: Era) -> Option<EraPointsMap> {
        let query = subxt::dynamic::storage("Staking", "ErasRewardPoints", vec![era.into()]);

        if let Some(all_era_points) = self.query_storage(query).await {
            let all_era_points: EraPointsMap = all_era_points.as_type().unwrap();
            info!("Fetched all era points at era {era}");
            Some(all_era_points)
        } else {
            warn!("Unable to fetch era points at era {era}!");
            None
        }
    }

    // get the total nominator stake and count till the given era for the given account
    pub async fn get_nominator_summary(
        &self,
        era: Era,
        account_id: &AccountId32,
    ) -> Option<NominatorSummary> {
        let query = subxt::dynamic::storage(
            "Staking",
            "ErasStakersOverview",
            vec![era.into(), Value::from_bytes(account_id)],
        );

        if let Some(nominator_summary) = self.query_storage(query).await {
            let nominator_summary: NominatorSummary = nominator_summary.as_type().unwrap();
            info!(
                "Fetched nominator summary for {} at era {era}",
                serialize_address(&self.network, account_id)
            );
            Some(nominator_summary)
        } else {
            warn!(
                "Unable to fetch nominator summary for {} at era {era}!",
                serialize_address(&self.network, account_id)
            );
            None
        }
    }

    // get the minimum active stake of the last successful election
    pub async fn get_minimum_active_stake(&self) -> Option<Tokens> {
        let query = subxt::dynamic::storage("Staking", "MinimumActiveStake", ());

        if let Some(minimum_active_stake) = self.query_storage(query).await {
            let minimum_active_stake: Tokens = minimum_active_stake.as_type().unwrap();
            info!("Fetched minimum active stake: {minimum_active_stake}");
            Some(minimum_active_stake)
        } else {
            warn!("Unable to fetch minimum active stake!");
            None
        }
    }

    // get the total stake till the given era
    pub async fn get_total_stake(&self, era: Era) -> Option<Tokens> {
        let query = subxt::dynamic::storage("Staking", "ErasTotalStake", vec![era.into()]);

        if let Some(total_stake) = self.query_storage(query).await {
            let total_stake: Tokens = total_stake.as_type().unwrap();
            info!("Fetched total stake: {total_stake}");
            Some(total_stake)
        } else {
            warn!("Unable to fetch total stake!");
            None
        }
    }
}
