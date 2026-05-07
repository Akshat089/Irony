#![allow(dead_code)]
#![allow(unused_imports)]

mod heartbeat;
mod replication;
mod routes;
mod state;

#[tokio::main]
async fn main() {
    // Phase 7 — controller startup, axum server init, background task spawns
    println!("IronRing Controller starting...");
}
