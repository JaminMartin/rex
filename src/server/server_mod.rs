use crate::cli_tool::run_session;
use crate::cli_tool::{RunArgs, ServeArgs};
use crate::data_handler::{
    get_configuration, ClientMessage, ServerCommand, ServerEvent, SharedSessionController,
    LIVE_PROTOCOL_VERSION,
};
use axum::{
    extract::{ConnectInfo, ws::{Message, WebSocket, WebSocketUpgrade}, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use log::LevelFilter;
use serde::Deserialize;
use serde::Serialize;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use walkdir::WalkDir;

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
async fn get_allowed_scripts_list() -> impl IntoResponse {
    match get_configuration() {
        Ok(config) => {
            let base_dirs = config.get_allowed_script_dirs();
            let mut files = Vec::new();

            for base_dir in &base_dirs {
                for entry in WalkDir::new(base_dir)
                    .max_depth(4)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if matches!(ext.to_str(), Some("py" | "rs" | "m")) {
                                files.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }

            Json(serde_json::json!({
                // Keep the legacy field while clients move to `base_dirs`.
                "base_dir": base_dirs.first().map(|path| path.to_string_lossy().to_string()),
                "base_dirs": base_dirs.iter().map(|path| path.to_string_lossy().to_string()).collect::<Vec<_>>(),
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

fn is_allowed_script_extension(path: &Path) -> bool {
    matches!(path.extension().and_then(|ext| ext.to_str()), Some("py" | "rs" | "m"))
}

/// Validate the path independently of the frontend.  Browser clients can
/// forge a request, so the control plane is the authority for remote access.
fn validate_remote_script_path(path: &Path) -> Result<(), String> {
    if !is_allowed_script_extension(path) {
        return Err("Script must have a .py, .rs, or .m extension".to_string());
    }
    let script = path
        .canonicalize()
        .map_err(|error| format!("Cannot resolve script path: {error}"))?;
    if !script.is_file() {
        return Err("Script path is not a regular file".to_string());
    }
    let config = get_configuration().map_err(|error| format!("Failed to get configuration: {error}"))?;
    let roots = config.get_allowed_script_dirs();
    if roots.is_empty() {
        return Err("No remote script roots are configured".to_string());
    }
    let allowed = roots.into_iter().filter_map(|root| root.canonicalize().ok()).any(|root| script.starts_with(root));
    allowed.then_some(()).ok_or_else(|| "Script must be inside an allowed script directory for remote clients".to_string())
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

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn send_snapshot(
    socket: &mut WebSocket,
    controller: Option<crate::data_handler::SessionController>,
    projection: Option<crate::data_handler::StreamProjectionConfig>,
) -> Result<(), ()> {
    let (revision, session, stream) = if let Some(controller) = controller {
        let revision = controller.revision();
        let session = controller.session_snapshot().await.ok();
        let stream = Some(controller.stream_snapshot_with_projection(projection).await);
        (revision, session, stream)
    } else {
        (0, None, None)
    };
    let payload = serde_json::to_string(&ServerEvent::Snapshot { revision, session, stream })
        .map_err(|_| ())?;
    socket.send(Message::Text(payload.into())).await.map_err(|_| ())
}

async fn handle_websocket(mut socket: WebSocket, state: AppState) {
    let mut projection = None;
    let mut events = state.session_controller.subscribe();
    let controller = state.session_controller.get();
    let session_id = if let Some(controller) = controller.as_ref() {
        Some(controller.session_id().await)
    } else {
        None
    };
    let hello = ServerEvent::Hello {
        protocol_version: LIVE_PROTOCOL_VERSION,
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        session_id,
    };
    let Ok(hello) = serde_json::to_string(&hello) else { return };
    if socket.send(Message::Text(hello.into())).await.is_err()
        || send_snapshot(&mut socket, controller, projection.clone()).await.is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            incoming = socket.next() => match incoming {
                Some(Ok(Message::Text(text))) => match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Subscribe { projection: requested }) => {
                        projection = requested;
                        if send_snapshot(&mut socket, state.session_controller.get(), projection.clone()).await.is_err() {
                            return;
                        }
                    }
                    Ok(ClientMessage::Ping) => {}
                    Err(error) => {
                        let error = ServerEvent::Error { code: "invalid_client_message".to_string(), message: error.to_string() };
                        if let Ok(payload) = serde_json::to_string(&error) {
                            if socket.send(Message::Text(payload.into())).await.is_err() { return; }
                        }
                    }
                },
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
                _ => {}
            },
            event = events.recv() => match event {
                Ok(ServerEvent::StreamUpdated { revision, .. }) => {
                    let Some(controller) = state.session_controller.get() else { continue; };
                    let stream = controller.stream_snapshot_with_projection(projection.clone()).await;
                    let session = controller.session_snapshot().await.ok();
                    let event = ServerEvent::StreamUpdated { revision, stream, session };
                    let Ok(payload) = serde_json::to_string(&event) else { continue; };
                    if socket.send(Message::Text(payload.into())).await.is_err() { return; }
                }
                Ok(event) => {
                    let Ok(payload) = serde_json::to_string(&event) else { continue; };
                    if socket.send(Message::Text(payload.into())).await.is_err() { return; }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    if send_snapshot(&mut socket, state.session_controller.get(), projection.clone()).await.is_err() { return; }
                }
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    }
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
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(args): Json<RunArgs>,
) -> Result<Json<RunResponse>, (StatusCode, String)> {
    if !peer.ip().is_loopback() {
        validate_remote_script_path(&args.path)
            .map_err(|error| (StatusCode::FORBIDDEN, error))?;
    }
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
pub async fn run_control_plane(
    host: String,
    port: u32,
    shutdown_tx: broadcast::Sender<()>,
    mut control_shutdown_rx: broadcast::Receiver<()>,
    log_level: LevelFilter,
    session_controller: SharedSessionController,
    running: Arc<AtomicBool>,
) {
    let state = AppState {
        shutdown_tx: shutdown_tx.clone(),
        log_level,
        running,
        session_controller,
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
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let address = format!("{host}:{port}");
    log::info!("Rex Server listening on http://{address}");
    let listener = match tokio::net::TcpListener::bind(&address).await {
        Ok(listener) => listener,
        Err(error) => {
            log::error!("Could not bind Rex control plane at http://{address}: {error}");
            return;
        }
    };

    if let Err(error) = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async move {
            control_shutdown_rx.recv().await.ok();
        })
        .await
    {
        log::error!("Rex control plane stopped unexpectedly: {error}");
    }
}

pub async fn run_server(
    args: ServeArgs,
    _shutting_down: Arc<AtomicBool>,
    shutdown_tx: broadcast::Sender<()>,
    log_level: LevelFilter,
) {
    let (session_shutdown_tx, _) = broadcast::channel(1);
    run_control_plane(
        args.host,
        args.port,
        session_shutdown_tx,
        shutdown_tx.subscribe(),
        log_level,
        SharedSessionController::default(),
        Arc::new(AtomicBool::new(false)),
    )
    .await;
}
