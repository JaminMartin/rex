use clap::Parser;
use rex_core::cli_tool::{
    cli_standalone, get_log_level, init_logger, process_args, run_experiment, Cli, Commands,
};
use rex_core::server::run_server;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tokio::signal::ctrl_c;
use tokio::sync::broadcast;
use uuid::Uuid;
#[tokio::main]
async fn main() {
    let original_args: Vec<String> = std::env::args().collect();
    let cleaned_args = process_args(original_args);
    let cli = Cli::parse_from(cleaned_args);

    match cli.command {
        Commands::Run(args) => {
            let log_level = get_log_level(cli.verbosity);
            init_logger(log_level, args.interactive);
            // Create a broadcast channel for shutdown signals
            let (shutdown_tx, _) = broadcast::channel(1);
            let shutting_down = Arc::new(AtomicBool::new(false));
            let shutting_down_clone = shutting_down.clone();
            let shutdown_tx_clone = shutdown_tx.clone();
            let uuid = Uuid::new_v4();
            tokio::spawn(async move {
                if let Ok(()) = ctrl_c().await {
                    if !shutting_down_clone.load(Ordering::SeqCst) {
                        shutting_down_clone.store(true, Ordering::SeqCst);
                        if let Err(_) = shutdown_tx_clone.send(()) {}
                    }
                }
            });

            let cli_thread = thread::spawn(move || {
                run_experiment(args, shutdown_tx, log_level, uuid);
            });

            if let Err(_) = cli_thread.join() {}
        }
        Commands::View(args) => {
            let log_level = get_log_level(cli.verbosity);
            cli_standalone(args, log_level)
        }

        Commands::Serve(args) => {
            let log_level = get_log_level(cli.verbosity);
            init_logger(log_level, false);
            // Create a broadcast channel for shutdown signals
            let (shutdown_tx, _) = broadcast::channel(1);
            let shutting_down = Arc::new(AtomicBool::new(false));
            let shutting_down_clone = shutting_down.clone();
            let server_shutting_down_clone = shutting_down.clone();
            let shutdown_tx_clone = shutdown_tx.clone();

            tokio::spawn(async move {
                if let Ok(()) = ctrl_c().await {
                    if !shutting_down_clone.load(Ordering::SeqCst) {
                        shutting_down_clone.store(true, Ordering::SeqCst);
                        if let Err(_) = shutdown_tx_clone.send(()) {}
                    }
                }
            });

            run_server(args, server_shutting_down_clone, shutdown_tx, log_level).await;
        }
    }
}
