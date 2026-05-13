use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use common::models::NodeInfo;
use common::ring::HashRing;

pub struct AppState {
    pub ring: Arc<RwLock<HashRing>>,
    pub nodes: Arc<RwLock<HashMap<String,NodeInfo>>>, //registry of all knows workers keyed by node_id
    pub last_heartbeat: Arc<RwLock<HashMap<String,chrono::DateTime<chrono::Utc>>>>, //last heartbeat timestamp keyed by node_id
    pub http_client: reqwest::Client, //shared HTTP client for making requests to workers
    pub ring_version: Arc<RwLock<u64>>, //version number to track ring state changes
    pub ring_changed: Arc<RwLock<bool>>, //flag to indicate if ring has changed since last heartbeat
}
pub type SharedState = Arc<AppState>;