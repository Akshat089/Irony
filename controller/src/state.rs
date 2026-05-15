use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use common::models::NodeInfo;
use common::ring::HashRing;
use std::sync::atomic::AtomicU64;
use chrono::{DateTime, Utc};
pub struct AppState {
    pub ring: Arc<RwLock<HashRing>>,
    pub nodes: Arc<RwLock<HashMap<String,NodeInfo>>>, //registry of all knows workers keyed by node_id
    pub last_heartbeat: Arc<RwLock<HashMap<String,chrono::DateTime<chrono::Utc>>>>, //last heartbeat timestamp keyed by node_id
    pub http_client: reqwest::Client, //shared HTTP client for making requests to workers
    pub ring_version: Arc<RwLock<u64>>, //version number to track ring state changes

    pub re_replication_count: Arc<AtomicU64>,
    pub started_at: DateTime<Utc>,
    pub last_replication_node: Arc<RwLock<Option<String>>>,
    pub last_replication_at: Arc<RwLock<Option<DateTime<Utc>>>>,
    pub last_replication_keys_success: Arc<AtomicU64>,
    pub last_replication_keys_failed: Arc<AtomicU64>,
}
pub type SharedState = Arc<AppState>;