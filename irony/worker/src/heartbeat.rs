use tokio::time::{sleep, Duration};
use chrono::Utc;
use common::models::{HeartbeatRequest, HeartbeatResponse};
use crate::state::SharedState;
use crate::fetch_ring_state;

pub async fn run(state: SharedState) {
    loop {
        sleep(Duration::from_secs(2)).await;
        let version = {
            let v = state.ring_version.read().await;
            *v
        };
        let request = HeartbeatRequest {
            node_id: state.node_id.clone(),
            timestamp: Utc::now(),
            ring_version: version,
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

                // If controller says ring changed, fetch the latest ring state
                if hb_response.ring_changed {
                    println!("Ring changed detected via heartbeat. Refreshing ring state...");
                    fetch_ring_state(state.clone()).await;
                }
            }
            Ok(response) => {
                println!("Heartbeat failed with status: {}", response.status());
            }
            Err(e) => {
                // Do not panic — controller might be temporarily unreachable
                println!("Heartbeat could not reach controller: {}", e);
            }
        }
    }
}