use crate::http::State;
use crate::prometheus::Labels;
use crate::substrate::{tokens_to_i64, EraPointsMap};
use crate::utils::Validator;

use async_std::task;
use std::sync::Arc;
use std::time::Duration;
use subxt::utils::AccountId32;

const SCRAPE_INTERVAL: u64 = 15;

async fn _wait_for_rpc(state: &State) -> Arc<crate::substrate::SubstrateRPC> {
    loop {
        if let Some(rpc) = state.rpc.read().await.clone() {
            return rpc;
        }

        task::sleep(Duration::from_secs(2)).await;
    }
}

// Chain-level metrics worker
pub async fn chain_metrics_worker(state: State) {
    let labels = Labels {
        network: serde_yaml::to_string(&state.config.network)
            .unwrap()
            .trim()
            .into(),
        chain: state.config.chain.clone(),
        validator_name: None,
        validator_address: None,
    };
    // let rpc = wait_for_rpc(&state).await;
    loop {
        if *state.shutdown.read().await {
            log::info!("chain_metrics_worker shutting down");
            break;
        }
        let rpc_opt = state.rpc.read().await.clone();
        match rpc_opt {
            Some(rpc) => {
                if let Some(mut era) = rpc.get_current_era().await {
                    state.metrics.era.get_or_create(&labels).set(era.into());
                    let era_points = match rpc.get_all_era_points(era).await {
                        Some(res) => res,
                        // if fetching from current era fails, try fetching from previous era
                        None => match rpc.get_all_era_points(era - 1).await {
                            Some(res) => {
                                era -= 1;
                                res
                            }
                            None => EraPointsMap::default(),
                        },
                    };

                    let active_count = era_points.individual.len() as i64;

                    if let Some(min) = rpc.get_minimum_active_stake().await {
                        state
                            .metrics
                            .minimum_active_stake
                            .get_or_create(&labels)
                            .set(tokens_to_i64(min));
                    }

                    if let Some(total) = rpc.get_total_stake(era).await {
                        if active_count > 0 {
                            state
                                .metrics
                                .average_stake
                                .get_or_create(&labels)
                                .set(tokens_to_i64(total) / active_count);
                        }
                    }
                }
            }
            None => {
                // RPC DOWN → RESET TO DEFAULTS
                state.metrics.era.get_or_create(&labels).set(0);
                state
                    .metrics
                    .minimum_active_stake
                    .get_or_create(&labels)
                    .set(0);
                state.metrics.average_stake.get_or_create(&labels).set(0);
            }
        }

        task::sleep(Duration::from_secs(SCRAPE_INTERVAL)).await;
    }
}

/// Validator-level metrics worker (one per validator)
pub async fn validator_metrics_worker(state: State, validator: Validator) {
    // let rpc = wait_for_rpc(&state).await;

    let account_id: AccountId32 = validator
        .address
        .parse()
        .expect("Invalid validator address");

    let labels = Labels {
        network: serde_yaml::to_string(&state.config.network)
            .unwrap()
            .trim()
            .into(),
        chain: state.config.chain.clone(),
        validator_name: Some(validator.name.clone()),
        validator_address: Some(validator.address.clone()),
    };

    loop {
        if *state.shutdown.read().await {
            log::info!("chain_metrics_worker shutting down");
            break;
        }
        // RPC availability check
        let rpc_opt = state.rpc.read().await.clone();

        if rpc_opt.is_none() {
            // RPC DOWN → RESET ALL METRICS TO DEFAULTS
            state.metrics.active.get_or_create(&labels).set(0);
            state.metrics.era_points.get_or_create(&labels).set(0);
            state.metrics.nominator_stake.get_or_create(&labels).set(0);
            state.metrics.nominator_count.get_or_create(&labels).set(0);

            task::sleep(Duration::from_secs(SCRAPE_INTERVAL)).await;
            continue;
        }
        let rpc = rpc_opt.unwrap();

        // determine active era
        let active_era = rpc.get_current_era().await.unwrap_or(0);

        // find a finalized era using nominator data

        let mut effective_era = None;

        if active_era > 0 {
            for era in [active_era, active_era - 1] {
                if rpc.get_nominator_summary(era, &account_id).await.is_some() {
                    effective_era = Some(era);
                    break;
                }
            }
        }

        // era points

        let mut active = false;
        let mut points = 0;

        if let Some(effective) = effective_era {
            if let Some(points_era) = effective.checked_sub(1) {
                if let Some(map) = rpc.get_all_era_points(points_era).await {
                    if let Some((_, p)) = map.individual.iter().find(|(id, _)| id == &account_id) {
                        active = true;
                        points = *p;
                    }
                }
            }
        }

        // publish activity metrics
        state
            .metrics
            .active
            .get_or_create(&labels)
            .set(active.into());

        state.metrics.era_points.get_or_create(&labels).set(points);

        // publish nominator metrics
        let summary = if let Some(era) = effective_era {
            rpc.get_nominator_summary(era, &account_id)
                .await
                .unwrap_or_default()
        } else {
            Default::default()
        };

        state
            .metrics
            .nominator_stake
            .get_or_create(&labels)
            .set(tokens_to_i64(summary.total));

        state
            .metrics
            .nominator_count
            .get_or_create(&labels)
            .set(summary.nominator_count.into());

        task::sleep(Duration::from_secs(SCRAPE_INTERVAL)).await;
    }
}
