use tokio::time::{sleep, Duration};
use chrono::Utc;
use common::models::{HeartbeatRequest, HeartbeatResponse};
use crate::state::SharedState;
use crate::fetch_ring_state;

pub async fn run(state: SharedState) {
    loop {
        sleep(Duration::from_secs(2)).await;

        let request = HeartbeatRequest {
            node_id: state.node_id.clone(),
            timestamp: Utc::now(),
        };

        let result = state
            .http_client
            .post(format!("{}/v1/heartbeat", state.controller_addr))
            .json(&request)
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                let hb_response: HeartbeatResponse = match response.json().await {
                    Ok(r) => r,
                    Err(e) => {
                        println!("Failed to parse heartbeat response: {}", e);
                        continue;
                    }
                };

                if hb_response.ring_changed {
                    println!("Ring changed detected via heartbeat. Refreshing ring state...");
                    fetch_ring_state(state.clone()).await;
                }
            }
            Ok(response) => {
                println!("Heartbeat failed with status: {}", response.status());
            }
            Err(e) => {
                println!("Heartbeat could not reach controller: {}", e);
            }
        }
    }
}