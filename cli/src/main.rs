use rex_core::cli_tool::cli_parser_core;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tokio;
use tokio::signal::ctrl_c;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    // Create a broadcast channel for shutdown signals
    let (shutdown_tx, _) = broadcast::channel(1);

    // Create a flag to track if we're in the process of shutting down
    let shutting_down = Arc::new(AtomicBool::new(false));
    let shutting_down_clone = shutting_down.clone();

    // Clone the shutdown_tx to be used in the signal handler
    let shutdown_tx_clone = shutdown_tx.clone();

    // Spawn a task to handle Ctrl+C signal
    tokio::spawn(async move {
        if let Ok(()) = ctrl_c().await {
            // Only send shutdown signal if we're not already shutting down
            if !shutting_down_clone.load(Ordering::SeqCst) {
                shutting_down_clone.store(true, Ordering::SeqCst);

                // Send the shutdown signal to all receivers
                if let Err(_) = shutdown_tx_clone.send(()) {}
            }
        }
    });

    // Run cli_parser_core in a separate thread so we don't block the main thread
    let cli_thread = thread::spawn(move || {
        cli_parser_core(shutdown_tx);
    });

    // Wait for the CLI thread to finish
    if let Err(_) = cli_thread.join() {}
}
