use common::models::{
    ReplicateRequest,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
pub async fn replicate_to_node(key: String, value: String, target_node_addr: String,http_client: reqwest::Client,origin_node_id: String, replication_success: Arc<AtomicU64>, replication_failures: Arc<AtomicU64> ) -> Result<(),String>{
    let rep_req = ReplicateRequest{
        key: key.clone(),
        value: value.clone(),
        origin_node_id: origin_node_id.clone(),
    };
    let result = http_client.post(format!("{}/v1/replicate",target_node_addr))
        .json(&rep_req)
        .send()
        .await;
    match result{
        Ok(response) => {
            if response.status().is_success(){
                println!("Successfully replicated key {} to {}", key, target_node_addr);
                replication_success.fetch_add(1, Ordering::Relaxed);

                Ok(())
            }
            else{
                println!("Failed to replicate key {} to {}. Status: {}", key, target_node_addr, response.status());
                replication_failures.fetch_add(1, Ordering::Relaxed);
                Err("Failed to replicate key".into())
            }
        }
        Err(e) => {
            println!("Error replicating key {} to {}: {}", key, target_node_addr, e);
            replication_failures.fetch_add(1, Ordering::Relaxed);
            Err("Error replicating key".into())
        }
    }
}
