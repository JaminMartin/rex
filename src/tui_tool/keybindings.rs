use crate::data_handler::transport::Transport;
use crate::tui_tool::action::Action;
use crate::tui_tool::app::{App, TabView};
use crate::tui_tool::tabs::state::StateMode;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_key_event<T: Transport>(app: &App<T>, event: KeyEvent) -> Vec<Action> {
    let key = event.code;
    let mut actions = vec![];

    match app.active_tab {
        TabView::State => {
            if matches!(
                app.state_tab.mode,
                StateMode::PickingConfig | StateMode::PickingScript | StateMode::PickingOutputDir
            ) {
                actions.push(Action::StateFilePickerKey(key));
                return actions;
            }

            if app.state_tab.mode == StateMode::EditingRunArgs {
                if let Some(ref editor) = app.state_tab.run_args_editor {
                    if editor.editing {
                        match key {
                            KeyCode::Char(c) => actions.push(Action::StateRunArgsEditInput(c)),
                            KeyCode::Backspace => actions.push(Action::StateRunArgsEditBackspace),
                            KeyCode::Delete => actions.push(Action::StateRunArgsEditDelete),
                            KeyCode::Enter => actions.push(Action::StateRunArgsCommitEdit),
                            KeyCode::Esc => actions.push(Action::StateRunArgsCancelEdit),
                            _ => {}
                        }
                    } else {
                        match key {
                            KeyCode::Up => actions.push(Action::StateRunArgsPreviousField),
                            KeyCode::Down => actions.push(Action::StateRunArgsNextField),
                            KeyCode::Char('e') => actions.push(Action::StateRunArgsEditCurrent),
                            KeyCode::Enter => actions.push(Action::StateRunArgsConfirm),
                            KeyCode::Esc => actions.push(Action::StateRunArgsCancel),
                            _ => {}
                        }
                    }
                }
                return actions;
            }

            if app.state_tab.editing {
                match key {
                    KeyCode::Char(c) => actions.push(Action::StateEditInput(c)),
                    KeyCode::Backspace => actions.push(Action::StateEditBackspace),
                    KeyCode::Delete => actions.push(Action::StateEditDelete),
                    KeyCode::Left => actions.push(Action::StateMoveCursorLeft),
                    KeyCode::Right => actions.push(Action::StateMoveCursorRight),
                    KeyCode::Home => actions.push(Action::StateMoveCursorStart),
                    KeyCode::End => actions.push(Action::StateMoveCursorEnd),
                    KeyCode::Enter => actions.push(Action::StateCommitEdit),
                    KeyCode::Esc => actions.push(Action::StateCancelEdit),
                    _ => {}
                }
                return actions;
            }
        }
        _ => {}
    }

    // Global keys
    match key {
        KeyCode::Char('q') => {
            actions.push(Action::Quit);
        }
        KeyCode::Tab => {
            actions.push(Action::SwitchTab);
        }
        KeyCode::Char('m') => {
            actions.push(Action::TogglePopup);
        }
        _ => match app.active_tab {
            TabView::Chart => {
                actions.extend(handle_chart_keys(key));
            }
            TabView::State => {
                actions.extend(handle_state_keys(key, app));
            }
        },
    }

    actions
}

fn handle_chart_keys(key: KeyCode) -> Vec<Action> {
    let mut actions = vec![];

    match key {
        KeyCode::Down => actions.push(Action::NextDevice),
        KeyCode::Up => actions.push(Action::PreviousDevice),
        KeyCode::Right => actions.push(Action::NextStream),
        KeyCode::Left => actions.push(Action::PreviousStream),
        KeyCode::Char('x') => actions.push(Action::SetXAxis),
        KeyCode::Char('y') => actions.push(Action::SetYAxis),
        KeyCode::Char('k') => actions.push(Action::KillServer),
        KeyCode::Char('c') => actions.push(Action::ClearAxes),
        KeyCode::Char('p') => actions.push(Action::PauseServer),
        KeyCode::Char('r') => actions.push(Action::ResumeServer),
        KeyCode::Char('n') => actions.push(Action::StartNewRun),
        _ => {}
    }

    actions
}

fn handle_state_keys<T: Transport>(key: KeyCode, app: &App<T>) -> Vec<Action> {
    let mut actions = vec![];

    match key {
        KeyCode::Down => actions.push(Action::StateNextPrimary),
        KeyCode::Up => actions.push(Action::StatePreviousPrimary),
        KeyCode::Right => actions.push(Action::StateNextSecondary),
        KeyCode::Left => actions.push(Action::StatePreviousSecondary),
        KeyCode::Char('f') => actions.push(Action::StateToggleFocus),
        KeyCode::Char('e') => {
            if app.session_running {
                log::warn!("Cannot edit while a session is running");
            } else {
                actions.push(Action::StateStartEdit);
            }
        }
        KeyCode::Char('l') => {
            if app.session_running {
                log::warn!("Cannot load files while a session is running");
            } else {
                actions.push(Action::StateStartConfigPicker);
            }
        }
        KeyCode::Char('n') => actions.push(Action::StartNewRun),
        KeyCode::Char('k') => actions.push(Action::KillServer),
        KeyCode::Char('p') => actions.push(Action::PauseServer),
        KeyCode::Char('r') => actions.push(Action::ResumeServer),
        _ => {}
    }

    actions
}
