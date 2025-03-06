use std::sync::Arc;
use std::time::Duration;

use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::Result;
use e2e_helpers::{
    fuel_node_simulated::{Compressibility, FuelNode, SimulationConfig},
    whole_stack::{
        create_and_fund_kms_keys, deploy_contract, start_avail_committer, start_db,
        start_eigen_committer, start_eth, start_kms,
    },
};
use serde::Deserialize;
use tokio::sync::Mutex;

mod data;
mod handlers;
mod template;

#[actix_web::main]
async fn main() -> Result<()> {
    let simulation_config = Arc::new(Mutex::new(SimulationConfig::new(
        150_000,
        Compressibility::Medium,
    )));

    let mut fuel_node = FuelNode::new(4000, simulation_config.clone());
    fuel_node.run().await?;

    let logs = false;
    let kms = start_kms(logs).await?;
    let eth_node = start_eth(logs).await?;
    let (main_key, _) = create_and_fund_kms_keys(&kms, &eth_node).await?;
    // let eigen_key = "8b449ef1d1a5297e7e14656d424f1c8710403ca0b0389d5001f11bbf844101ea".to_string();
    let avail_key = "//Alice".to_string();
    let request_timeout = Duration::from_secs(50);
    let max_fee = 1_000_000_000_000;
    let (_contract_args, deployed_contract) =
        deploy_contract(&eth_node, &main_key, max_fee, request_timeout).await?;
    let db = start_db().await?;

    let logs = true;
    let committer = start_avail_committer(
        logs,
        db.clone(),
        &eth_node,
        &fuel_node.url(),
        &deployed_contract,
        &main_key,
        avail_key,
    )
    .await?;

    let app_data = web::Data::new(data::AppData {
        simulation_config: simulation_config.clone(),
        metrics_url: committer.metrics_url().to_string(),
    });

    println!("Control panel available at http://localhost:3030");

    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .route("/", web::get().to(handlers::serve_control_panel))
            .route("/update", web::post().to(handlers::update_config))
            .route("/proxy/metrics", web::get().to(handlers::proxy_metrics))
    })
    .bind("0.0.0.0:3030")?
    .run()
    .await?;

    Ok(())
}
