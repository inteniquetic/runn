use axum::{
    extract::{Json, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Router,
};
use core_lib::webhooks::gitlab;
use serde_json::Value;
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, warn};

#[derive(Clone)]
struct AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let state = Arc::new(AppState);
    let app = Router::new()
        .route("/health", get(health))
        .route("/webhooks/gitlab/:pipeline", post(gitlab_webhook))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind 0.0.0.0:8080");
    axum::serve(listener, app)
        .await
        .expect("serve");
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn gitlab_webhook(
    State(_state): State<Arc<AppState>>,
    Path(pipeline): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> StatusCode {
    let request = match gitlab::handle_webhook(&headers, &pipeline, payload) {
        Ok(request) => request,
        Err(err) => {
            warn!("invalid gitlab webhook: {:?}", err);
            return gitlab::status_from_error(&err);
        }
    };

    gitlab::trigger_pipeline(&request);
    StatusCode::ACCEPTED
}
