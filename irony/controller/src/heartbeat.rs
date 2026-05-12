use tokio::time::{sleep, Duration};
use chrono::Utc;
use common::models::NodeStatus;
use crate::state::SharedState;

pub async fn run(state: SharedState) {
    loop {
        sleep(Duration::from_secs(5)).await;

        let now = Utc::now();
        let mut nodes_to_mark_dead: Vec<String> = vec![];
        let mut nodes_to_mark_suspect: Vec<String> = vec![];

        {
            let heartbeats = state.last_heartbeat.read().await;
            let nodes = state.nodes.read().await;

            for (node_id, last_seen) in heartbeats.iter() {
                let elapsed = (now - *last_seen).num_seconds();
                
                if let Some(node) = nodes.get(node_id) {
                    match node.status {
                        NodeStatus::Alive if elapsed > 6 => {
                            nodes_to_mark_suspect.push(node_id.clone());
                            println!(
                                "Node {} transitioned Alive -> Suspect after {}s",
                                node_id, elapsed
                            );
                        }
                        NodeStatus::Suspect if elapsed > 12 => {
                            nodes_to_mark_dead.push(node_id.clone());
                            println!(
                                "Node {} transitioned Suspect -> Dead after {}s",
                                node_id, elapsed
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        if !nodes_to_mark_suspect.is_empty() {
            let mut nodes = state.nodes.write().await;
            for node_id in &nodes_to_mark_suspect {
                if let Some(node) = nodes.get_mut(node_id) {
                    node.status = NodeStatus::Suspect;
                }
            }
        }

        if !nodes_to_mark_dead.is_empty() {
            for node_id in &nodes_to_mark_dead {
                {
                    let mut ring = state.ring.write().await;
                    ring.remove_node(node_id);
                }
                {
                    let mut nodes = state.nodes.write().await;
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.status = NodeStatus::Dead;
                    }
                }

                {
                    let mut version = state.ring_version.write().await;
                    *version += 1;
                    println!(
                        "Ring rebuilt after {} died. New ring version: {}",
                        node_id, *version
                    );
                }

                println!("Re-replication would be triggered for {}", node_id);
            }
        }
    }
}