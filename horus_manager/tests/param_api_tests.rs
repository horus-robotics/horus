// Integration tests for parameter API handlers
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use horus_core::params::{ParamMetadata, RuntimeParams, ValidationRule};
use horus_manager::security::auth::AuthService;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

// Helper to create test app state
fn create_test_state() -> Arc<horus_manager::dashboard::AppState> {
    let params = Arc::new(RuntimeParams::default());
    // Use a dummy password hash for testing (hash of "test")
    let auth_service = Arc::new(
        AuthService::new("$argon2id$v=19$m=19456,t=2,p=1$test$test".to_string())
            .unwrap_or_else(|_| panic!("Failed to create auth service")),
    );

    Arc::new(horus_manager::dashboard::AppState {
        port: 0,
        params,
        auth_service,
        current_workspace: None,
        auth_disabled: true,
    })
}

// Helper to create test router with parameter routes
fn create_test_router(state: Arc<horus_manager::dashboard::AppState>) -> Router {
    use axum::routing::{delete, get, post};

    Router::new()
        .route(
            "/api/params",
            get(horus_manager::dashboard::params_list_handler),
        )
        .route(
            "/api/params/:key",
            get(horus_manager::dashboard::params_get_handler),
        )
        .route(
            "/api/params/:key",
            post(horus_manager::dashboard::params_set_handler),
        )
        .route(
            "/api/params/:key",
            delete(horus_manager::dashboard::params_delete_handler),
        )
        .route(
            "/api/params/export",
            post(horus_manager::dashboard::params_export_handler),
        )
        .route(
            "/api/params/import",
            post(horus_manager::dashboard::params_import_handler),
        )
        .with_state(state)
}

#[tokio::test]
async fn test_params_list_empty() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    // RuntimeParams::default() may create some initial parameters
    assert!(json["count"].is_number());
    assert!(json["params"].is_array());
}

#[tokio::test]
async fn test_params_list_with_data() {
    let state = create_test_state();

    // Get initial count
    let initial_count = state.params.get_all().len();

    // Add some test parameters
    state.params.set("test_string", "hello").unwrap();
    state.params.set("test_number", 42).unwrap();
    state.params.set("test_bool", true).unwrap();

    let app = create_test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    // Should have initial params + our 3 new ones
    assert_eq!(json["count"], initial_count + 3);
    assert!(json["params"].is_array());

    // Verify our params are in the list
    let params_array = json["params"].as_array().unwrap();
    let param_keys: Vec<String> = params_array
        .iter()
        .map(|p| p["key"].as_str().unwrap().to_string())
        .collect();
    assert!(param_keys.contains(&"test_string".to_string()));
    assert!(param_keys.contains(&"test_number".to_string()));
    assert!(param_keys.contains(&"test_bool".to_string()));
}

#[tokio::test]
async fn test_params_get_existing() {
    let state = create_test_state();
    state.params.set("test_key", "test_value").unwrap();

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["key"], "test_key");
    assert_eq!(json["value"], "test_value");
}

#[tokio::test]
async fn test_params_get_nonexistent() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/nonexistent_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_params_set_string() {
    let state = create_test_state();
    let app = create_test_router(state.clone());

    let payload = json!({
        "value": "new_value"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/test_key")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["key"], "test_key");
    assert_eq!(json["value"], "new_value");

    // Verify it was actually set
    let stored_value: String = state.params.get("test_key").unwrap();
    assert_eq!(stored_value, "new_value");
}

#[tokio::test]
async fn test_params_set_number() {
    let state = create_test_state();
    let app = create_test_router(state.clone());

    let payload = json!({
        "value": 123
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/num_key")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify it was actually set
    let stored_value: i32 = state.params.get("num_key").unwrap();
    assert_eq!(stored_value, 123);
}

#[tokio::test]
async fn test_params_set_array() {
    let state = create_test_state();
    let app = create_test_router(state.clone());

    let payload = json!({
        "value": [1, 2, 3, 4, 5]
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/array_key")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify it was actually set
    let stored_value: Vec<i32> = state.params.get("array_key").unwrap();
    assert_eq!(stored_value, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_params_set_object() {
    let state = create_test_state();
    let app = create_test_router(state.clone());

    let payload = json!({
        "value": {
            "name": "test",
            "count": 42
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/obj_key")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify it was actually set
    let stored_value: Value = state.params.get("obj_key").unwrap();
    assert_eq!(stored_value["name"], "test");
    assert_eq!(stored_value["count"], 42);
}

#[tokio::test]
async fn test_params_set_with_validation_fail() {
    let state = create_test_state();

    // Set metadata with validation rules
    let metadata = ParamMetadata {
        description: Some("Test parameter".to_string()),
        unit: None,
        validation: vec![ValidationRule::Range(0.0, 100.0)],
        read_only: false,
    };
    state
        .params
        .set_metadata("validated_key", metadata)
        .unwrap();

    let app = create_test_router(state.clone());

    // Try to set value outside range
    let payload = json!({
        "value": 150
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/validated_key")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("range"));
}

#[tokio::test]
async fn test_params_set_readonly_fail() {
    let state = create_test_state();

    // Set initial value
    state.params.set("readonly_key", "initial").unwrap();

    // Make it read-only
    let metadata = ParamMetadata {
        description: Some("Read-only parameter".to_string()),
        unit: None,
        validation: vec![],
        read_only: true,
    };
    state.params.set_metadata("readonly_key", metadata).unwrap();

    let app = create_test_router(state.clone());

    // Try to modify read-only parameter
    let payload = json!({
        "value": "modified"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/readonly_key")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("read-only"));
}

#[tokio::test]
async fn test_params_delete_existing() {
    let state = create_test_state();
    state.params.set("to_delete", "value").unwrap();

    let app = create_test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/to_delete")
                .method("DELETE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["key"], "to_delete");
    assert_eq!(json["old_value"], "value");

    // Verify it was actually deleted
    assert!(state.params.get::<String>("to_delete").is_none());
}

#[tokio::test]
async fn test_params_delete_nonexistent() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/nonexistent")
                .method("DELETE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
}

#[tokio::test]
async fn test_params_export() {
    let state = create_test_state();

    // Add some test data
    state.params.set("key1", "value1").unwrap();
    state.params.set("key2", 42).unwrap();
    state.params.set("key3", true).unwrap();

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/export")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["format"], "yaml");
    assert!(json["data"].is_string());

    // Verify YAML contains our parameters
    let yaml_str = json["data"].as_str().unwrap();
    assert!(yaml_str.contains("key1"));
    assert!(yaml_str.contains("value1"));
}

#[tokio::test]
async fn test_params_import_yaml() {
    let state = create_test_state();
    let app = create_test_router(state.clone());

    let yaml_data = r#"
test_key: test_value
num_key: 123
bool_key: true
"#;

    let payload = json!({
        "data": yaml_data,
        "format": "yaml"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/import")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["count"], 3);

    // Verify parameters were imported
    assert_eq!(
        state.params.get::<String>("test_key").unwrap(),
        "test_value"
    );
    assert_eq!(state.params.get::<i32>("num_key").unwrap(), 123);
    assert_eq!(state.params.get::<bool>("bool_key").unwrap(), true);
}

#[tokio::test]
async fn test_params_import_json() {
    let state = create_test_state();
    let app = create_test_router(state.clone());

    let json_data = json!({
        "json_key": "json_value",
        "json_num": 456
    })
    .to_string();

    let payload = json!({
        "data": json_data,
        "format": "json"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/import")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["count"], 2);

    // Verify parameters were imported
    assert_eq!(
        state.params.get::<String>("json_key").unwrap(),
        "json_value"
    );
    assert_eq!(state.params.get::<i32>("json_num").unwrap(), 456);
}

#[tokio::test]
async fn test_params_import_invalid_format() {
    let state = create_test_state();
    let app = create_test_router(state);

    let payload = json!({
        "data": "some data",
        "format": "xml"  // Invalid format
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/import")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("Invalid format"));
}

#[tokio::test]
async fn test_params_import_malformed_yaml() {
    let state = create_test_state();
    let app = create_test_router(state);

    let bad_yaml = "invalid: yaml: data: [[[";

    let payload = json!({
        "data": bad_yaml,
        "format": "yaml"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/import")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("Failed to parse"));
}

#[tokio::test]
async fn test_concurrent_param_updates() {
    let state = create_test_state();

    // Spawn multiple tasks updating the same parameter
    let mut handles = vec![];

    for i in 0..10 {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            state_clone.params.set("concurrent_key", i).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all updates to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify the final value is one of the expected values (0-9)
    let final_value: i32 = state.params.get("concurrent_key").unwrap();
    assert!(final_value >= 0 && final_value < 10);
}

#[tokio::test]
async fn test_version_tracking() {
    let state = create_test_state();
    let app = create_test_router(state.clone());

    // Set initial value
    let payload = json!({
        "value": "initial"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/versioned_key")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let version1 = json["version"].as_u64().unwrap();

    // Verify version is returned
    assert_eq!(version1, 1);

    // Get the parameter to see version
    let app2 = create_test_router(state.clone());
    let response2 = app2
        .oneshot(
            Request::builder()
                .uri("/api/params/versioned_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body2 = to_bytes(response2.into_body(), usize::MAX).await.unwrap();
    let json2: Value = serde_json::from_slice(&body2).unwrap();

    assert_eq!(json2["version"], 1);
    assert_eq!(json2["value"], "initial");
}

#[tokio::test]
async fn test_version_mismatch_protection() {
    let state = create_test_state();

    // Set initial value
    state.params.set("protected_key", "v1").unwrap();
    let v1 = state.params.get_version("protected_key");

    // Update the value (increments version)
    state.params.set("protected_key", "v2").unwrap();
    let v2 = state.params.get_version("protected_key");

    assert_eq!(v2, v1 + 1);

    // Try to update with old version - should fail
    let app = create_test_router(state.clone());

    let payload = json!({
        "value": "v3",
        "version": v1  // Using old version
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/protected_key")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should get 409 Conflict
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("Version mismatch"));

    // Verify value wasn't changed
    assert_eq!(state.params.get::<String>("protected_key").unwrap(), "v2");
}

#[tokio::test]
async fn test_version_update_with_correct_version() {
    let state = create_test_state();

    // Set initial value
    state.params.set("versioned_param", "value1").unwrap();
    let version = state.params.get_version("versioned_param");

    let app = create_test_router(state.clone());

    let payload = json!({
        "value": "value2",
        "version": version  // Using correct version
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/params/versioned_param")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should succeed
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["version"], version + 1); // Version should be incremented

    // Verify value was changed
    assert_eq!(
        state.params.get::<String>("versioned_param").unwrap(),
        "value2"
    );
}
