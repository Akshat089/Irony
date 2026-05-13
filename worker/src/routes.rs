use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::replication::replicate_to_node;
use serde_json::json;
use std::collections::HashMap;
use common::models::{
    GetResponse,
    KeyDumpResponse,
    PutRequest,
    PutResponse,
    ReplicateRequest,
    ReplicateToRequest,
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
) -> Result<Json<PutResponse>, (StatusCode, Json<serde_json::Value>)> {

    let ring = state.ring.read().await;

    let primary = match ring.find_primary(&key) {
        Some(node) => node,

        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "cluster not ready"
                })),
            ));
        }
    };

    if primary.node_id != state.node_id {

        let redirect_url = format!(
            "http://{}:{}/v1/keys/{}",
            primary.host,
            primary.node_port,
            key
        );

        return Err((
            StatusCode::TEMPORARY_REDIRECT,
            Json(json!({
                "redirect_to": redirect_url
            })),
        ));
    }
    drop(ring);

    state
        .store
        .insert(key.clone(), payload.value.clone());

    let ring = state.ring.read().await;
    let replicas: Vec<_> = ring.find_replicas(&key).into_iter().cloned().collect();
    println!("Replica count: {}", replicas.len());

    for replica in &replicas {
        println!("Replica node: {}", replica.node_id);
    }
    drop(ring);
    if replicas.is_empty() {

        return Ok(Json(PutResponse {
            key,
            success: true,
            replicas_confirmed: 1,
        }));
    }
    let replica1 = replicas.get(0);

    let replica2 = replicas.get(1);

    let mut quorum_achieved = false;

    let mut successful_replica_addr = None;

    let mut failed_replica_addr = None;

    if let Some(replica) = replica1 {

        let target_addr = format!(
            "http://{}:{}",
            replica.host,
            replica.node_port
        );

        let result = replicate_to_node(
            key.clone(),
            payload.value.clone(),
            target_addr.clone(),
            state.http_client.clone(),
            state.node_id.clone(),
        )
        .await;

        match result {

            Ok(_) => {

                println!(
                    "Successfully replicated key {} to replica 1 {}",
                    key,
                    replica.node_id
                );

                quorum_achieved = true;

                successful_replica_addr = Some(target_addr);
            }

            Err(e) => {

                println!(
                    "Replica 1 failed for key {}: {}",
                    key,
                    e
                );

                failed_replica_addr = Some(target_addr);
            }
        }
    }
    if !quorum_achieved {

        if let Some(replica) = replica2 {

            let target_addr = format!(
                "http://{}:{}",
                replica.host,
                replica.node_port
            );

            let result = replicate_to_node(
                key.clone(),
                payload.value.clone(),
                target_addr.clone(),
                state.http_client.clone(),
                state.node_id.clone(),
            )
            .await;

            match result {

                Ok(_) => {

                    println!(
                        "Successfully replicated key {} to replica 2 {}",
                        key,
                        replica.node_id
                    );

                    quorum_achieved = true;

                    successful_replica_addr = Some(target_addr);
                }

                Err(e) => {

                    println!(
                        "Replica 2 also failed for key {}: {}",
                        key,
                        e
                    );
                }
            }
        }
    }
    if !quorum_achieved {

        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "quorum could not be achieved"
            })),
        ));
    }
    let background_replica = if let Some(replica1_node) = replica1 {
        let addr1 = format!(
            "http://{}:{}",
            replica1_node.host,
            replica1_node.node_port
        );

        if successful_replica_addr.as_ref() == Some(&addr1) {
            replica2
        } else {
            Some(replica1_node)
        }

    } else {
        None
    };

    if let Some(replica) = background_replica {

        let target_addr = format!(
            "http://{}:{}",
            replica.host,
            replica.node_port
        );

        let key_clone = key.clone();

        let value_clone = payload.value.clone();

        let client_clone = state.http_client.clone();

        let origin_node_id = state.node_id.clone();

        tokio::spawn(async move {

            let result = replicate_to_node(
                key_clone,
                value_clone,
                target_addr.clone(),
                client_clone,
                origin_node_id,
            )
            .await;

            match result {

                Ok(_) => {
                    println!(
                        "Background replication succeeded to {}",
                        target_addr
                    );
                }

                Err(e) => {
                    println!(
                        "Background replication failed to {}: {}",
                        target_addr,
                        e
                    );
                }
            }
        });
    }

    Ok(Json(PutResponse {
        key,
        success: true,
        replicas_confirmed: 2,
    }))

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

    let key : HashMap<String, String> = state.store.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect();

    Json(KeyDumpResponse {
        node_id: state.node_id.clone(),
        keys: key,
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

pub async fn replicate_to(
    State(state): State<SharedState>,
    Json(payload): Json<ReplicateToRequest>,
) -> Json<serde_json::Value> {

    match state.store.get(&payload.key) {

        Some(value) => {

            let result = replicate_to_node(
                payload.key.clone(),
                value.clone(),
                payload.target_node_addr.clone(),
                state.http_client.clone(),
                state.node_id.clone(),
            )
            .await;

            match result {

                Ok(_) => {
                    Json(json!({
                        "success": true
                    }))
                }

                Err(e) => {
                    Json(json!({
                        "success": false,
                        "error": e
                    }))
                }
            }
        }

        None => {
            Json(json!({
                "success": false,
                "error": "key not found locally"
            }))
        }
    }
}
