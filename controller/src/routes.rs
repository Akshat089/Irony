use axum::Json;
use serde_json::json;
use axum::extract::State;
use crate::SharedState;
use common::models::NodeStatus;
use common::models::{
    NodeRegisterRequest,
    NodeRegisterResponse,
    RingState,
    HeartbeatRequest,
    HeartbeatResponse,
    NodeInfo,
    ControllerMetrics,
};
use chrono::Utc;
use std::sync::atomic::Ordering;
pub async fn register_node(
    State(state): State<SharedState>,
    Json(req): Json<NodeRegisterRequest>,
) -> Json<NodeRegisterResponse> {
    println!("REGISTER HIT: {:?}", req.node_id);

    let node_info = NodeInfo {
        node_id: req.node_id.clone(),
        node_port: req.port,
        status: NodeStatus::Alive,
        host: req.host.clone(),
        last_heartbeat: None,
    };

    {
        let mut ring = state.ring.write().await;
        ring.add_node(node_info.clone());
    } 

    {
        let mut write_nodes = state.nodes.write().await;
        write_nodes.insert(req.node_id.clone(), node_info);
    }

    {
        let mut write_heartbeats = state.last_heartbeat.write().await;
        write_heartbeats.insert(req.node_id.clone(), chrono::Utc::now());
    }

    let current_version = {
        let mut version = state.ring_version.write().await;
        *version += 1;
        *version
    };

    // Now safe to read — write lock is already dropped
    let ring_state = {
        let ring = state.ring.read().await;
        ring.to_ring_state(current_version)
    };

    println!(
        "Node {} registered. Ring now has {} nodes.",
        req.node_id,
        ring_state.nodes.len()
    );

    Json(NodeRegisterResponse {
        success: true,
        ring_state,
    })
}
pub async fn get_heartbeat(
    State(state): State<SharedState>,
    Json(req): Json<HeartbeatRequest>,
) -> Json<HeartbeatResponse> {
    let now = chrono::Utc::now();
    
    // Update the separate heartbeat timestamp map
    {
        let mut heartbeats = state.last_heartbeat.write().await;
        heartbeats.insert(req.node_id.clone(), now);
    }

    {
        let mut nodes = state.nodes.write().await;
        if let Some(node) = nodes.get_mut(&req.node_id) {
            node.last_heartbeat = Some(now);
            node.status = NodeStatus::Alive; 
        }
    }

    let current_version = {
        let version = state.ring_version.read().await;
        *version
    };

    let ring_changed = current_version != req.ring_version;

    println!("HEARTBEAT HIT: {:?}", req.node_id);

    Json(HeartbeatResponse {
        acknowledged: true,
        ring_changed,
    })
}
pub async fn get_ring(
    State(state): State<SharedState>,
) -> Json<RingState> {
    let version = {
        let ring_version = state.ring_version.read().await;
        *ring_version
    };

    let ring = state.ring.read().await;

    Json(ring.to_ring_state(version))
}
pub async fn get_nodes(State(state): State<SharedState>) -> Json<Vec<NodeInfo>> {
    let read_nodes = state.nodes.read().await;
    let nodes: Vec<NodeInfo> = read_nodes.values().cloned().collect();
    drop(read_nodes);
    Json(nodes)
}
pub async fn get_health(State(state): State<SharedState>,) -> Json<serde_json::Value> {

    Json(json!({
        "status": "ok"
    }))
}

pub async fn get_metrics(
    State(state): State<SharedState>,
) -> Json<ControllerMetrics> {
    let nodes = state.nodes.read().await;
    let total = nodes.len() as u64;
    let alive = nodes.values().filter(|n| n.status == NodeStatus::Alive).count() as u64;
    let suspect = nodes.values().filter(|n| n.status == NodeStatus::Suspect).count() as u64;
    let dead = nodes.values().filter(|n| n.status == NodeStatus::Dead).count() as u64;
    drop(nodes);

    let ring_version = *state.ring_version.read().await;
    let uptime = (Utc::now() - state.started_at).num_seconds() as u64;

    Json(ControllerMetrics {
        total_nodes: total,
        alive_nodes: alive,
        suspect_nodes: suspect,
        dead_nodes: dead,
        ring_version,
        re_replication_count: state.re_replication_count.load(Ordering::Relaxed),
        uptime_seconds: uptime,
    })
}