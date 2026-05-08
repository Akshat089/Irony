use std::sync::Arc;
use reqwest::Client;
use dashmap::DashMap;
use tokio::sync::RwLock;

use common::ring::HashRing;

pub struct AppState {
    pub store: DashMap<String, String>,
    pub ring: Arc<RwLock<HashRing>>,
    pub node_id: String,
    pub node_addr: String,
    pub controller_addr: String,
    pub http_client:reqwest::Client,
    pub ring_version: Arc<RwLock<u64>>,
}

pub type SharedState = Arc<AppState>;