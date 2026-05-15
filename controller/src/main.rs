use axum::{Router, routing::{post, get}};
use tokio::sync::RwLock;
use std::sync::Arc;
mod state;
use state::{AppState, SharedState};
use common::ring::HashRing;
mod routes;
mod heartbeat;
mod replication;
use routes::*;
use std::sync::atomic::AtomicU64;
#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    println!("IronRing Controller starting...");

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("NODE_PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("{}:{}", host, port);

    let state: SharedState = Arc::new(AppState {
        ring: Arc::new(RwLock::new(HashRing::new())),
        nodes: Arc::new(RwLock::new(std::collections::HashMap::new())),
        last_heartbeat: Arc::new(RwLock::new(std::collections::HashMap::new())),
        http_client: reqwest::Client::new(),
        ring_version: Arc::new(RwLock::new(0)),
        re_replication_count: Arc::new(AtomicU64::new(0)),
        started_at: chrono::Utc::now(),
        last_replication_node: Arc::new(RwLock::new(None)),
        last_replication_at: Arc::new(RwLock::new(None)),
        last_replication_keys_success: Arc::new(AtomicU64::new(0)),
        last_replication_keys_failed: Arc::new(AtomicU64::new(0)),
    });

    tokio::spawn(heartbeat::run(state.clone()));

    let app = Router::new()
        .route("/v1/nodes/register", post(register_node))
        .route("/v1/heartbeat", post(get_heartbeat))
        .route("/v1/ring", get(get_ring))
        .route("/v1/nodes", get(get_nodes))
        .route("/v1/health", get(get_health))
        .route("/v1/metrics", get(get_metrics))
        .route("/v1/status", get(get_status))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind");

    println!("Controller listening on http://{}", bind_addr);

    axum::serve(listener, app)
        .await
        .expect("Controller server failed");
}