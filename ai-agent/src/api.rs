use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::llm::client_for_role;
use crate::swarm::{SwarmConfig, SwarmOrchestrator};
use crate::tools::ToolRegistry;

#[derive(Clone)]
struct AppState {
    tools: ToolRegistry,
    config: SwarmConfig,
}

#[derive(Debug, Deserialize)]
struct RunSwarmRequest {
    input: String,
}

#[derive(Debug, Serialize)]
struct RunSwarmResponse {
    final_decision: String,
    blackboard_entries: Vec<crate::blackboard::BlackboardEntry>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

/// Starts an HTTP API service for swarm requests.
pub async fn serve(tools: ToolRegistry, config: SwarmConfig) -> Result<()> {
    let state = Arc::new(AppState { tools, config });
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/swarm/run", post(run_swarm))
        .with_state(state);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|raw| raw.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("starting api server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}

async fn run_swarm(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RunSwarmRequest>,
) -> impl IntoResponse {
    if req.input.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "input must not be empty".to_string(),
            }),
        )
            .into_response();
    }

    let swarm = SwarmOrchestrator::new(client_for_role, state.tools.clone(), state.config.clone());
    match swarm.run(&req.input).await {
        Ok(result) => (
            StatusCode::OK,
            Json(RunSwarmResponse {
                final_decision: result.final_decision,
                blackboard_entries: result.blackboard.entries_owned(),
            }),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: err.to_string(),
            }),
        )
            .into_response(),
    }
}
