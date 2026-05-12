use std::env;
use std::sync::Arc;
use reqwest::Client;
use dashmap::DashMap;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use common::models::{NodeRegisterRequest, NodeRegisterResponse};
use common::models::RingState;
use common::ring::HashRing;
use axum::{
    routing::{get, post},
    Router,
};

mod routes;
mod state;
mod heartbeat;
mod replication;
use routes::*;
use state::{AppState, SharedState};

async fn register_with_controller(state: SharedState) {
    let reg_request = NodeRegisterRequest {
        node_id: state.node_id.clone(),
        host: state.node_addr
            .trim_start_matches("http://")
            .split(":")
            .next()
            .unwrap()
            .to_string(),
        port: state.node_addr
            .split(":")
            .last()
            .unwrap()
            .parse::<u16>()
            .unwrap(),
    };

    loop {
        println!("Attempting to register with controller at {}...", state.controller_addr);

        let result = state
            .http_client
            .post(format!("{}/v1/nodes/register", state.controller_addr))
            .json(&reg_request)
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                let reg_response: NodeRegisterResponse = response
                    .json()
                    .await
                    .expect("Failed to parse registration response");

                if reg_response.success {
                    let ring_version_from_controller = reg_response.ring_state.ring_version;
                    let mut ring = state.ring.write().await;
                    *ring = HashRing::from_ring_state(reg_response.ring_state);
                    {
                        let mut version = state.ring_version.write().await;
                        *version = ring_version_from_controller;
                    }
                    println!(
                        "Successfully registered with controller. Ring has {} nodes.",
                        ring.get_all_nodes().len()
                    );
                    return;
                } else {
                    println!("Controller rejected registration. Retrying in 2 seconds...");
                }
            }
            Ok(response) => {
                println!(
                    "Registration failed with status {}. Retrying in 2 seconds...",
                    response.status()
                );
            }
            Err(e) => {
                println!(
                    "Could not reach controller: {}. Retrying in 2 seconds...",
                    e
                );
            }
        }

        sleep(Duration::from_secs(2)).await;
    }
}

pub async fn fetch_ring_state(state: SharedState) {
    let result = state
        .http_client
        .get(format!("{}/v1/ring", state.controller_addr))
        .send()
        .await;

    match result {
        Ok(response) if response.status().is_success() => {
            let ring_state: RingState = response
                .json()
                .await
                .expect("Failed to parse ring state");
            let new_version = ring_state.ring_version;
            let mut ring = state.ring.write().await;
            *ring = HashRing::from_ring_state(ring_state);
            {
                let mut version = state.ring_version.write().await;
                *version = new_version;
            }
            println!(
                "Ring state refreshed. Ring now has {} nodes.",
                ring.get_all_nodes().len()
            );
        }
        Ok(response) => {
            println!("Failed to fetch ring state: HTTP {}", response.status());
        }
        Err(e) => {
            println!("Failed to fetch ring state: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let node_id = env::var("NODE_ID")
        .expect("Missing NODE_ID environment variable");

    let node_port = env::var("NODE_PORT")
        .expect("Missing NODE_PORT environment variable");

    let host = env::var("HOST")
        .expect("Missing HOST environment variable");

    let controller_addr = env::var("CONTROLLER_ADDR")
        .expect("Missing CONTROLLER_ADDR environment variable");

    let node_addr = format!("http://{}:{}", host, node_port);
    let bind_addr = format!("{}:{}", host, node_port);

    let state: SharedState = Arc::new(AppState {
        store: DashMap::new(),
        ring: Arc::new(RwLock::new(HashRing::new())),
        node_id,
        node_addr,
        controller_addr,
        http_client: Client::new(),
        ring_version: Arc::new(RwLock::new(0)),
    });

    register_with_controller(state.clone()).await;

    tokio::spawn(heartbeat::run(state.clone()));

    let app = Router::new()
        .route("/v1/health", get(healthcheck))
        .route("/v1/keys", get(get_all_keys))
        .route("/v1/keys/{key}", get(get_key).put(put_key))
        .route("/v1/replicate", post(replicate))
        .route("/v1/replicate-to", post(replicate_to))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind TCP listener");

    println!("Worker HTTP server listening on {}", bind_addr);

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}