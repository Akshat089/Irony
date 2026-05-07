use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum IronRingError {
    NodeNotFound(String),
    KeyNotFound(String),
    QuorumFailed, //could not write to 2 replias
    ReplicationFailed(String),
    NotPrimary(String),
    InternalError(String),
}