use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::Result;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::llm::client_for_role;
use crate::swarm::{SwarmConfig, SwarmOrchestrator};
use crate::tools::ToolRegistry;

#[derive(Clone)]
struct AppState {
    tools: ToolRegistry,
    config: SwarmConfig,
    jobs: Arc<RwLock<HashMap<String, JobRecord>>>,
    job_sequence: Arc<AtomicU64>,
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

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Clone)]
struct JobRecord {
    id: String,
    input: String,
    status: JobStatus,
    final_decision: Option<String>,
    blackboard_entries: Option<Vec<crate::blackboard::BlackboardEntry>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateJobRequest {
    input: String,
}

#[derive(Debug, Serialize)]
struct CreateJobResponse {
    job_id: String,
    status: JobStatus,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

/// Starts an HTTP API service for swarm requests.
pub async fn serve(tools: ToolRegistry, config: SwarmConfig) -> Result<()> {
    let state = Arc::new(AppState {
        tools,
        config,
        jobs: Arc::new(RwLock::new(HashMap::new())),
        job_sequence: Arc::new(AtomicU64::new(1)),
    });
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/swarm/run", post(run_swarm))
        .route("/v1/jobs", post(create_job))
        .route("/v1/jobs/:id", get(get_job))
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

async fn create_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateJobRequest>,
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

    let seq = state.job_sequence.fetch_add(1, Ordering::Relaxed);
    let job_id = format!("job-{seq}");
    let initial_record = JobRecord {
        id: job_id.clone(),
        input: req.input.clone(),
        status: JobStatus::Queued,
        final_decision: None,
        blackboard_entries: None,
        error: None,
    };

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), initial_record);
    }

    let state_for_task = state.clone();
    let input_for_task = req.input;
    let job_id_for_task = job_id.clone();

    tokio::spawn(async move {
        {
            let mut jobs = state_for_task.jobs.write().await;
            if let Some(record) = jobs.get_mut(&job_id_for_task) {
                record.status = JobStatus::Running;
            }
        }

        let swarm = SwarmOrchestrator::new(
            client_for_role,
            state_for_task.tools.clone(),
            state_for_task.config.clone(),
        );
        let result = swarm.run(&input_for_task).await;

        let mut jobs = state_for_task.jobs.write().await;
        if let Some(record) = jobs.get_mut(&job_id_for_task) {
            match result {
                Ok(output) => {
                    record.status = JobStatus::Completed;
                    record.final_decision = Some(output.final_decision);
                    record.blackboard_entries = Some(output.blackboard.entries_owned());
                    record.error = None;
                }
                Err(err) => {
                    record.status = JobStatus::Failed;
                    record.error = Some(err.to_string());
                }
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(CreateJobResponse {
            job_id,
            status: JobStatus::Queued,
        }),
    )
        .into_response()
}

async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let jobs = state.jobs.read().await;
    match jobs.get(&id) {
        Some(record) => (StatusCode::OK, Json(record.clone())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("job `{id}` not found"),
            }),
        )
            .into_response(),
    }
}
