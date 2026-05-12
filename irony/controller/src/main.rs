use axum::{Router, routing::{post, get}};
use tokio::sync::RwLock;
use std::sync::Arc;
mod state;
use state::{AppState, SharedState};
use common::ring::HashRing;
mod routes;

use routes::*;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    println!("IronRing Controller starting...");
    let state: SharedState = Arc::new(
        AppState {
            ring: Arc::new(RwLock::new(HashRing::new())),
            nodes: Arc::new(RwLock::new(std::collections::HashMap::new())),
            last_heartbeat: Arc::new(RwLock::new(std::collections::HashMap::new())),
            http_client: reqwest::Client::new(),
            ring_version: Arc::new(RwLock::new(0)),
        }
    );

    let app = Router::new()
        .route("/v1/nodes/register", post(register_node))
        .route("/v1/heartbeat", post(get_heartbeat))
        .route("/v1/ring", get(get_ring))
        .route("/v1/nodes", get(get_nodes))
        .route("/v1/health", get(get_health))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:9090")
        .await
        .expect("Failed to bind port 9090");

    println!("Controller listening on http://127.0.0.1:9090");

    axum::serve(listener, app)
        .await
        .expect("Controller server failed");
}