//! HTTP route handlers.
//!
//! Keep route modules small and free of wallet secrets. Chain and payment
//! handlers will be added as separate modules in later milestones.

use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use bitsabi_shared::HealthResponse;
use serde::Serialize;

/// Build the API router.
pub fn router() -> Router {
    Router::new()
        .route("/api/health", get(health))
        .fallback(not_found)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::healthy())
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody {
            error: "Route not found",
        }),
    )
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use bitsabi_shared::HealthResponse;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let config = crate::config::Config::for_test();
        let response = crate::app(&config)
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(health.status, "ok");
        assert_eq!(health.service, "bitsabi-api");
    }

    #[tokio::test]
    async fn unknown_route_returns_json_404() {
        let config = crate::config::Config::for_test();
        let response = crate::app(&config)
            .oneshot(
                Request::builder()
                    .uri("/api/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
