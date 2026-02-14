//! HTTP endpoints for the RVF server using axum.
//!
//! Endpoints:
//! - POST /v1/ingest  - batch vector ingest
//! - POST /v1/query   - k-NN query
//! - POST /v1/delete  - delete by IDs
//! - GET  /v1/status  - store status
//! - GET  /v1/health  - health check

use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use rvf_runtime::{QueryOptions, RvfStore};

use crate::error::ServerError;

/// Shared server state: the store behind a mutex.
pub type SharedStore = Arc<Mutex<RvfStore>>;

/// Build the axum router with all endpoints.
pub fn router(store: SharedStore) -> Router {
    Router::new()
        .route("/v1/ingest", post(ingest))
        .route("/v1/query", post(query))
        .route("/v1/delete", post(delete))
        .route("/v1/status", get(status))
        .route("/v1/health", get(health))
        .with_state(store)
}

// ── Request / Response types ────────────────────────────────────────

#[derive(Deserialize)]
pub struct IngestRequest {
    /// 2-D array of vectors: each inner array is one vector's f32 components.
    pub vectors: Vec<Vec<f32>>,
    /// Corresponding vector IDs (must have same length as `vectors`).
    pub ids: Vec<u64>,
    /// Optional metadata entries (one per vector, flattened).
    pub metadata: Option<Vec<MetadataEntryJson>>,
}

#[derive(Deserialize)]
pub struct MetadataEntryJson {
    pub field_id: u16,
    pub value: MetadataValueJson,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum MetadataValueJson {
    U64(u64),
    F64(f64),
    String(String),
}

#[derive(Serialize, Deserialize)]
pub struct IngestResponse {
    pub accepted: u64,
    pub rejected: u64,
    pub epoch: u32,
}

#[derive(Deserialize)]
pub struct QueryRequest {
    /// The query vector.
    pub vector: Vec<f32>,
    /// Number of nearest neighbors to return.
    pub k: usize,
    /// Optional ef_search override.
    pub ef_search: Option<u16>,
}

#[derive(Serialize, Deserialize)]
pub struct QueryResponse {
    pub results: Vec<QueryResultEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct QueryResultEntry {
    pub id: u64,
    pub distance: f32,
}

#[derive(Deserialize)]
pub struct DeleteRequest {
    /// Vector IDs to delete.
    pub ids: Vec<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteResponse {
    pub deleted: u64,
    pub epoch: u32,
}

#[derive(Serialize, Deserialize)]
pub struct StatusResponse {
    pub total_vectors: u64,
    pub total_segments: u32,
    pub file_size: u64,
    pub current_epoch: u32,
    pub profile_id: u8,
    pub dead_space_ratio: f64,
    pub read_only: bool,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn ingest(
    State(store): State<SharedStore>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, ServerError> {
    if req.vectors.len() != req.ids.len() {
        return Err(ServerError::BadRequest(
            "vectors and ids must have the same length".into(),
        ));
    }

    let vec_refs: Vec<&[f32]> = req.vectors.iter().map(|v| v.as_slice()).collect();

    let metadata: Option<Vec<rvf_runtime::MetadataEntry>> = req.metadata.map(|entries| {
        entries
            .into_iter()
            .map(|e| rvf_runtime::MetadataEntry {
                field_id: e.field_id,
                value: match e.value {
                    MetadataValueJson::U64(v) => rvf_runtime::MetadataValue::U64(v),
                    MetadataValueJson::F64(v) => rvf_runtime::MetadataValue::F64(v),
                    MetadataValueJson::String(v) => rvf_runtime::MetadataValue::String(v),
                },
            })
            .collect()
    });

    let result = {
        let mut s = store.lock().await;
        s.ingest_batch(
            &vec_refs,
            &req.ids,
            metadata.as_deref(),
        )?
    };

    Ok(Json(IngestResponse {
        accepted: result.accepted,
        rejected: result.rejected,
        epoch: result.epoch,
    }))
}

async fn query(
    State(store): State<SharedStore>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ServerError> {
    if req.k == 0 {
        return Err(ServerError::BadRequest("k must be > 0".into()));
    }

    let opts = QueryOptions {
        ef_search: req.ef_search.unwrap_or(100),
        ..Default::default()
    };

    let results = {
        let s = store.lock().await;
        s.query(&req.vector, req.k, &opts)?
    };

    Ok(Json(QueryResponse {
        results: results
            .into_iter()
            .map(|r| QueryResultEntry {
                id: r.id,
                distance: r.distance,
            })
            .collect(),
    }))
}

async fn delete(
    State(store): State<SharedStore>,
    Json(req): Json<DeleteRequest>,
) -> Result<Json<DeleteResponse>, ServerError> {
    if req.ids.is_empty() {
        return Err(ServerError::BadRequest("ids must not be empty".into()));
    }

    let result = {
        let mut s = store.lock().await;
        s.delete(&req.ids)?
    };

    Ok(Json(DeleteResponse {
        deleted: result.deleted,
        epoch: result.epoch,
    }))
}

async fn status(
    State(store): State<SharedStore>,
) -> Result<Json<StatusResponse>, ServerError> {
    let s = store.lock().await;
    let st = s.status();

    Ok(Json(StatusResponse {
        total_vectors: st.total_vectors,
        total_segments: st.total_segments,
        file_size: st.file_size,
        current_epoch: st.current_epoch,
        profile_id: st.profile_id,
        dead_space_ratio: st.dead_space_ratio,
        read_only: st.read_only,
    }))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use rvf_runtime::RvfOptions;
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn create_test_store() -> (TempDir, SharedStore) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rvf");
        let options = RvfOptions {
            dimension: 4,
            ..Default::default()
        };
        let store = RvfStore::create(&path, options).unwrap();
        (dir, Arc::new(Mutex::new(store)))
    }

    #[tokio::test]
    async fn test_health() {
        let (_dir, store) = create_test_store();
        let app = router(store);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_status_empty_store() {
        let (_dir, store) = create_test_store();
        let app = router(store);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v1/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: StatusResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(status.total_vectors, 0);
        assert!(!status.read_only);
    }

    #[tokio::test]
    async fn test_ingest_and_query() {
        let (_dir, store) = create_test_store();
        let app = router(store.clone());

        // Ingest
        let ingest_body = serde_json::json!({
            "vectors": [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]],
            "ids": [1, 2]
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&ingest_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let ingest_resp: IngestResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(ingest_resp.accepted, 2);
        assert_eq!(ingest_resp.rejected, 0);

        // Query
        let app2 = router(store);
        let query_body = serde_json::json!({
            "vector": [1.0, 0.0, 0.0, 0.0],
            "k": 2
        });

        let resp = app2
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/query")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&query_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let query_resp: QueryResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(query_resp.results.len(), 2);
        assert_eq!(query_resp.results[0].id, 1);
        assert!(query_resp.results[0].distance < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_ingest_and_delete() {
        let (_dir, store) = create_test_store();
        let app = router(store.clone());

        // Ingest 3 vectors
        let ingest_body = serde_json::json!({
            "vectors": [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0]
            ],
            "ids": [10, 20, 30]
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&ingest_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Delete one
        let app2 = router(store.clone());
        let delete_body = serde_json::json!({ "ids": [20] });

        let resp = app2
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/delete")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&delete_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let del_resp: DeleteResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(del_resp.deleted, 1);

        // Verify status shows 2 vectors
        let app3 = router(store);
        let resp = app3
            .oneshot(
                Request::builder()
                    .uri("/v1/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: StatusResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(status.total_vectors, 2);
    }

    #[tokio::test]
    async fn test_ingest_bad_request() {
        let (_dir, store) = create_test_store();
        let app = router(store);

        // Mismatched lengths
        let body = serde_json::json!({
            "vectors": [[1.0, 0.0, 0.0, 0.0]],
            "ids": [1, 2]
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_query_bad_k() {
        let (_dir, store) = create_test_store();
        let app = router(store);

        let body = serde_json::json!({
            "vector": [1.0, 0.0, 0.0, 0.0],
            "k": 0
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/query")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_delete_empty_ids() {
        let (_dir, store) = create_test_store();
        let app = router(store);

        let body = serde_json::json!({ "ids": [] });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/delete")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
