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
    NodeInfo,};
pub async fn register_node(
    State(state): State<SharedState>,
    Json(req): Json<NodeRegisterRequest>,
) -> Json<NodeRegisterResponse> {
    println!("REGISTER HIT: {:?}", req.node_id);
    let node_info = NodeInfo{
        node_id : req.node_id.clone(),
        node_port : req.port,
        status : NodeStatus::Alive,
        host : req.host.clone(),    
        last_heartbeat : None,
    };
    let mut ring = state.ring.write().await;
    ring.add_node(node_info.clone());
    let mut write_nodes = state.nodes.write().await;
    write_nodes.insert(req.node_id.clone(), node_info);
    drop(write_nodes);
    let mut write_heartbeats = state.last_heartbeat.write().await;
    write_heartbeats.insert(req.node_id.clone(), chrono::Utc::now());
    drop(write_heartbeats);
    let mut version = state.ring_version.write().await;
    *version += 1;
    let ring_state = ring.to_ring_state();
    Json(NodeRegisterResponse {
        success: true,
        ring_state,
    })
}
pub async fn get_heartbeat(State(state): State<SharedState>, Json(req): Json<HeartbeatRequest>,)-> Json<HeartbeatResponse>{
    println!("HEARTBEAT HIT: {:?}", req.node_id);
    let mut write_heartbeats = state.last_heartbeat.write().await;
    write_heartbeats.insert(req.node_id.clone(), req.timestamp);
    let read_nodes = state.nodes.read().await;
    if !read_nodes.contains_key(&req.node_id) {
        println!(
            "WARNING: heartbeat received from unknown node {}",
            req.node_id
        );
    }
    drop(read_nodes);
    drop(write_heartbeats);
    let ring_ver = state.ring_version.read().await;
    Json(HeartbeatResponse{
        acknowledged: true,
        ring_changed: true,
    })
}
pub async fn get_ring(State(state): State<SharedState>) -> Json<RingState> {
    let ring = state.ring.read().await;
    let ring_state = ring.to_ring_state();
    drop(ring);
    Json(ring_state)    
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