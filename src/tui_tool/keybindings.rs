use crate::data_handler::transport::{Transport, TransportType};

use crate::tui_tool::app::{App, TabView};
use crossterm::event::{KeyCode, KeyEvent};
pub fn handle_key_event<T: Transport>(app: &mut App<T>, event: KeyEvent, remote: bool) -> bool {
    let key = event.code;

    match app.active_tab {
        TabView::State => {
            use crate::tui_tool::tabs::state::StateMode;

            if matches!(
                app.state_tab.mode,
                StateMode::PickingConfig | StateMode::PickingScript
            ) {
                app.state_tab.handle_file_picker_key(key);
                return false;
            }

            if app.state_tab.editing {
                match key {
                    KeyCode::Char(c) => app.state_tab.handle_edit_input(c),
                    KeyCode::Backspace => app.state_tab.handle_edit_backspace(),
                    KeyCode::Delete => app.state_tab.handle_edit_delete(),
                    KeyCode::Left => app.state_tab.move_cursor_left(),
                    KeyCode::Right => app.state_tab.move_cursor_right(),
                    KeyCode::Home => app.state_tab.move_cursor_start(),
                    KeyCode::End => app.state_tab.move_cursor_end(),
                    KeyCode::Enter => app.state_tab.commit_edit(),
                    KeyCode::Esc => app.state_tab.cancel_edit(),
                    _ => {}
                }
                return false;
            }
        }
        _ => {}
    }

    match key {
        KeyCode::Char('q') => {
            // Check if it's TCP transport and cleanup if needed
            if app.transport.transport_type() == TransportType::Tcp {
                // Downcast to access TCP-specific cleanup
                if let Some(tcp_transport) = app
                    .transport
                    .as_any_mut()
                    .downcast_mut::<crate::tcp_handler::TCPTransport>()
                {
                    tcp_transport.cleanup_rerun();
                }
            }

            if !remote {
                app.kill_server();
            }

            return true;
        }
        KeyCode::Tab => {
            app.switch_tab();
        }
        KeyCode::Char('m') => {
            app.show_popup = !app.show_popup;
        }
        _ => match app.active_tab {
            TabView::Chart => handle_chart_keys(app, key),
            TabView::State => handle_state_keys(app, key),
        },
    }
    false
}

fn handle_chart_keys<T: Transport>(app: &mut App<T>, key: KeyCode) {
    match key {
        KeyCode::Down => app.next_device(),
        KeyCode::Up => app.previous_device(),
        KeyCode::Right => app.next_stream(),
        KeyCode::Left => app.previous_stream(),
        KeyCode::Char('x') => app.set_x_axis(),
        KeyCode::Char('y') => app.set_y_axis(),
        KeyCode::Char('k') => app.kill_server(),
        KeyCode::Char('c') => {
            app.x_axis_stream = None;
            app.y_axis_stream = None;
            log::info!("Cleared axis selections");
        }
        KeyCode::Char('p') => app.pause_server(),
        KeyCode::Char('r') => app.resume_server(),
        KeyCode::Char('n') => try_start_new_run(app),

        _ => {}
    }
}

fn handle_state_keys<T: Transport>(app: &mut App<T>, key: KeyCode) {
    use crate::tui_tool::tabs::state::StateMode;

    if matches!(
        app.state_tab.mode,
        StateMode::PickingConfig | StateMode::PickingScript
    ) {
        app.state_tab.handle_file_picker_key(key);
        return;
    }

    if app.state_tab.editing {
        match key {
            KeyCode::Char(c) => app.state_tab.handle_edit_input(c),
            KeyCode::Backspace => app.state_tab.handle_edit_backspace(),
            KeyCode::Delete => app.state_tab.handle_edit_delete(),
            KeyCode::Left => app.state_tab.move_cursor_left(),
            KeyCode::Right => app.state_tab.move_cursor_right(),
            KeyCode::Home => app.state_tab.move_cursor_start(),
            KeyCode::End => app.state_tab.move_cursor_end(),
            KeyCode::Enter => app.state_tab.commit_edit(),
            KeyCode::Esc => app.state_tab.cancel_edit(),
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Down => app.state_tab.next_primary(),
        KeyCode::Up => app.state_tab.previous_primary(),
        KeyCode::Right => app.state_tab.next_secondary(),
        KeyCode::Left => app.state_tab.previous_secondary(),
        KeyCode::Char('f') => app.state_tab.toggle_focus(),
        KeyCode::Char('e') => {
            match (
                app.session_running,
                app.state_tab.remote,
                app.transport.transport_type(),
            ) {
                (true, _, _) => {
                    log::warn!("Cannot edit while a session is running");
                }
                (false, true, TransportType::Tcp) => {
                    log::warn!("Remote TCP is view-only - editing not allowed");
                }
                (false, _, _) => {
                    app.state_tab.start_edit();
                }
            }
        }

        KeyCode::Char('l') => {
            match (
                app.session_running,
                app.state_tab.remote,
                app.transport.transport_type(),
            ) {
                (true, _, _) => {
                    log::warn!("Cannot load files while a session is running");
                }
                (false, true, TransportType::Tcp) => {
                    log::warn!("Remote TCP is view-only - cannot load files");
                }
                (false, _, _) => {
                    app.state_tab.start_config_picker();
                }
            }
        }
        KeyCode::Char('n') => try_start_new_run(app),
        KeyCode::Char('k') => app.kill_server(),
        KeyCode::Char('p') => app.pause_server(),
        KeyCode::Char('r') => app.resume_server(),
        _ => {}
    }
}
fn try_start_new_run<T: Transport>(app: &mut App<T>) {
    if app.state_tab.remote && app.transport.transport_type() == TransportType::Tcp {
        log::warn!("Cannot start new run: Remote TCP is view-only.");
        return;
    }

    if app.session_running {
        log::warn!(
            "A session is already running. Press 'k' to kill it first, then 'n' to start new run."
        );
        return;
    }

    if app.state_tab.remote && app.transport.transport_type() == TransportType::Http {
        if app.state_tab.loaded_script_path.is_none() && app.state_tab.server_script_path.is_none()
        {
            log::warn!("Cannot start new run: No script available.");
            log::info!("Press 'l' to load a config and script file.");
            return;
        }
    }

    if !app.state_tab.remote && !app.state_tab.can_rerun() {
        log::warn!("Cannot start new run: No config loaded. Press 'l' to load files first.");
        return;
    }

    let run_args_result = match app.transport.transport_type() {
        TransportType::Http | TransportType::Ws => app.state_tab.build_http_run_args(),
        TransportType::Tcp => app.state_tab.build_run_args(),
    };

    match run_args_result {
        Ok(run_args) => match app.transport.rerun(run_args) {
            Ok(()) => {
                log::info!("New session started successfully!");
                app.session_running = true; // Mark session as running
                if let Some(path) = app.state_tab.server_script_path.as_ref() {
                    log::info!("The server will execute: {}", path);
                }
            }
            Err(e) => log::error!("Failed to start new session: {}", e),
        },
        Err(e) => log::error!("Failed to build run arguments: {}", e),
    }
}
