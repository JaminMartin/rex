use crate::cli_tool::run_session;
use crate::cli_tool::{RunArgs, ServeArgs};
use crate::data_handler::{get_configuration, ServerCommand, SharedSessionController};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use log::LevelFilter;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

use crate::data_handler::configurable_dir_path;
use tokio::signal::ctrl_c;
use tokio::sync::broadcast;
use uuid::Uuid;
async fn get_allowed_output_dirs() -> impl IntoResponse {
    match get_configuration() {
        Ok(config) => {
            let dirs = config.get_allowed_output_dirs();
            let dir_strings: Vec<String> = dirs
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            Json(serde_json::json!({
                "dirs": dir_strings
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get configuration: {}", e),
        )
            .into_response(),
    }
}
fn get_allowed_scripts_dir() -> Result<PathBuf, String> {
    configurable_dir_path("XDG_CONFIG_HOME", dirs::config_dir)
        .map(|mut path| {
            path.push("rex");
            path.push("scripts");
            path
        })
        .ok_or("Failed to get config directory".to_string())
}
async fn get_allowed_scripts_list() -> impl IntoResponse {
    match get_allowed_scripts_dir() {
        Ok(base_dir) => {
            let mut files = Vec::new();

            for entry in WalkDir::new(&base_dir)
                .max_depth(4)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "py" || ext == "rs" || ext == "m" {
                            files.push(path.to_string_lossy().to_string());
                        }
                    }
                }
            }

            Json(serde_json::json!({
                "base_dir": base_dir.to_string_lossy(),
                "files": files
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get scripts directory: {}", e),
        )
            .into_response(),
    }
}
async fn status() -> Result<&'static str, (StatusCode, String)> {
    Ok("Server is up!")
}

#[derive(Debug, Deserialize)]
struct StreamQuery {
    max_data_points: Option<usize>,
}

async fn fetch_and_parse_json(
    state: &AppState,
    command: ServerCommand,
    max_data_points_override: Option<usize>,
) -> impl IntoResponse {
    let json = match command {
        ServerCommand::GetDatastream => state
            .session_controller
            .stream_snapshot(max_data_points_override)
            .await
            .and_then(|snapshot| serde_json::to_value(snapshot).map_err(|e| e.to_string())),
        ServerCommand::State => state
            .session_controller
            .session_snapshot()
            .await
            .and_then(|snapshot| serde_json::to_value(snapshot).map_err(|e| e.to_string())),
        _ => Err("Command does not return JSON data".to_string()),
    };

    match json {
        Ok(json) => Json(json).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("Error communicating with active session: {e}"),
        )
            .into_response(),
    }
}
async fn fetch(state: &AppState, command: ServerCommand) -> impl IntoResponse {
    let result = match command {
        ServerCommand::Pause => state.session_controller.pause().await,
        ServerCommand::Resume => state.session_controller.resume().await,
        ServerCommand::Kill => state.session_controller.kill().await,
        _ => Err("Command does not return plain text data".to_string()),
    };

    match result {
        Ok(data) => (StatusCode::OK, data),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("Error communicating with active session: {e}"),
        ),
    }
}
async fn get_data(
    State(state): State<AppState>,
    Query(query): Query<StreamQuery>,
) -> impl IntoResponse {
    fetch_and_parse_json(&state, ServerCommand::GetDatastream, query.max_data_points).await
}

async fn server_status(State(state): State<AppState>) -> impl IntoResponse {
    fetch_and_parse_json(&state, ServerCommand::State, None).await
}

async fn pause(State(state): State<AppState>) -> impl IntoResponse {
    fetch(&state, ServerCommand::Pause).await
}

async fn resume(State(state): State<AppState>) -> impl IntoResponse {
    fetch(&state, ServerCommand::Resume).await
}
async fn kill(State(state): State<AppState>) -> impl IntoResponse {
    fetch(&state, ServerCommand::Kill).await
}
#[derive(Serialize)]
struct RunResponse {
    id: String,
    message: String,
}

#[derive(Clone)]
struct AppState {
    shutdown_tx: broadcast::Sender<()>,
    log_level: LevelFilter,
    running: Arc<AtomicBool>,
    session_controller: SharedSessionController,
}
async fn run_handler(
    State(state): State<AppState>,

    Json(args): Json<RunArgs>,
) -> Result<Json<RunResponse>, (StatusCode, String)> {
    let shutdown_tx = state.shutdown_tx.clone();
    let log_level = state.log_level;
    let uuid = Uuid::new_v4();
    let was_running = state.running.swap(true, Ordering::SeqCst);
    match was_running {
        false => {
            let running_clone = state.running.clone();
            let session_controller = state.session_controller.clone();

            tokio::task::spawn(async move {
                tokio::task::spawn_blocking(move || {
                    run_session(args, shutdown_tx, log_level, uuid, Some(session_controller));
                })
                .await
                .unwrap_or_else(|e| {
                    log::error!("Task panicked: {e:?}");
                });
                log::info!("server is back to listening for its next task...");

                running_clone.store(false, Ordering::SeqCst);
            });
            Ok(Json(RunResponse {
                id: uuid.to_string(),
                message: "session started".to_string(),
            }))
        }
        true => Ok(Json(RunResponse {
            id: "None".to_string(),
            message: "Session is already running, ignoring request".to_string(),
        })),
    }
}
async fn check_session(State(state): State<AppState>) -> impl IntoResponse {
    let running = state.running.load(Ordering::SeqCst);
    if running {
        StatusCode::OK
    } else {
        StatusCode::NO_CONTENT
    }
}
pub async fn run_server(
    args: ServeArgs,
    shutting_down: Arc<AtomicBool>,
    shutdown_tx: broadcast::Sender<()>,
    log_level: LevelFilter,
) {
    let state = AppState {
        shutdown_tx: shutdown_tx.clone(),
        log_level,
        running: Arc::new(AtomicBool::new(false)),
        session_controller: SharedSessionController::default(),
    };

    let app = Router::new()
        .route("/", get(status))
        .route("/run", post(run_handler))
        .route("/datastream", get(get_data))
        .route("/status", get(server_status))
        .route("/kill", post(kill))
        .route("/pause", post(pause))
        .route("/continue", post(resume))
        .route("/status_check", get(check_session))
        .route("/allowed_scripts", get(get_allowed_scripts_list))
        .route("/allowed_output_dirs", get(get_allowed_output_dirs))
        .with_state(state);

    log::info!("Rex Server listening on http://0.0.0.0:{}", args.port);
    let address = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(address.clone())
        .await
        .unwrap();

    let server_shutting_down_clone = shutting_down.clone();
    let (shutdown_server_tx, _) = broadcast::channel(1);
    let mut server_shutdown = shutdown_server_tx.subscribe();
    tokio::spawn(async move {
        if let Ok(()) = ctrl_c().await {
            if !server_shutting_down_clone.load(Ordering::SeqCst) {
                server_shutting_down_clone.store(true, Ordering::SeqCst);
                if shutdown_server_tx.send(()).is_err() {}
            }
        }
    });
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            server_shutdown.recv().await.ok();
            println!("Shutting down server...");
        })
        .await
        .unwrap();
}
