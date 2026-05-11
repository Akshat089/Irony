use axum::{Router, routing::{post, get}};
use std::net::SocketAddr;

mod routes;

use routes::*;

#[tokio::main]
async fn main() {
    println!("IronRing Controller starting...");

    let app = Router::new()
        .route("/v1/nodes/register", post(register_node))
        .route("/v1/heartbeat", post(get_heartbeat))
        .route("/v1/ring", get(get_ring));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:9090")
        .await
        .expect("Failed to bind port 9090");

    println!("Controller listening on http://127.0.0.1:9090");

    axum::serve(listener, app)
        .await
        .expect("Controller server failed");
}