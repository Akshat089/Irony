use std::env;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::RwLock;

use axum::{
    routing::{get, post},
    Router,
};

mod routes;
mod state;

use routes::*;
use state::{AppState, SharedState};

use common::ring::HashRing;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let node_id = env::var("NODE_ID")
        .expect("Missing NODE_ID");

    let node_port = env::var("NODE_PORT")
        .expect("Missing NODE_PORT");

    let host = env::var("HOST")
        .expect("Missing HOST");

    let controller_addr = env::var("CONTROLLER_ADDR")
        .expect("Missing CONTROLLER_ADDR");

    let node_addr = format!("http://{}:{}", host, node_port);

    let bind_addr = format!("{}:{}", host, node_port);

    let state: SharedState = Arc::new(AppState {
        store: DashMap::new(),
        ring: Arc::new(RwLock::new(HashRing::new())),
        node_id,
        node_addr: node_addr.clone(),
        controller_addr,
    });

    let app = Router::new()
        .route("/v1/health", get(healthcheck))
        .route("/v1/keys", get(get_all_keys))
        .route(
            "/v1/keys/{key}",
            get(get_key).put(put_key),
        )
        .route("/v1/replicate", post(replicate))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind TCP listener");

    println!("Worker running on {}", node_addr);

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}