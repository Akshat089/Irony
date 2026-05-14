use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use chrono::{DateTime, Utc};
use common::ring::HashRing;

pub struct AppState {
    pub store: DashMap<String, String>, // In-memory key-value store
    pub ring: Arc<RwLock<HashRing>>, // Consistent hash ring for node management
    pub node_id: String, // Unique identifier for this node
    pub node_addr: String, // Address this node is listening on
    pub controller_addr: String, // Address of the controller for registration and heartbeats
    pub http_client:reqwest::Client, // Shared HTTP client for outgoing requests
    pub ring_version: Arc<RwLock<u64>>, // Version of the ring state to detect changes
    pub total_puts: Arc<AtomicU64>, // Total number of PUT operations handled
    pub total_gets: Arc<AtomicU64>,
    pub replication_success: Arc<AtomicU64>,
    pub replication_failures: Arc<AtomicU64>,
    pub started_at: DateTime<Utc>,
}

pub type SharedState = Arc<AppState>;