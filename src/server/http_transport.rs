use crate::data_handler::transport::{Transport, TransportType};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
#[derive(Clone)]
pub struct HTTPTransport {
    inner: Arc<Mutex<HTTPTransportInner>>,
}

struct HTTPTransportInner {
    base_url: String,
    client: reqwest::Client,
    last_session_check: Option<std::time::Instant>,
    has_active_session: bool,
}

impl HTTPTransport {
    pub fn new(base_url: &str) -> Self {
        let base_url = if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            format!("http://{}", base_url)
        } else {
            base_url.to_string()
        };

        let base_url = base_url.trim_end_matches('/');

        log::info!("Creating HTTPTransport with base_url: {}", base_url);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client");

        HTTPTransport {
            inner: Arc::new(Mutex::new(HTTPTransportInner {
                base_url: base_url.to_string(),
                client,
                last_session_check: None,
                has_active_session: false,
            })),
        }
    }
    pub async fn get_allowed_output_dirs(
        &self,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error + Send>> {
        let inner = self.inner.lock().await;
        let url = format!("{}/allowed_output_dirs", inner.base_url);

        let response = inner
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

        if !response.status().is_success() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get output dirs list",
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

        let dirs: Vec<PathBuf> = json["dirs"]
            .as_array()
            .ok_or_else(|| -> Box<dyn std::error::Error + Send> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid dirs array",
                ))
            })?
            .iter()
            .filter_map(|v| v.as_str().map(PathBuf::from))
            .collect();

        Ok(dirs)
    }
    pub async fn get_allowed_scripts(
        &self,
    ) -> Result<(PathBuf, Vec<PathBuf>), Box<dyn std::error::Error + Send>> {
        let inner = self.inner.lock().await;
        let url = format!("{}/allowed_scripts", inner.base_url);

        let response = inner
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

        if !response.status().is_success() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get scripts list",
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

        let base_dir = PathBuf::from(json["base_dir"].as_str().ok_or_else(
            || -> Box<dyn std::error::Error + Send> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid base_dir",
                ))
            },
        )?);

        let files: Vec<PathBuf> = json["files"]
            .as_array()
            .ok_or_else(|| -> Box<dyn std::error::Error + Send> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid files array",
                ))
            })?
            .iter()
            .filter_map(|v| v.as_str().map(PathBuf::from))
            .collect();

        Ok((base_dir, files))
    }

    async fn check_active_session(&self) -> bool {
        let inner = self.inner.lock().await;
        let url = format!("{}/status_check", inner.base_url);

        match inner.client.get(&url).send().await {
            Ok(response) => response.status() == reqwest::StatusCode::OK,
            Err(_) => false,
        }
    }
}

#[async_trait]
impl Transport for HTTPTransport {
    async fn send_command(
        &mut self,
        command: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send>> {
        let mut inner = self.inner.lock().await;

        let endpoint = match command.trim() {
            "GET_DATASTREAM\n" | "GET_DATASTREAM" => "datastream",
            "STATE\n" | "STATE" => "status",
            "KILL\n" | "KILL" => "kill",
            "PAUSE_STATE\n" | "PAUSE_STATE" => "pause",
            "RESUME_STATE\n" | "RESUME_STATE" => "continue",
            _ => {
                log::warn!("Unknown command: {}", command);
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Unknown command: {}", command),
                )));
            }
        };

        let url = format!("{}/{}", inner.base_url, endpoint);

        let is_post = matches!(endpoint, "kill" | "pause" | "continue");

        let request_builder = if is_post {
            inner.client.post(&url)
        } else {
            inner.client.get(&url)
        };

        let request = match request_builder.build() {
            Ok(req) => req,
            Err(e) => {
                log::error!("Failed to build HTTP request for {}: {}", url, e);
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Request builder error: {}", e),
                )));
            }
        };

        let response = match inner.client.execute(request).await {
            Ok(resp) => resp,
            Err(e) => {
                log::error!("HTTP request to {} failed: {}", url, e);

                inner.has_active_session = false;

                if e.is_timeout() {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("Request to {} timed out", url),
                    )));
                } else if e.is_connect() {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        format!("Could not connect to HTTP server at {}", url),
                    )));
                } else {
                    return Err(Box::new(e));
                }
            }
        };

        let status = response.status();

        if status == reqwest::StatusCode::BAD_GATEWAY {
            inner.has_active_session = false;
            inner.last_session_check = Some(std::time::Instant::now());

            log::debug!("No active session running (502 from HTTP server)");
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No active session",
            )));
        }

        if status.is_success() {
            inner.has_active_session = true;
            inner.last_session_check = Some(std::time::Instant::now());
        }

        if !status.is_success() {
            log::warn!("HTTP {} returned status {}", endpoint, status);
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error body".to_string());
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "HTTP {} returned status {}: {}",
                    endpoint, status, error_body
                ),
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send> {
                log::error!("Failed to read HTTP response: {}", e);
                Box::new(e)
            })?;

        Ok(text)
    }

    fn is_connected(&self) -> bool {
        if let Ok(inner) = self.inner.try_lock() {
            inner.has_active_session
        } else {
            false
        }
    }

    async fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        let has_session = self.check_active_session().await;

        let mut inner = self.inner.lock().await;
        inner.has_active_session = has_session;
        inner.last_session_check = Some(std::time::Instant::now());

        if has_session {
            log::debug!("Active session detected");
            Ok(())
        } else {
            log::debug!("No active session detected");
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "No active session",
            )))
        }
    }

    async fn disconnect(&mut self) -> Option<String> {
        let mut inner = self.inner.lock().await;
        inner.has_active_session = false;
        None
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Http
    }

    async fn rerun(
        &mut self,
        args: crate::cli_tool::RunArgs,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let inner = self.inner.lock().await;

        let payload = serde_json::json!({
            "path": args.path.to_string_lossy(),
            "config": args.config,
            "output": args.output,
            "dry_run": args.dry_run,
            "email": args.email,
            "delay": args.delay,
            "loops": args.loops,
            "interactive": args.interactive,
            "port": args.port,
            "meta_json": args.meta_json,
        });

        let url = format!("{}/run", inner.base_url);
        log::info!("Starting new session via HTTP POST to {}", url);

        let request = match inner.client.post(&url).json(&payload).build() {
            Ok(req) => req,
            Err(e) => {
                log::error!("Failed to build rerun request: {}", e);
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Request builder error: {}", e),
                )));
            }
        };

        let response = inner.client.execute(request).await.map_err(
            |e| -> Box<dyn std::error::Error + Send> {
                log::error!("Failed to start session: {}", e);
                Box::new(e)
            },
        )?;

        if response.status().is_success() {
            let response_text = response.text().await.unwrap_or_default();
            log::info!("New session started via HTTP Response: {}", response_text);

            drop(inner);
            let mut inner = self.inner.lock().await;
            inner.has_active_session = true;
            inner.last_session_check = Some(std::time::Instant::now());

            Ok(())
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("HTTP /run failed with status {}: {}", status, error_text),
            )))
        }
    }
}
