use crate::cli_tool::run_session;
use crate::cli_tool::{RunArgs, ServeArgs};
use crate::data_handler::get_configuration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use futures_util::{SinkExt, StreamExt};
use log::LevelFilter;
use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use tokio::net::TcpStream;

use tokio::signal::ctrl_c;
use tokio::sync::broadcast;
use uuid::Uuid;

async fn status() -> Result<&'static str, (StatusCode, String)> {
    Ok("Server is up!")
}

async fn fetch_tcp(state: &AppState, command: &str) -> Result<String, String> {
    let addr = {
        let tcp_addr = state.tcp_addr.lock().await;
        tcp_addr.clone()
    };

    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| format!("Failed to connect to {addr}: {e}"))?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut response = String::new();
    let command = command.as_bytes();
    writer
        .write_all(command)
        .await
        .map_err(|e| format!("Failed to write: {e}"))?;

    reader
        .read_line(&mut response)
        .await
        .map_err(|e| format!("Failed to read: {e}"))?;

    Ok(response.trim().to_string())
}

async fn fetch_and_parse_json(state: &AppState, command: &str) -> impl IntoResponse {
    match fetch_tcp(state, command).await {
        Ok(data) => match serde_json::from_str::<Value>(&data) {
            Ok(json) => Json(json).into_response(),
            Err(_) => (
                StatusCode::BAD_GATEWAY,
                format!("Received malformed JSON: {data}"),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("Error communicating with TCP server: {e}"),
        )
            .into_response(),
    }
}
async fn fetch(state: &AppState, command: &str) -> impl IntoResponse {
    match fetch_tcp(state, command).await {
        Ok(data) => (StatusCode::OK, data),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("Error communicating with TCP server: {e}"),
        ),
    }
}
async fn get_data(State(state): State<AppState>) -> impl IntoResponse {
    fetch_and_parse_json(&state, "GET_DATASTREAM\n").await
}

async fn server_status(State(state): State<AppState>) -> impl IntoResponse {
    fetch_and_parse_json(&state, "STATE\n").await
}

async fn pause(State(state): State<AppState>) -> impl IntoResponse {
    fetch(&state, "PAUSE_STATE\n").await
}

async fn resume(State(state): State<AppState>) -> impl IntoResponse {
    fetch(&state, "RESUME_STATE\n").await
}
async fn kill(State(state): State<AppState>) -> impl IntoResponse {
    fetch(&state, "KILL\n").await
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
    tcp_addr: Arc<tokio::sync::Mutex<String>>,
}
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}
async fn handle_websocket(socket: WebSocket, state: AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    log::info!("WebSocket client connected");

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                log::debug!("Received WebSocket command: {}", text);

                match process_websocket_command(&state, &text).await {
                    Ok(response) => {
                        if let Err(e) = ws_sender.send(Message::Text(response.into())).await {
                            log::error!("Failed to send WebSocket response: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("Command processing error: {}", e);
                        let error_msg = format!("ERROR: {}", e);
                        if let Err(e) = ws_sender.send(Message::Text(error_msg.into())).await {
                            log::error!("Failed to send error: {}", e);
                            break;
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                log::info!("WebSocket client disconnected");
                break;
            }
            Ok(Message::Ping(data)) => {
                if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                    log::error!("Failed to send pong: {}", e);
                    break;
                }
            }
            Err(e) => {
                log::error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    log::info!("WebSocket connection closed");
}

async fn process_websocket_command(state: &AppState, command: &str) -> Result<String, String> {
    let tcp_command = match command.trim() {
        "GET_DATASTREAM" => "GET_DATASTREAM\n",
        "STATE" => "STATE\n",
        "KILL" => "KILL\n",
        "PAUSE_STATE" => "PAUSE_STATE\n",
        "RESUME_STATE" => "RESUME_STATE\n",
        other => return Err(format!("Unknown command: {}", other)),
    };
    let addr = {
        let tcp_addr = state.tcp_addr.lock().await;
        tcp_addr.clone()
    };

    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| format!("Failed to connect to TCP backend at {}: {}", addr, e))?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    writer
        .write_all(tcp_command.as_bytes())
        .await
        .map_err(|e| format!("Failed to write: {}", e))?;

    let mut response = String::new();
    reader
        .read_line(&mut response)
        .await
        .map_err(|e| format!("Failed to read: {}", e))?;

    Ok(response.trim().to_string())
}
async fn run_handler(
    State(state): State<AppState>,

    Json(args): Json<RunArgs>,
) -> Result<Json<RunResponse>, (StatusCode, String)> {
    let shutdown_tx = state.shutdown_tx.clone();
    let config_port = match get_configuration() {
        Ok(conf) => conf.general.port,
        Err(e) => {
            log::error!("Failed to get configuration due to: {e}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read configuration: {e}"),
            ));
        }
    };
    let addr = format!("0.0.0.0:{}", args.port.clone().unwrap_or(config_port));
    {
        let mut tcp_addr = state.tcp_addr.lock().await;
        *tcp_addr = addr;
    }
    let log_level = state.log_level;
    let uuid = Uuid::new_v4();
    let was_running = state.running.swap(true, Ordering::SeqCst);
    match was_running {
        false => {
            let running_clone = state.running.clone();

            tokio::task::spawn(async move {
                tokio::task::spawn_blocking(move || {
                    run_session(args, shutdown_tx, log_level, uuid);
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
        StatusCode::NO_CONTENT // 204: Server is up, but no session active
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
        tcp_addr: Arc::new(tokio::sync::Mutex::new("0.0.0.0:7676".to_string())),
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
        .route("/ws", get(websocket_handler))
        .with_state(state);

    log::info!("Rex Server listening on http://0.0.0.0:{}", args.address);
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
