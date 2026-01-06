use substratheus::constants::METRICS_PREFIX;
use substratheus::helper::{initialize_metrics, rpc_manager};
use substratheus::http::{handle_metrics, State};
use substratheus::prometheus::Metrics;
use substratheus::utils::{Args, Config};

use async_ctrlc::CtrlC;
use async_std::{sync::RwLock, task};
use clap::Parser;
use prometheus_client::registry::Registry;
use std::sync::Arc;

#[async_std::main]
async fn main() -> tide::Result<()> {
    // parse CLI args
    let args = Args::parse();

    // load config
    let config = Config::load(&args.config).expect("Unable to parse config file!");

    // initialize registry
    let mut registry = Registry::default();

    // register prometheus metrics
    let metrics = Metrics::default();
    registry.register(
        format!("{METRICS_PREFIX}_era"),
        "Current era",
        metrics.era.clone(),
    );
    registry.register(
        format!("{METRICS_PREFIX}_active"),
        "Whether the validator is in the active set",
        metrics.active.clone(),
    );
    registry.register(
        format!("{METRICS_PREFIX}_era_points"),
        "Era points earned since the current era started",
        metrics.era_points.clone(),
    );
    registry.register(
        format!("{METRICS_PREFIX}_nominator_stake"),
        "Total amount staked by nominators",
        metrics.nominator_stake.clone(),
    );
    registry.register(
        format!("{METRICS_PREFIX}_nominator_count"),
        "Total number of nominators",
        metrics.nominator_count.clone(),
    );
    registry.register(
        format!("{METRICS_PREFIX}_minimum_active_stake"),
        "The minimum active nominator stake of the last successful election",
        metrics.minimum_active_stake.clone(),
    );
    registry.register(
        format!("{METRICS_PREFIX}_average_stake"),
        "The average amount staked till the current era",
        metrics.average_stake.clone(),
    );
    registry.register(
        format!("{METRICS_PREFIX}_asset_hub_rpc_health"),
        "Whether at least one RPC endpoint is healthy",
        metrics.asset_hub_rpc_health.clone(),
    );
    let state = State {
        config: Arc::new(config),
        registry: Arc::new(registry),
        metrics: Arc::new(metrics),
        rpc: Arc::new(RwLock::new(None)),
        shutdown: Arc::new(RwLock::new(false)),
    };
    task::spawn({
        let state = state.clone();
        async move {
            CtrlC::new().expect("Error setting Ctrl-C handler").await;

            log::info!("Shutdown signal received");

            *state.shutdown.write().await = true;
            *state.rpc.write().await = None;
        }
    });

    initialize_metrics(&state);

    tide::log::start();

    let mut app = tide::with_state(state.clone());

    app.at("/metrics").get(handle_metrics);

    task::spawn({
        let host = args.host.clone();

        let port = args.port;

        async move {
            app.listen(format!("{host}:{port}"))
                .await
                .expect("HTTP server failed");
        }
    });

    task::spawn(rpc_manager(state.clone()));

    // WORKERS
    task::spawn(substratheus::workers::chain_metrics_worker(state.clone()));

    for validator in state.config.validators.clone() {
        task::spawn(substratheus::workers::validator_metrics_worker(
            state.clone(),
            validator,
        ));
    }

    // future::pending::<()>().await;
    loop {
        if *state.shutdown.read().await {
            log::info!("Main loop exiting");
            break;
        }

        task::sleep(std::time::Duration::from_secs(1)).await;
    }
    Ok(())
}
