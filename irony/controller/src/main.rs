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

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    println!("Controller running on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}