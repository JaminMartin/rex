use crate::data_handler::transport::Transport;
use reqwest::blocking::Client;
use std::time::Duration;

use reqwest::StatusCode;

#[derive(Debug)]
pub struct HTTPTransport {
    base_url: String,
    client: Client,
}

impl HTTPTransport {
    pub fn new(addr: &str) -> Self {
        let base_url = if addr.starts_with("http://") || addr.starts_with("https://") {
            addr.to_string()
        } else {
            format!("http://{}", addr)
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        HTTPTransport { base_url, client }
    }
}

impl Transport for HTTPTransport {
    fn send_command(&mut self, command: &str) -> Result<String, Box<dyn std::error::Error>> {
        let endpoint = match command.trim() {
            "GET_DATASTREAM" => "/datastream",
            "STATE" => "/status",
            "KILL" => "/kill",
            "PAUSE_STATE" => "/pause",
            "RESUME_STATE" => "/continue",
            "RUN" => "/run",
            _ => return Err(format!("Unknown command: {}", command).into()),
        };

        let url = format!("{}{}", self.base_url, endpoint);

        let timeout = if endpoint == "/datastream" || endpoint == "/status" {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };

        let response = if endpoint == "/kill" || endpoint == "/pause" || endpoint == "/continue" {
            self.client.post(&url).timeout(timeout).send()?
        } else {
            self.client.get(&url).timeout(timeout).send()?
        };

        if response.status() == StatusCode::BAD_GATEWAY {
            return Ok(String::new());
        }

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()).into());
        }

        let body = response.text()?;
        Ok(body)
    }

    fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/", self.base_url);
        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("Server returned status: {}", response.status()).into())
        }
    }

    fn is_connected(&self) -> bool {
        let url = format!("{}/", self.base_url);
        match self
            .client
            .get(&url)
            .timeout(Duration::from_millis(1000))
            .send()
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    fn disconnect(&mut self) -> Option<String> {
        Some("HTTP transport closed".to_string())
    }
}
