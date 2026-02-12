use crate::data_handler::transport::{Transport, TransportType};
use crate::data_handler::{
    create_log_timestamp, sanitize_filename, DataSession, Device, Entity, Listner, ServerState,
};
use crate::db::ClickhouseServer;
use clickhouse::Client;
use std::io;
use std::net::SocketAddr;
use std::path::MAIN_SEPARATOR;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::sync::Mutex;
pub async fn start_tcp_server(
    addr: String,
    state: Arc<Mutex<ServerState>>,
    mut shutdown_rx: broadcast::Receiver<()>,
    shutdown_tx: broadcast::Sender<()>,
) -> tokio::io::Result<()> {
    let listener = TcpListener::bind(addr.clone()).await?;
    log::info!("TCP server listening on {addr}");

    loop {
        tokio::select! {
            Ok((socket, addr)) = listener.accept() => {

                log::debug!("New connection from: {addr}");


                let shutdown_tx = shutdown_tx.clone();
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    handle_connection(socket, addr, state, shutdown_tx).await;
                });
            },
            _ = shutdown_rx.recv() => {
                log::info!("Shutdown signal received for TCP server.");
                tokio::time::sleep(Duration::from_secs(3)).await;
                break;
            }
        }
    }
    Ok(())
}

async fn handle_connection(
    socket: TcpStream,
    addr: SocketAddr,
    state: Arc<Mutex<ServerState>>,
    shutdown_tx: broadcast::Sender<()>,
) {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                log::debug!("Connection closed by {addr}");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                log::debug!("Raw data stream:{trimmed}");

                if handle_command(trimmed, &mut writer, &state, &shutdown_tx).await {
                    if trimmed == "KILL" {
                        break;
                    }
                    continue;
                }

                if handle_entity(trimmed, &mut writer, &state).await {
                    continue;
                }

                log::error!("Unknown message format: {}", trimmed);
                let _ = writer.write_all(b"Invalid format\n").await;
            }
            Err(e) => {
                log::error!("Error reading from {addr}: {e}");
                break;
            }
        }
    }
}

async fn handle_command(
    message: &str,
    writer: &mut OwnedWriteHalf,
    state: &Arc<Mutex<ServerState>>,
    shutdown_tx: &broadcast::Sender<()>,
) -> bool {
    match message {
        "GET_DATASTREAM" => {
            let state = state.lock().await;
            let steam_data = state.send_stream();
            if let Ok(state_json) = serde_json::to_string(&steam_data) {
                let _ = writer.write_all(format!("{state_json}\n").as_bytes()).await;
            }
            true
        }
        "STATE" => {
            let state = state.lock().await;
            if let Some(summary) = state.to_summary() {
                if let Ok(state_json) = serde_json::to_string(&summary) {
                    let _ = writer.write_all(format!("{state_json}\n").as_bytes()).await;
                }
            }
            true
        }
        "PAUSE_STATE" => {
            let _ = writer
                .write_all(b"Setting internal server state to paused...\n")
                .await;
            let mut state = state.lock().await;
            state.internal_state = false;
            log::info!("setting server state to paused....");
            true
        }
        "KILL" => {
            let _ = writer.write_all(b"Shutting down server...\n").await;
            log::info!("Received remote termination command, shutting down server");
            let _ = shutdown_tx.send(());
            true
        }
        "RESUME_STATE" => {
            let _ = writer
                .write_all(b"Setting internal server state to start...\n")
                .await;
            let mut state = state.lock().await;
            state.internal_state = true;
            true
        }
        _ => false, // Not a command
    }
}

async fn handle_entity(
    message: &str,
    writer: &mut OwnedWriteHalf,
    state: &Arc<Mutex<ServerState>>,
) -> bool {
    if let Ok(mut device) = serde_json::from_str::<Device>(message) {
        let timestamp = vec![create_log_timestamp()];
        for measure_type in device.measurements.keys() {
            device
                .timestamps
                .entry(measure_type.clone())
                .or_insert(timestamp.clone());
        }
        let device_name = device.device_name.clone();
        let mut state = state.lock().await;
        state.update_entity(device_name, Entity::Device(device));
        let _ = writer.write_all(b"Device measurements recorded\n").await;
        return true;
    }

    if let Ok(data_session) = serde_json::from_str::<DataSession>(message) {
        log::info!("Session data processed");
        let session_name = data_session.info.name.clone();
        let mut state = state.lock().await;
        state.update_entity(session_name, Entity::Session(data_session));
        let _ = writer.write_all(b"Session configuration processed\n").await;
        return true;
    }

    if let Ok(_) = serde_json::from_str::<Listner>(message) {
        log::debug!("Listener query");
        let state = state.lock().await;
        if state.internal_state {
            let _ = writer.write_all(b"Running\n").await;
        } else {
            let _ = writer.write_all(b"Paused\n").await;
        }
        return true;
    }

    false
}

pub async fn save_state(
    state: Arc<Mutex<ServerState>>,
    mut shutdown_rx: broadcast::Receiver<()>,
    shutdown_tx: broadcast::Sender<()>,
    file_name_suffix: &str,
    output_path: &String,
) -> io::Result<String> {
    let mut interval = tokio::time::interval(Duration::from_secs(3));
    tokio::time::sleep(Duration::from_secs(1)).await;
    let mut output_file_name = String::new();
    let _ = output_file_name;

    let mut validated_once = false;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut retries = 3;
                while retries > 0 {
                    {
                        let state_guard = state.lock().await;
                        if !validated_once {
                            if let Err(err) = state_guard.validate() {
                                log::warn!("Validation failed: {err:?}. Retrying in 5 seconds...");
                                retries -= 1;
                                if retries == 0 {
                                    let _ = shutdown_tx.send(());
                                    return Err(io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        format!("State is invalid after retry: {err:?}"),
                                    ));
                                }
                            } else {
                                validated_once = true;
                            }
                        } else {
                            let file_name = match state_guard.get_session_name() {
                                Some(name) => name,
                                None => break,
                            };
                            let sanitized_file_name = sanitize_filename(file_name);
                            let sanitized_output_path = clean_trailing_slash(output_path);
                            output_file_name = format_file_path(&sanitized_output_path, &sanitized_file_name, file_name_suffix);

                            state_guard.dump_to_toml(&output_file_name)?;
                            break;
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
            _ = shutdown_rx.recv() => {
                tokio::time::sleep(Duration::from_secs(3)).await;
                let mut state = state.lock().await;
                state.finalise_time();
                let file_name = match state.get_session_name() {
                    Some(file_name) => file_name,
                    None => break,
                };
                let sanitized_file_name = sanitize_filename(file_name);
                let sanitized_output_path = clean_trailing_slash(output_path);
                output_file_name = format_file_path(&sanitized_output_path, &sanitized_file_name, file_name_suffix);
                state.dump_to_toml(&output_file_name)?;
                break;
            }
        }
    }
    log::info!("Saved state to: {output_file_name}");
    Ok(output_file_name)
}
pub async fn server_status(
    state: Arc<Mutex<ServerState>>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> tokio::io::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        tokio::select! {
                 _ = interval.tick() => {
                let state = state.lock().await;
                state.print_state();
            },
            _ = shutdown_rx.recv() => {
            tokio::time::sleep(Duration::from_secs(3)).await;
            break;
            }
        }
    }

    Ok(())
}

pub fn clean_trailing_slash(path: &str) -> String {
    path.trim_end_matches(['/', '\\']).to_string()
}

fn format_file_path(output_path: &str, file_name: &str, file_suffix: &str) -> String {
    let sanitized_output_path = clean_trailing_slash(output_path);
    let separator = MAIN_SEPARATOR;
    format!("{sanitized_output_path}{separator}{file_name}_{file_suffix}.toml")
}

pub async fn send_to_clickhouse(
    state: Arc<Mutex<ServerState>>,
    config: ClickhouseServer,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::default()
        .with_url(format!("{}:{}", config.server, config.port))
        .with_database(config.database)
        .with_user(config.username)
        .with_password(config.password)
        .with_option("allow_experimental_json_type", "1")
        .with_option("input_format_binary_read_json_as_string", "1");

    log::info!("Starting clickhouse Logging!");
    {
        let state = state.lock().await;
        let session_data = state
            .session_data_ch(state.uuid)
            .ok_or("No session data found")?;
        let mut insert_session = client.insert(&config.session_meta_table)?;

        insert_session.write(&session_data).await?;

        let mut insert_measure = client.insert(&config.measurement_table)?;
        let device_data = state
            .device_data_ch(state.uuid)
            .ok_or("no device data found")?;
        for chm in device_data {
            for m in &chm.measurements {
                insert_measure.write(m).await?;
            }
        }

        let mut insert_conf = client.insert(&config.device_meta_table)?;
        let device_conf = state
            .device_config_ch(state.uuid)
            .ok_or("no device data found")?;

        for conf in device_conf.devices {
            insert_conf.write(&conf).await?;
        }

        insert_session.end().await?;
        insert_measure.end().await?;
        insert_conf.end().await?;

        log::info!("Completed Clickhouse logging!");
        Ok(())
    }
}

#[derive(Clone)]
pub struct TCPTransport {
    inner: Arc<Mutex<TCPTransportInner>>,
}

struct TCPTransportInner {
    addr: String,
    stream: Option<TcpStream>,
    rerun_handle: Option<JoinHandle<()>>,
    rerun_shutdown_tx: Option<broadcast::Sender<()>>,
    rerun_shutting_down: Option<Arc<AtomicBool>>,
}

impl TCPTransport {
    pub fn new(addr: &str) -> Self {
        TCPTransport {
            inner: Arc::new(Mutex::new(TCPTransportInner {
                addr: addr.to_string(),
                stream: None,
                rerun_handle: None,
                rerun_shutdown_tx: None,
                rerun_shutting_down: None,
            })),
        }
    }

    pub async fn cleanup_rerun(&mut self) {
        let mut inner = self.inner.lock().await;

        if let Some(ref handle) = inner.rerun_handle {
            if !handle.is_finished() {
                log::info!("TUI exiting, shutting down active rerun session gracefully...");

                if let Some(ref shutting_down) = inner.rerun_shutting_down {
                    shutting_down.store(true, Ordering::SeqCst);
                }

                if let Some(ref tx) = inner.rerun_shutdown_tx {
                    let _ = tx.send(());
                }

                if let Some(handle) = inner.rerun_handle.take() {
                    drop(inner);

                    let start = std::time::Instant::now();
                    let timeout = Duration::from_secs(30);

                    while !handle.is_finished() && start.elapsed() < timeout {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }

                    if handle.is_finished() {
                        let _ = handle.join();
                        log::info!("Rerun session shutdown completed successfully");
                    } else {
                        log::warn!(
                            "Rerun session did not complete within timeout, detaching thread"
                        );
                    }

                    let mut inner = self.inner.lock().await;
                    inner.rerun_shutdown_tx = None;
                    inner.rerun_shutting_down = None;
                }
            }
        } else {
            inner.rerun_shutdown_tx = None;
            inner.rerun_shutting_down = None;
        }
    }
}

use async_trait::async_trait;

#[async_trait]
impl Transport for TCPTransport {
    async fn send_command(
        &mut self,
        command: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send>> {
        self.ensure_connection().await?;

        let mut inner = self.inner.lock().await;
        let stream =
            inner
                .stream
                .as_mut()
                .ok_or_else(|| -> Box<dyn std::error::Error + Send> {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotConnected,
                        "No connection",
                    ))
                })?;

        if let Err(e) = stream.write_all(command.as_bytes()).await {
            inner.stream = None;
            return Err(Box::new(e));
        }

        if let Err(e) = stream.flush().await {
            inner.stream = None;
            return Err(Box::new(e));
        }

        // Need to split the stream to read
        let (reader, _writer) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut response = String::new();

        match reader.read_line(&mut response).await {
            Ok(0) => {
                inner.stream = None;
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    "Server closed connection",
                )))
            }
            Ok(_) => Ok(response.trim().to_string()),
            Err(e) => {
                inner.stream = None;
                Err(Box::new(e))
            }
        }
    }

    async fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut inner = self.inner.lock().await;

        if inner.stream.is_none() {
            let stream = TcpStream::connect(&inner.addr)
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;
            inner.stream = Some(stream);
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        if let Ok(inner) = self.inner.try_lock() {
            inner.stream.is_some()
        } else {
            false
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Tcp
    }

    async fn rerun(
        &mut self,
        args: crate::cli_tool::RunArgs,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        log::info!("Starting local rerun via TCP...");
        log::info!("  Config: {:?}", args.config);
        log::info!("  Script: {:?}", args.path);
        log::info!("  Output: {:?}", args.output);

        let (shutdown_tx, _) = broadcast::channel(1);
        let shutting_down = Arc::new(AtomicBool::new(false));
        let uuid = uuid::Uuid::new_v4();
        let shutdown_tx_clone = shutdown_tx.clone();

        let shutdown_tx_ctrlc = shutdown_tx.clone();
        let shutting_down_ctrlc = shutting_down.clone();

        tokio::spawn(async move {
            if let Ok(()) = tokio::signal::ctrl_c().await {
                if !shutting_down_ctrlc.load(Ordering::SeqCst) {
                    log::info!("Ctrl-C received in rerun session, initiating shutdown...");
                    shutting_down_ctrlc.store(true, Ordering::SeqCst);
                    let _ = shutdown_tx_ctrlc.send(());
                }
            }
        });

        let handle = std::thread::spawn(move || {
            log::info!("Rerun session thread started");
            crate::cli_tool::run_session(args, shutdown_tx_clone, log::LevelFilter::Info, uuid);
            log::info!("Rerun session thread completed");
        });

        let mut inner = self.inner.lock().await;
        inner.rerun_handle = Some(handle);
        inner.rerun_shutdown_tx = Some(shutdown_tx);
        inner.rerun_shutting_down = Some(shutting_down);

        log::info!("New session spawned successfully!");
        Ok(())
    }
}
