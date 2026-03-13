use std::sync::Mutex;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::data_handler::transport::{Transport, TransportType};

struct WsCommand {
    command: String,
    respond_to: oneshot::Sender<Result<String, String>>,
}

#[derive(Debug)]
pub struct WebSocketTransport {
    tx: Mutex<Option<mpsc::Sender<WsCommand>>>,
    url: String,
}

impl WebSocketTransport {
    pub fn new(url: &str) -> Self {
        Self {
            tx: Mutex::new(None),
            url: url.to_string(),
        }
    }

    fn start_runtime(&self) -> mpsc::Sender<WsCommand> {
        let (tx, rx) = mpsc::channel(8);
        let url = self.url.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(websocket_task(url, rx));
        });

        tx
    }
}

async fn websocket_task(url: String, mut rx: mpsc::Receiver<WsCommand>) {
    let (ws, _) = connect_async(&url).await.expect("ws connect failed");
    let (mut sender, mut receiver) = ws.split();

    while let Some(cmd) = rx.recv().await {
        if sender
            .send(Message::Text(cmd.command.into()))
            .await
            .is_err()
        {
            let _ = cmd.respond_to.send(Err("send failed".into()));
            break;
        }

        match receiver.next().await {
            Some(Ok(Message::Text(text))) => {
                let _ = cmd.respond_to.send(Ok(text.to_string()));
            }
            Some(Ok(Message::Close(_))) => break,
            _ => {
                let _ = cmd.respond_to.send(Err("ws closed".into()));
                break;
            }
        }
    }
}

impl Transport for WebSocketTransport {
    fn send_command(&mut self, command: &str) -> Result<String, Box<dyn std::error::Error>> {
        self.ensure_connection()?;

        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .lock()
            .unwrap()
            .as_ref()
            .ok_or("WebSocket not connected")?
            .blocking_send(WsCommand {
                command: command.trim().to_string(),
                respond_to: resp_tx,
            })?;

        Ok(resp_rx.blocking_recv()??)
    }

    fn is_connected(&self) -> bool {
        self.tx.lock().expect("web socket not running").is_some()
    }
    fn transport_type(&self) -> TransportType {
        TransportType::Ws
    }
    fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.tx.lock().unwrap();
        if guard.is_none() {
            *guard = Some(self.start_runtime());
        }
        Ok(())
    }

    fn disconnect(&mut self) -> Option<String> {
        self.tx.lock().unwrap().take();
        Some("WebSocket disconnected".into())
    }
}
