use rex_core::cli_tool::cli_parser_core;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tokio::signal::ctrl_c;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    // Create a broadcast channel for shutdown signals
    let (shutdown_tx, _) = broadcast::channel(1);

    let shutting_down = Arc::new(AtomicBool::new(false));
    let shutting_down_clone = shutting_down.clone();

  
    let shutdown_tx_clone = shutdown_tx.clone();

   
    tokio::spawn(async move {
        if let Ok(()) = ctrl_c().await {
            // Only send shutdown signal if we're not already shutting down
            if !shutting_down_clone.load(Ordering::SeqCst) {
                shutting_down_clone.store(true, Ordering::SeqCst);

        
                if let Err(_) = shutdown_tx_clone.send(()) {}
            }
        }
    });

    
    let cli_thread = thread::spawn(move || {
        cli_parser_core(shutdown_tx);
    });

  
    if let Err(_) = cli_thread.join() {}
}
