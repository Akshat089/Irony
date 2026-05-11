use axum::Json;
use serde_json::json;
use common::models::{
    GetResponse,
    KeyDumpResponse,
    NodeRegisterRequest,
    NodeRegisterResponse,
    RingState,
    HeartbeatRequest,
    HeartbeatResponse,
     PutRequest,
     PutResponse,
     ReplicateRequest,};
pub async fn register_node(
    Json(req): Json<NodeRegisterRequest>,
) -> Json<NodeRegisterResponse> {
    println!("REGISTER HIT: {:?}", req.node_id);

    Json(NodeRegisterResponse {
        success: true,
        ring_state: RingState {
            nodes: vec![],
            virtual_nodes: vec![],
            replication_factor: 3,
        },
    })
}
pub async fn get_heartbeat(Json(req): Json<HeartbeatRequest>,)-> Json<HeartbeatResponse>{
    println!("HEARTBEAT HIT: {:?}", req.node_id);
    Json(HeartbeatResponse{
        acknowledged: true,
        ring_changed: false,
    })
}
pub async fn get_ring() -> Json<RingState> {
    Json(RingState {
        nodes: vec![],
        virtual_nodes: vec![],
        replication_factor: 3,
    })
}