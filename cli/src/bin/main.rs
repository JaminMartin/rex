use clap::Parser;
use rex_core::cli_tool::{cli_standalone, process_args, run_experiment, Cli, Commands};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tokio::signal::ctrl_c;
use tokio::sync::broadcast;
#[tokio::main]
async fn main() {
    let original_args: Vec<String> = std::env::args().collect();
    let cleaned_args = process_args(original_args);
    let cli = Cli::parse_from(cleaned_args);

    match cli.command {
        Commands::Run(args) => {
            // Create a broadcast channel for shutdown signals
            let (shutdown_tx, _) = broadcast::channel(1);
            let shutting_down = Arc::new(AtomicBool::new(false));
            let shutting_down_clone = shutting_down.clone();
            let shutdown_tx_clone = shutdown_tx.clone();

            tokio::spawn(async move {
                if let Ok(()) = ctrl_c().await {
                    if !shutting_down_clone.load(Ordering::SeqCst) {
                        shutting_down_clone.store(true, Ordering::SeqCst);
                        if let Err(_) = shutdown_tx_clone.send(()) {}
                    }
                }
            });

            let cli_thread = thread::spawn(move || {
                run_experiment(args, shutdown_tx);
            });

            if let Err(_) = cli_thread.join() {}
        }
        Commands::View(args) => cli_standalone(args),
    }
}
