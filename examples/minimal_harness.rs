#!/usr/bin/env rust-script

//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```

use serde_json::json;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn send_message(
    stream: &mut TcpStream,
    message: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    stream.write_all(message.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut response = String::new();
    let mut reader = BufReader::new(stream.try_clone()?);
    reader.read_line(&mut response)?;
    Ok(response.trim().to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("REX_PORT").unwrap_or_else(|_| "7676".to_string());
    let addr = format!("127.0.0.1:{port}");

    let mut stream = TcpStream::connect(&addr)?;

    let session = json!({
        "info": {
            "name": "Rust Harness",
            "email": "harness@example.com",
            "session_name": "rust_harness_session",
            "session_description": "Minimal end-to-end TCP harness"
        }
    });

    let session_response = send_message(&mut stream, &session.to_string())?;
    eprintln!("session -> {session_response}");

    // Five minutes at the existing 1.5 s cadence: long enough to attach the
    // TUI/web viewer, exercise pause/resume, and inspect live streaming.
    for step in 0..200 {
        let x = step as f64 * 0.5;
        let y = x.sin();
        let trace: Vec<f64> = (0..256)
            .map(|i| {
                let t = i as f64 / 32.0;
                (t + x).sin()
            })
            .collect();

        let device = json!({
            "device_name": "demo_device",
            "device_config": {
                "gain": 10,
                "mode": "test"
            },
            "measurements": {
                "x": { "data": [x], "unit": "s" },
                "y": { "data": [y], "unit": "V" },
                "trace": { "data": [trace], "unit": "arb" }
            }
        });

        let device_response = send_message(&mut stream, &device.to_string())?;
        eprintln!("device[{step}] -> {device_response}");
        thread::sleep(Duration::from_millis(1500));
    }

    Ok(())
}
