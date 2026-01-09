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
            if remote {
                app.state_tab.cleanup_rerun();

                return true;
            } else {
                app.state_tab.cleanup_rerun();
                app.kill_server();
                return true;
            }
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
            if !app.connection_status {
                app.state_tab.start_edit();
            } else {
                log::warn!("Cannot edit while connected to server");
            }
        }
        KeyCode::Char('n') => try_start_new_run(app),

        KeyCode::Char('l') => {
            if app.state_tab.remote {
                log::warn!("Cannot load files in remote mode (security restriction).");
                log::info!("Remote viewers can only use the server's existing script.");
            } else if !app.connection_status {
                app.state_tab.start_config_picker();
            } else {
                log::warn!("Cannot load files while connected to server");
            }
        }
        KeyCode::Char('k') => app.kill_server(),
        KeyCode::Char('p') => app.pause_server(),
        KeyCode::Char('r') => app.resume_server(),
        _ => {}
    }
}
fn try_start_new_run<T: Transport>(app: &mut App<T>) {
    // if app.state_tab.remote && app.transport.transport_type() == TransportType::Tcp {
    //     log::warn!("Cannot start new run: Remote TCP transport does not support this action.");
    //     return;
    // }

    if app.state_tab.remote {
        if app.state_tab.server_script_path.is_none() {
            log::warn!("Cannot start new run: No script available from server.");
            log::info!("The server must have a running script for remote rerun.");
            return;
        }
    } else if !app.state_tab.can_rerun() {
        log::warn!("Cannot start new run: No config loaded. Press 'l' to load files first.");
        return;
    }

    if app.connection_status {
        log::warn!(
            "Server is still running. Press 'k' to kill it first, then 'n' to start new run."
        );
        return;
    }

    if !app.state_tab.remote
        && app.state_tab.loaded_script_path.is_none()
        && app.state_tab.server_script_path.is_none()
    {
        log::warn!("No script file specified. Press 'l' to select one.");
        return;
    }

    if app.state_tab.remote {
        log::info!("Starting remote rerun with server's script...");
    } else {
        log::info!("Starting new run...");
    }

    match app.state_tab.rerun() {
        Ok(()) => {
            log::info!("New session started successfully!");
            if let Some(path) = app.state_tab.server_script_path.as_ref() {
                log::info!("The server will execute: {}", path);
            }
        }
        Err(e) => log::error!("Failed to start new session: {}", e),
    }
}
