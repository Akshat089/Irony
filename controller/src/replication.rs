use std::collections::HashMap;
use common::ring::HashRing;
use common::models::{NodeStatus, KeyDumpResponse};
use crate::state::SharedState;
use reqwest;

pub async fn trigger_re_replication(
    state: SharedState,
    failed_node_id: String,
    old_ring: HashRing,
) {
    println!("Starting re-replication for failed node: {}", failed_node_id);

    let surviving_nodes: Vec<(String, String, u16)> = {
        state.nodes.read().await
            .iter()
            .filter(|(_, node)| node.status != NodeStatus::Dead)
            .map(|(id, node)| (id.clone(), node.host.clone(), node.node_port))
            .collect()
    };

    if surviving_nodes.is_empty() {
        println!("No surviving nodes — cannot re-replicate.");
        return;
    }

    let mut tasks = vec![];
    for (node_id, host, port) in &surviving_nodes {
        let client = state.http_client.clone();
        let addr = format!("http://{}:{}", host, port);
        let nid = node_id.clone();
        tasks.push(tokio::spawn(async move {
            let result = client
                .get(format!("{}/v1/keys", addr))
                .send()
                .await;
            match result {
                Ok(resp) if resp.status().is_success() => {
                    let dump: KeyDumpResponse = resp
                        .json()
                        .await
                        .unwrap_or_else(
                            |_| KeyDumpResponse {
                                node_id: nid.clone(),
                                keys: HashMap::new(),
                            }
                        );
                    Some((nid, addr, dump.keys))
                }
                _ => {
                    println!("Failed to fetch keys from {}", nid);
                    None
                }
            }
        }));
    }

    let mut node_keys: Vec<(String, String, HashMap<String, String>)> = vec![];
    for task in tasks {
        if let Ok(Some(result)) = task.await {
            node_keys.push(result);
        }
    }

    
    let mut affected_keys: HashMap<String, String> = HashMap::new();

    for (_, _, keys) in &node_keys {
        for (key, value) in keys {
            let primary = old_ring.find_primary(key);
            let replicas = old_ring.find_replicas(key);

            let was_primary = primary
                .as_ref()
                .map(|n| n.node_id == failed_node_id)
                .unwrap_or(false);

            let was_replica = replicas
                .iter()
                .any(|n| n.node_id == failed_node_id);

            if was_primary || was_replica {
                affected_keys.insert(key.clone(), value.clone());
            }
        }
    }

    println!(
        "{} keys affected by failure of {}",
        affected_keys.len(),
        failed_node_id
    );

    // Step 4 — for each affected key, find source and target
    let mut re_replicated = 0;
    let mut failed = 0;

    for (key, _) in &affected_keys {
        let holders: Vec<(String, String)> = node_keys
            .iter()
            .filter(|(_, _, keys)| keys.contains_key(key))
            .map(|(nid, addr, _)| (nid.clone(), addr.clone()))
            .collect();

        if holders.is_empty() {
            println!("CRITICAL: Key {} has no surviving copies — unrecoverable", key);
            failed += 1;
            continue;
        }

        let new_ring = state.ring.read().await;
        let new_primary = new_ring.find_primary(key);
        let new_replicas = new_ring.find_replicas(key);

        let mut new_owners: Vec<String> = vec![];
        if let Some(p) = &new_primary {
            new_owners.push(p.node_id.clone());
        }
        for r in &new_replicas {
            new_owners.push(r.node_id.clone());
        }
        drop(new_ring);

        let holder_ids: Vec<String> = holders.iter().map(|(id, _)| id.clone()).collect();
        let targets: Vec<String> = new_owners
            .iter()
            .filter(|id| !holder_ids.contains(id))
            .cloned()
            .collect();

        if targets.is_empty() {
            continue;
        }

        let (source_id, source_addr) = &holders[0];

        for target_id in &targets {
            let target_addr = surviving_nodes
                .iter()
                .find(|(id, _, _)| id == target_id)
                .map(|(_, host, port)| format!("http://{}:{}", host, port));

            match target_addr {
                Some(addr) => {
                    let result = state.http_client
                        .post(format!("{}/v1/replicate-to", source_addr))
                        .json(&serde_json::json!({
                            "key": key,
                            "target_node_addr": addr
                        }))
                        .send()
                        .await;

                    match result {
                        Ok(r) if r.status().is_success() => {
                            println!(
                                "Re-replicated key {} from {} to {}",
                                key, source_id, target_id
                            );
                            re_replicated += 1;
                        }
                        _ => {
                            println!(
                                "Failed to re-replicate key {} from {} to {}",
                                key, source_id, target_id
                            );
                            failed += 1;
                        }
                    }
                }
                None => {
                    println!("Target node {} not found in surviving nodes", target_id);
                    failed += 1;
                }
            }
        }
    }

    println!(
        "Re-replication complete. Keys re-replicated: {}, Failed: {}",
        re_replicated, failed
    );
}