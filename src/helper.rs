use crate::http::State;
use crate::prometheus::Labels;
use crate::substrate::SubstrateRPC;

use async_std::task;
use std::sync::Arc;
use std::time::Duration;

pub fn initialize_metrics(state: &State) {
    // chain-level labels
    let chain_labels = Labels {
        network: serde_yaml::to_string(&state.config.network)
            .unwrap()
            .trim()
            .into(),
        chain: state.config.chain.clone(),
        validator_name: None,
        validator_address: None,
    };

    // initialize chain metrics
    state.metrics.era.get_or_create(&chain_labels).set(0);
    state
        .metrics
        .asset_hub_rpc_health
        .get_or_create(&chain_labels)
        .set(0);
    state
        .metrics
        .minimum_active_stake
        .get_or_create(&chain_labels)
        .set(0);
    state
        .metrics
        .average_stake
        .get_or_create(&chain_labels)
        .set(0);

    // validator-level metrics
    for v in state.config.validators.iter() {
        let validator_labels = Labels {
            network: chain_labels.network.clone(),
            chain: state.config.chain.clone(),
            validator_name: Some(v.name.clone()),
            validator_address: Some(v.address.clone()),
        };

        state.metrics.active.get_or_create(&validator_labels).set(0);
        state
            .metrics
            .era_points
            .get_or_create(&validator_labels)
            .set(0);
        state
            .metrics
            .nominator_stake
            .get_or_create(&validator_labels)
            .set(0);
        state
            .metrics
            .nominator_count
            .get_or_create(&validator_labels)
            .set(0);
    }
}

pub async fn rpc_manager(state: State) {
    let mut use_backup = false;
    let chain_labels = Labels {
        network: serde_yaml::to_string(&state.config.network)
            .unwrap()
            .trim()
            .into(),
        chain: state.config.chain.clone(),
        validator_name: None,
        validator_address: None,
    };
    loop {
        if *state.shutdown.read().await {
            log::info!("rpc_manager shutting down");
            break;
        }
        let rpc_url = if use_backup {
            &state.config.backup_rpc_url
        } else {
            &state.config.rpc_url
        };

        log::info!("Connecting to RPC: {}", rpc_url);

        match SubstrateRPC::new(state.config.network.clone(), rpc_url).await {
            Ok(rpc) => {
                {
                    let mut guard = state.rpc.write().await;
                    *guard = Some(Arc::new(rpc));
                }

                log::info!("RPC connected: {}", rpc_url);
                state
                    .metrics
                    .asset_hub_rpc_health
                    .get_or_create(&chain_labels)
                    .set(1);
                // stay alive until RPC fails
                loop {
                    task::sleep(Duration::from_secs(10)).await;

                    let healthy = {
                        let guard = state.rpc.read().await;
                        match guard.as_ref() {
                            Some(rpc) => rpc.get_current_era().await.is_some(),
                            None => false,
                        }
                    };

                    if !healthy {
                        log::warn!("RPC unhealthy: {}", rpc_url);
                        state
                            .metrics
                            .asset_hub_rpc_health
                            .get_or_create(&chain_labels)
                            .set(0);
                        break;
                    }
                }

                // drop RPC
                {
                    let mut guard = state.rpc.write().await;
                    *guard = None;
                }

                // switch RPC next time
                use_backup = !use_backup;
            }

            Err(e) => {
                log::warn!("RPC connection failed ({}): {}, retrying...", rpc_url, e);
                state
                    .metrics
                    .asset_hub_rpc_health
                    .get_or_create(&chain_labels)
                    .set(0);
                use_backup = !use_backup;
                task::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
