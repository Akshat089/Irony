
use common::models::{
    ReplicateRequest,
};

pub async fn replicate_to_node(key: String, value: String, target_node_addr: String,http_client: reqwest::Client,origin_node_id: String) -> Result<(),String>{
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
                Ok(())
            }
            else{
                println!("Failed to replicate key {} to {}. Status: {}", key, target_node_addr, response.status());
                Err("Failed to replicate key".into())
            }
        }
        Err(e) => {
            println!("Error replicating key {} to {}: {}", key, target_node_addr, e);
            Err("Error replicating key".into())
        }
    }
}
