use crate::cli_tool::run_experiment;
use crate::cli_tool::{RunArgs, ServeArgs};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use log::LevelFilter;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal::ctrl_c;
use tokio::sync::broadcast;
use uuid::Uuid;
async fn server_status() -> Result<&'static str, (StatusCode, String)> {
    Ok("Server is up!")
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
}

async fn run_handler(
    State(state): State<AppState>,
    // to be changed to defaults that can be overridden via serverargs
    Json(args): Json<RunArgs>,
) -> Result<Json<RunResponse>, (StatusCode, String)> {
    let shutdown_tx = state.shutdown_tx.clone();
    let log_level = state.log_level;
    let uuid = Uuid::new_v4();
    let was_running = state.running.swap(true, Ordering::SeqCst);
    match was_running {
        false => {
            let running_clone = state.running.clone();

            tokio::task::spawn(async move {
                tokio::task::spawn_blocking(move || {
                    run_experiment(args, shutdown_tx, log_level, uuid);
                })
                .await
                .unwrap_or_else(|e| {
                    log::error!("Task panicked: {:?}", e);
                });
                log::info!("server is back to listening for its next task...");

                running_clone.store(false, Ordering::SeqCst);
            });
            Ok(Json(RunResponse {
                id: uuid.to_string(),
                message: "Experiment started".to_string(),
            }))
        }
        true => Ok(Json(RunResponse {
            id: "None".to_string(),
            message: "Experiment is already running, ignoring request".to_string(),
        })),
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
    };

    let app = Router::new()
        .route("/", get(server_status))
        .route("/run", post(run_handler))
        .with_state(state);

    log::info!("Rex Server listening on http://127.0.0.1:{}", args.address);
    let address = format!("0.0.0.0:{}", args.address);
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
                if let Err(_) = shutdown_server_tx.send(()) {}
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
