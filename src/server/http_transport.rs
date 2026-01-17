use crate::data_handler::transport::{Transport, TransportType};
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
        // Check if this is a RUN command with JSON payload
        if command.starts_with("POST /run") {
            let json_start = command.find('{').ok_or("No JSON payload found")?;
            let json_payload = &command[json_start..];

            let url = format!("{}/run", self.base_url);
            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .body(json_payload.to_string())
                .timeout(Duration::from_secs(5))
                .send()?;

            if response.status() == StatusCode::BAD_GATEWAY {
                return Ok(String::new());
            }

            if !response.status().is_success() {
                return Err(format!("HTTP error: {}", response.status()).into());
            }

            let body = response.text()?;
            return Ok(body);
        }

        // Handle standard commands
        let endpoint = match command.trim() {
            "GET_DATASTREAM" => "/datastream",
            "STATE" => "/status",
            "KILL" => "/kill",
            "PAUSE_STATE" => "/pause",
            "RESUME_STATE" => "/continue",
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
    fn transport_type(&self) -> TransportType {
        TransportType::Http
    }
    // fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    //     let url = format!("{}/", self.base_url);
    //     let response = self
    //         .client
    //         .get(&url)
    //         .timeout(Duration::from_secs(2))
    //         .send()?;

    //     if response.status().is_success() {
    //         Ok(())
    //     } else {
    //         Err(format!("Server returned status: {}", response.status()).into())
    //     }
    // }
    fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/status_check", self.base_url);
        let response = self.client.get(&url).send()?;

        match response.status() {
            StatusCode::OK => Ok(()), // Session is running
            StatusCode::NO_CONTENT => Err("Axum is up, but no session is active".into()),
            _ => Err("Server error".into()),
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
    fn rerun(&mut self, args: crate::cli_tool::RunArgs) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting remote rerun via HTTP...");
        log::info!("  Config: {:?}", args.config);
        log::info!("  Script: {:?}", args.path);
        log::info!("  Output: {:?}", args.output);

        let json_payload = serde_json::to_string(&args)?;
        let response = self.send_command(&format!("POST /run {}\n", json_payload))?;

        log::info!("Remote run response: {}", response);
        Ok(())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
