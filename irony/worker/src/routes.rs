use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use serde_json::json;

use common::models::{
    GetResponse,
    KeyDumpResponse,
    PutRequest,
    PutResponse,
    ReplicateRequest,
};

use crate::state::SharedState;

pub async fn healthcheck(
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {

    Json(json!({
        "status": "ok",
        "node_id": state.node_id
    }))
}

pub async fn put_key(
    Path(key): Path<String>,
    State(state): State<SharedState>,
    Json(payload): Json<PutRequest>,
) -> Json<PutResponse> {

    state
        .store
        .insert(key.clone(), payload.value);

    Json(PutResponse {
        key,
        success: true,
        replicas_confirmed: 1,
    })
}

pub async fn get_key(
    Path(key): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<GetResponse>, (StatusCode, Json<serde_json::Value>)> {

    match state.store.get(&key) {

        Some(value) => {
            Ok(Json(GetResponse {
                key: key.clone(),
                value: Some(value.clone()),
                served_by: state.node_id.clone(),
            }))
        }

        None => {
            Err((
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "key not found"
                })),
            ))
        }
    }
}

pub async fn get_all_keys(
    State(state): State<SharedState>,
) -> Json<KeyDumpResponse> {

    let keys = state
        .store
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    Json(KeyDumpResponse {
        node_id: state.node_id.clone(),
        keys,
    })
}

pub async fn replicate(
    State(state): State<SharedState>,
    Json(payload): Json<ReplicateRequest>,
) -> Json<serde_json::Value> {

    state
        .store
        .insert(payload.key, payload.value);

    Json(json!({
        "success": true
    }))
}