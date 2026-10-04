use crate::data_handler::transport::{Transport, TransportType};
use crate::data_handler::{ClientMessage, ServerEvent, SessionSnapshot, StreamSnapshot};
use crate::server::http_transport::HTTPTransport;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex as StdMutex};
use std::thread::JoinHandle;
use tokio::sync::Mutex;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct WsTransport {
    http: HTTPTransport,
    ws_url: String,
    cache: Arc<Mutex<Cache>>,
    local_rerun: Option<Arc<StdMutex<LocalRerun>>>,
}

#[derive(Default)]
struct Cache {
    connected: bool,
    stream: Option<StreamSnapshot>,
    session: Option<SessionSnapshot>,
}

#[derive(Default)]
struct LocalRerun {
    handle: Option<JoinHandle<()>>,
    shutdown_tx: Option<broadcast::Sender<()>>,
    shutting_down: Option<Arc<AtomicBool>>,
}

impl WsTransport {
    pub fn new(address: &str) -> Self {
        let http = HTTPTransport::new(address);
        let normalized = address
            .trim_end_matches('/')
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let scheme = if address.starts_with("https://") { "wss" } else { "ws" };
        Self {
            http,
            ws_url: format!("{scheme}://{normalized}/ws"),
            cache: Arc::new(Mutex::new(Cache::default())),
            local_rerun: is_loopback_address(normalized)
                .then(|| Arc::new(StdMutex::new(LocalRerun::default()))),
        }
    }

    async fn spawn_local_rerun(
        &self,
        args: crate::cli_tool::RunArgs,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let Some(local_rerun) = &self.local_rerun else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Only a loopback viewer may start a local rerun",
            )));
        };
        let mut rerun = local_rerun.lock().map_err(|_| {
            Box::new(std::io::Error::other("Local rerun state is unavailable"))
                as Box<dyn std::error::Error + Send>
        })?;
        if rerun.handle.as_ref().is_some_and(|handle| !handle.is_finished()) {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "A local rerun is already active",
            )));
        }

        let (shutdown_tx, _) = broadcast::channel(1);
        let session_shutdown = shutdown_tx.clone();
        let shutting_down = Arc::new(AtomicBool::new(false));
        let runner_shutting_down = shutting_down.clone();
        let handle = std::thread::spawn(move || {
            // Allow a killed direct run to release its listener before the
            // next local run binds the same control port.
            std::thread::sleep(std::time::Duration::from_millis(300));
            if runner_shutting_down.load(Ordering::SeqCst) {
                return;
            }
            crate::cli_tool::run_session(
                args,
                session_shutdown,
                log::LevelFilter::Info,
                uuid::Uuid::new_v4(),
                None,
            );
        });
        rerun.handle = Some(handle);
        rerun.shutdown_tx = Some(shutdown_tx);
        rerun.shutting_down = Some(shutting_down);
        log::info!("Local control plane ended; spawned a new local rex run from the viewer");
        Ok(())
    }

    pub async fn cleanup_local_rerun(&self) {
        let Some(local_rerun) = &self.local_rerun else { return };
        let (handle, shutdown_tx, shutting_down) = match local_rerun.lock() {
            Ok(mut rerun) => (
                rerun.handle.take(),
                rerun.shutdown_tx.take(),
                rerun.shutting_down.take(),
            ),
            Err(_) => return,
        };
        if let Some(shutting_down) = shutting_down {
            shutting_down.store(true, Ordering::SeqCst);
        }
        if let Some(shutdown_tx) = shutdown_tx {
            let _ = shutdown_tx.send(());
        }
        if let Some(handle) = handle {
            let _ = tokio::task::spawn_blocking(move || handle.join()).await;
        }
    }

    async fn connect(&self) -> Result<(), Box<dyn std::error::Error + Send>> {
        if self.cache.lock().await.connected {
            return Ok(());
        }
        let (mut socket, _) = tokio_tungstenite::connect_async(&self.ws_url).await
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(std::io::Error::other(e.to_string())) })?;
        let subscribe = serde_json::to_string(&ClientMessage::Subscribe { projection: None })
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(std::io::Error::other(e.to_string())) })?;
        socket.send(tokio_tungstenite::tungstenite::Message::Text(subscribe.into())).await
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(std::io::Error::other(e.to_string())) })?;
        let cache = self.cache.clone();
        tokio::spawn(async move {
            while let Some(Ok(message)) = socket.next().await {
                let Ok(text) = message.into_text() else { continue };
                let Ok(event) = serde_json::from_str::<ServerEvent>(&text) else { continue };
                let mut cache = cache.lock().await;
                cache.connected = true;
                match event {
                    ServerEvent::Snapshot { session, stream, .. } => {
                        cache.session = session;
                        cache.stream = stream;
                    }
                    ServerEvent::StreamUpdated { stream, session, .. } => {
                        cache.stream = Some(stream);
                        if session.is_some() {
                            cache.session = session;
                        }
                    }
                    ServerEvent::SessionStateChanged { session, .. } => cache.session = session,
                    // Keep the final snapshot available for post-run inspection.
                    ServerEvent::RunFinished { .. } => {}
                    _ => {}
                }
            }
            cache.lock().await.connected = false;
        });
        self.cache.lock().await.connected = true;
        Ok(())
    }
}

#[async_trait]
impl Transport for WsTransport {
    async fn send_command(&mut self, command: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
        self.connect().await?;
        match command.trim() {
            "GET_DATASTREAM" => self.cache.lock().await.stream.as_ref()
                .map(serde_json::to_string).transpose().map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(std::io::Error::other(e.to_string())) })?
                .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotConnected, "No active session")) as Box<dyn std::error::Error + Send>),
            "STATE" => self.cache.lock().await.session.as_ref()
                .map(serde_json::to_string).transpose().map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(std::io::Error::other(e.to_string())) })?
                .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotConnected, "No active session")) as Box<dyn std::error::Error + Send>),
            _ => self.http.send_command(command).await,
        }
    }
    fn is_connected(&self) -> bool { self.cache.try_lock().map(|c| c.connected).unwrap_or(false) }
    async fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> { self.connect().await }
    async fn disconnect(&mut self) -> Option<String> { self.cache.lock().await.connected = false; None }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn transport_type(&self) -> TransportType { TransportType::WebSocket }
    async fn rerun(&mut self, args: crate::cli_tool::RunArgs) -> Result<(), Box<dyn std::error::Error + Send>> {
        match self.http.rerun(args.clone()).await {
            Ok(()) => Ok(()),
            Err(error) => {
                // Do not hold a non-Sync boxed error across the reachability
                // await; the message remains useful if the server is alive.
                let error_message = error.to_string();
                drop(error);
                if self.local_rerun.is_some() && !self.http.is_control_plane_reachable().await {
                    self.spawn_local_rerun(args).await
                } else {
                    Err(Box::new(std::io::Error::other(error_message)))
                }
            }
        }
    }
}

fn is_loopback_address(address: &str) -> bool {
    let host = address
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']').map(|(host, _)| host))
        .unwrap_or_else(|| address.split(':').next().unwrap_or(address));
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}
