use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum NodeStatus {
    Alive,
    Suspect,
    Dead,
}

#[derive(Serialize, Deserialize, Clone, Debug,PartialEq)]
pub struct NodeInfo {
    pub node_id: String,
    pub node_port: u16,
    pub status: NodeStatus,
    pub host: String,
    pub last_heartbeat: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RingState {
    pub nodes: Vec<NodeInfo>,
    pub virtual_nodes: Vec<(u64, String)>,
    pub replication_factor: u8,
    pub ring_version: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PutRequest {
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PutResponse {
    pub key: String,
    pub success: bool,
    pub replicas_confirmed: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetResponse {
    pub key: String,
    pub value: Option<String>,
    pub served_by: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeartbeatRequest {
    pub node_id: String,
    pub timestamp: DateTime<Utc>,
    pub ring_version: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeartbeatResponse {
    pub acknowledged: bool,
    pub ring_changed: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeRegisterRequest {
    pub node_id: String,
    pub host: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeRegisterResponse {
    pub success: bool,
    pub ring_state: RingState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReplicateRequest {
    pub key: String,
    pub value: String,
    pub origin_node_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReplicateToRequest {
    pub key: String,
    pub target_node_addr: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyDumpResponse {
    pub node_id: String,
    pub keys: HashMap<String, String>,
}