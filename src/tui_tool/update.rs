use crate::data_handler::transport::{Transport, TransportType};
use crate::tui_tool::action::Action;
use crate::tui_tool::app::{App, ServerResponse, TabView};
use crate::tui_tool::tabs::state::StateMode;
use crate::tui_tool::widgets::file_picker::FilePicker; // ADD THIS
use itertools::Itertools;
use std::path::PathBuf;
pub fn update<T: Transport + Clone + Send + 'static>(app: &mut App<T>, action: Action) {
    match action {
        Action::Tick => {
            handle_tick(app);
        }

        Action::ServerDataFetched(Ok(response)) => {
            handle_server_data_fetched(app, response);
        }

        Action::ServerDataFetched(Err(e)) => {
            let err = std::io::Error::new(std::io::ErrorKind::Other, e);
            app.handle_transport_error(&err);
            app.session_running = false;
        }

        Action::StateDataFetched(Ok(response)) => {
            handle_state_data_fetched(app, response);
        }

        Action::StateDataFetched(Err(e)) => {
            let err = std::io::Error::new(std::io::ErrorKind::Other, e);
            app.handle_transport_error(&err);
            app.session_running = false;
        }

        Action::NextDevice => app.next_device(),
        Action::PreviousDevice => app.previous_device(),
        Action::NextStream => app.next_stream(),
        Action::PreviousStream => app.previous_stream(),
        Action::SetXAxis => app.set_x_axis(),
        Action::SetYAxis => app.set_y_axis(),
        Action::ClearAxes => {
            app.x_axis_stream = None;
            app.y_axis_stream = None;
            log::info!("Cleared axis selections");
        }

        Action::StateNextPrimary => app.state_tab.next_primary(),
        Action::StatePreviousPrimary => app.state_tab.previous_primary(),
        Action::StateNextSecondary => app.state_tab.next_secondary(),
        Action::StatePreviousSecondary => app.state_tab.previous_secondary(),
        Action::StateToggleFocus => app.state_tab.toggle_focus(),

        Action::StateStartEdit => app.state_tab.start_edit(),
        Action::StateCommitEdit => app.state_tab.commit_edit(),
        Action::StateCancelEdit => app.state_tab.cancel_edit(),
        Action::StateEditInput(c) => app.state_tab.handle_edit_input(c),
        Action::StateEditBackspace => app.state_tab.handle_edit_backspace(),
        Action::StateEditDelete => app.state_tab.handle_edit_delete(),
        Action::StateMoveCursorLeft => app.state_tab.move_cursor_left(),
        Action::StateMoveCursorRight => app.state_tab.move_cursor_right(),
        Action::StateMoveCursorStart => app.state_tab.move_cursor_start(),
        Action::StateMoveCursorEnd => app.state_tab.move_cursor_end(),

        Action::StateStartConfigPicker => app.state_tab.start_config_picker(),

        Action::StateFilePickerKey(key) => match app.state_tab.mode {
            StateMode::PickingConfig => {
                let needs_remote_fetch = app
                    .state_tab
                    .handle_file_picker_key(key, app.transport.transport_type());

                if needs_remote_fetch {
                    let tx = app.action_tx.clone();
                    let mut transport = app.transport.clone();

                    tokio::spawn(async move {
                        if let Some(http) = transport
                            .as_any_mut()
                            .downcast_mut::<crate::server::http_transport::HTTPTransport>(
                        ) {
                            match http.get_allowed_scripts().await {
                                Ok((base_dir, files)) => {
                                    let _ = tx
                                        .send(Action::RemoteScriptsFetched(Ok((base_dir, files))));
                                }
                                Err(e) => {
                                    let _ =
                                        tx.send(Action::RemoteScriptsFetched(Err(e.to_string())));
                                }
                            }
                        }
                    });
                }
            }
            StateMode::PickingScript => {
                let _ = app
                    .state_tab
                    .handle_file_picker_key(key, app.transport.transport_type());
            }
            StateMode::PickingOutputDir => {
                if let Some(ref mut picker) = app.state_tab.file_picker {
                    match key {
                        crossterm::event::KeyCode::Enter => {
                            if let Some(selected) = picker.get_selected() {
                                app.state_tab.set_output_dir(selected);
                                app.state_tab.file_picker = None;
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            app.state_tab.file_picker = None;
                            app.state_tab.mode = StateMode::EditingRunArgs;
                        }
                        _ => {
                            let _ = app
                                .state_tab
                                .handle_file_picker_key(key, app.transport.transport_type());
                        }
                    }
                }
            }
            _ => {
                let _ = app
                    .state_tab
                    .handle_file_picker_key(key, app.transport.transport_type());
            }
        },

        Action::RemoteScriptsFetched(Ok((base_dir, files))) => {
            log::info!("Received {} scripts from server", files.len());
            app.state_tab.set_remote_scripts(base_dir, files);
        }

        Action::RemoteScriptsFetched(Err(e)) => {
            log::error!("Failed to fetch remote scripts: {}", e);
            app.state_tab.mode = StateMode::Normal;
            app.state_tab.file_picker = None;
        }

        Action::StateStartRunArgsEditor => {
            app.state_tab.start_run_args_editor();
        }

        Action::StateRunArgsNextField => {
            app.state_tab.run_args_next_field();
        }

        Action::StateRunArgsPreviousField => {
            app.state_tab.run_args_previous_field();
        }

        Action::StateRunArgsEditCurrent => {
            let needs_dir_fetch = app.state_tab.run_args_edit_current();

            if needs_dir_fetch {
                let transport_type = app.transport.transport_type();

                if transport_type == TransportType::Http {
                    let tx = app.action_tx.clone();
                    let mut transport = app.transport.clone();

                    tokio::spawn(async move {
                        if let Some(http) = transport
                            .as_any_mut()
                            .downcast_mut::<crate::server::http_transport::HTTPTransport>(
                        ) {
                            match http.get_allowed_output_dirs().await {
                                Ok(dirs) => {
                                    let _ = tx.send(Action::OutputDirFetched(Ok(dirs)));
                                }
                                Err(e) => {
                                    let _ = tx.send(Action::OutputDirFetched(Err(e.to_string())));
                                }
                            }
                        }
                    });
                } else {
                    let start_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

                    app.state_tab.file_picker = Some(FilePicker::new_dir_only(
                        start_dir,
                        "Select Output Directory".to_string(),
                    ));
                    app.state_tab.mode = StateMode::PickingOutputDir;
                }
            }
        }

        Action::StateRunArgsEditInput(c) => {
            app.state_tab.run_args_edit_input(c);
        }

        Action::StateRunArgsEditBackspace => {
            app.state_tab.run_args_edit_backspace();
        }

        Action::StateRunArgsEditDelete => {
            app.state_tab.run_args_edit_delete();
        }

        Action::StateRunArgsCommitEdit => {
            app.state_tab.run_args_commit_edit();
        }

        Action::StateRunArgsCancelEdit => {
            app.state_tab.run_args_cancel_edit();
        }

        Action::StateRunArgsConfirm => {
            app.state_tab.run_args_confirm();
            handle_start_new_run(app);
        }

        Action::StateRunArgsCancel => {
            app.state_tab.run_args_cancel();
        }

        Action::OutputDirFetched(Ok(dirs)) => {
            log::info!("Received {} allowed output dirs from server", dirs.len());
            if dirs.is_empty() {
                log::error!("No allowed output directories configured on server");
                app.state_tab.mode = StateMode::EditingRunArgs;
                return;
            }
            let first_dir = dirs.first().unwrap().clone();

            app.state_tab.file_picker = Some(FilePicker::new_remote_dirs(
                first_dir,
                dirs,
                "Select Output Directory".to_string(),
            ));
            app.state_tab.mode = StateMode::PickingOutputDir;
        }

        Action::OutputDirFetched(Err(e)) => {
            log::error!("Failed to fetch allowed output dirs: {}", e);
            app.state_tab.mode = StateMode::EditingRunArgs;
        }

        Action::SwitchTab => app.switch_tab(),
        Action::TogglePopup => app.show_popup = !app.show_popup,

        Action::StartNewRun => {
            if app.session_running {
                log::warn!(
                    "A session is already running. Press 'k' to kill it first, then 'n' to start new run."
                );
                return;
            }

            match app.transport.transport_type() {
                TransportType::Http | TransportType::Ws => {
                    if app.state_tab.loaded_script_path.is_none()
                        && app.state_tab.server_script_path.is_none()
                    {
                        log::warn!("Cannot start new run: No script available.");
                        log::info!("Press 'l' to load a config and script file.");
                        return;
                    }
                }
                TransportType::Tcp => {
                    if !app.state_tab.can_rerun() {
                        log::warn!(
                            "Cannot start new run: No config loaded. Press 'l' to load files first."
                        );
                        return;
                    }
                }
            }

            app.state_tab.start_run_args_editor();
        }

        Action::NewRunStarted(Ok(())) => {
            log::info!("New session started successfully!");
            app.session_running = true;
            app.in_rerun = true;
            if let Some(path) = app.state_tab.server_script_path.as_ref() {
                log::info!("The server will execute: {}", path);
            }
        }

        Action::NewRunStarted(Err(e)) => {
            log::error!("Failed to start new session: {}", e);
            app.in_rerun = false
        }

        Action::KillServer => {
            let mut transport = app.transport.clone();
            tokio::spawn(async move {
                kill_server(&mut transport).await;
            });
        }

        Action::PauseServer => {
            let mut transport = app.transport.clone();
            tokio::spawn(async move {
                pause_server(&mut transport).await;
            });
        }

        Action::ResumeServer => {
            let mut transport = app.transport.clone();
            tokio::spawn(async move {
                resume_server(&mut transport).await;
            });
        }

        Action::Quit => {
            log::info!("Quit requested");
            let mut transport = app.transport.clone();
            let in_rerun = app.in_rerun;
            let remote = app.state_tab.remote;
            let handle = std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create Tokio runtime for shutdown");
                rt.block_on(async {
                    if in_rerun && transport.transport_type() == TransportType::Tcp {
                        if let Some(tcp) = transport
                            .as_any_mut()
                            .downcast_mut::<crate::tcp_handler::TCPTransport>()
                        {
                            tcp.cleanup_rerun().await;
                        }
                    }

                    if !remote {
                        kill_server(&mut transport).await;
                    }
                });
                log::info!("Shutdown complete");
            });

            let _ = handle.join();

            app.should_quit = true;
        }
    }
}

fn handle_tick<T: Transport + Clone + Send + 'static>(app: &mut App<T>) {
    match app.active_tab {
        TabView::Chart => {
            let tx = app.action_tx.clone();
            let mut transport = app.transport.clone();
            tokio::spawn(async move {
                match transport.send_command("GET_DATASTREAM\n").await {
                    Ok(response) => {
                        let _ = tx.send(Action::ServerDataFetched(Ok(response)));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ServerDataFetched(Err(e.to_string())));
                    }
                }
            });
        }
        TabView::State => {
            if app.connection_status {
                let tx = app.action_tx.clone();
                let mut transport = app.transport.clone();
                tokio::spawn(async move {
                    match transport.send_command("STATE\n").await {
                        Ok(response) => {
                            let _ = tx.send(Action::StateDataFetched(Ok(response)));
                        }
                        Err(e) => {
                            let _ = tx.send(Action::StateDataFetched(Err(e.to_string())));
                        }
                    }
                });
            }
        }
    }
}

fn handle_server_data_fetched<T: Transport>(app: &mut App<T>, response: String) {
    if !app.connection_status {
        log::info!("Session started - clearing chart state");
        app.clear_chart_state();
        app.connection_status = true;
        app.has_warned_disconnected = false;
    }

    if !response.is_empty() {
        if !app.session_running {
            log::info!("Session detected as running");
            app.session_running = true;
        }

        match serde_json::from_str::<ServerResponse>(&response) {
            Ok(server_response) => {
                app.devices = server_response
                    .response
                    .into_iter()
                    .sorted_by_key(|(k, _)| k.clone())
                    .map(|(device_key, device_data)| {
                        let streams = device_data
                            .measurements
                            .into_iter()
                            .sorted_by_key(|(k, _)| k.clone())
                            .map(|(name, values)| crate::tui_tool::app::DataStream {
                                name,
                                points: values
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, v)| (i as f64, v))
                                    .collect(),
                            })
                            .collect();
                        crate::tui_tool::app::Device {
                            name: device_key,
                            streams,
                        }
                    })
                    .collect();
            }
            Err(e) => {
                log::warn!("Failed to parse server response: {}", e);
                log::debug!("Response was: {}", response);
            }
        }
    } else {
        if app.session_running {
            log::info!("Session ended");
            app.session_running = false;
        }
    }
}

fn handle_state_data_fetched<T: Transport>(app: &mut App<T>, response: String) {
    if !app.connection_status {
        log::info!("Session started");
        app.connection_status = true;
        app.has_warned_disconnected = false;
    }

    if !response.is_empty() {
        if !app.session_running {
            log::info!("Session detected as running");
            app.session_running = true;
        }
        let _ = app.state_tab.update_from_json(&response);
    } else {
        if app.session_running {
            log::info!("Session ended");
            app.session_running = false;
        }
    }
}

async fn kill_server<T: Transport>(transport: &mut T) {
    match transport.send_command("KILL\n").await {
        Ok(response) => log::info!("Kill command response: {}", response),
        Err(e) => log::error!("Kill command failed: {}", e),
    }
}

async fn pause_server<T: Transport>(transport: &mut T) {
    match transport.send_command("PAUSE_STATE\n").await {
        Ok(response) => log::info!("Pause command response: {}", response),
        Err(e) => log::error!("Pause command failed: {}", e),
    }
}

async fn resume_server<T: Transport>(transport: &mut T) {
    match transport.send_command("RESUME_STATE\n").await {
        Ok(response) => log::info!("Resume command response: {}", response),
        Err(e) => log::error!("Resume command failed: {}", e),
    }
}

fn handle_start_new_run<T: Transport + Clone + Send + 'static>(app: &mut App<T>) {
    let run_args_result = match app.transport.transport_type() {
        TransportType::Http | TransportType::Ws => app.state_tab.build_http_run_args(),
        TransportType::Tcp => app.state_tab.build_run_args(),
    };

    match run_args_result {
        Ok(run_args) => {
            app.in_rerun = true;
            let tx = app.action_tx.clone();
            let mut transport = app.transport.clone();

            tokio::spawn(async move {
                match transport.rerun(run_args).await {
                    Ok(()) => {
                        let _ = tx.send(Action::NewRunStarted(Ok(())));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::NewRunStarted(Err(e.to_string())));
                    }
                }
            });
        }
        Err(e) => {
            log::error!("Failed to build run arguments: {}", e);
        }
    }
}
