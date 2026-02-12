use crossterm::event::KeyCode;
use std::path::PathBuf;
#[derive(Clone, Debug)]
pub enum Action {
    Tick,
    Quit,

    // Chart tab actions
    NextDevice,
    PreviousDevice,
    NextStream,
    PreviousStream,
    SetXAxis,
    SetYAxis,
    ClearAxes,

    // General actions
    SwitchTab,
    TogglePopup,
    KillServer,
    PauseServer,
    ResumeServer,
    StartNewRun,

    // State tab navigation
    StateNextPrimary,
    StatePreviousPrimary,
    StateNextSecondary,
    StatePreviousSecondary,
    StateToggleFocus,

    // State tab editing
    StateStartEdit,
    StateCommitEdit,
    StateCancelEdit,
    StateEditInput(char),
    StateEditBackspace,
    StateEditDelete,
    StateMoveCursorLeft,
    StateMoveCursorRight,
    StateMoveCursorStart,
    StateMoveCursorEnd,

    // State tab file picker
    StateStartConfigPicker,
    StateFilePickerKey(KeyCode),
    RemoteScriptsFetched(Result<(PathBuf, Vec<PathBuf>), String>),
    // Async operation results
    ServerDataFetched(Result<String, String>),
    StateDataFetched(Result<String, String>),
    NewRunStarted(Result<(), String>),
}
