use crate::data_handler::configurable_dir_path;
use crate::data_handler::transport::TransportType;
use crate::data_handler::{SessionInfo, SessionMetadata, Summary};

use crate::tui_tool::theme::AppTheme;
use crate::tui_tool::widgets::file_picker::FilePicker;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc};
use std::thread::JoinHandle;
use tokio::sync::broadcast;
use toml::Value;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunArgsField {
    OutputDir,
    Loops,
    Delay,
    DryRun,
}

impl RunArgsField {
    pub fn next(&self) -> Self {
        match self {
            RunArgsField::OutputDir => RunArgsField::Loops,
            RunArgsField::Loops => RunArgsField::Delay,
            RunArgsField::Delay => RunArgsField::DryRun,
            RunArgsField::DryRun => RunArgsField::OutputDir,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            RunArgsField::OutputDir => RunArgsField::DryRun,
            RunArgsField::Loops => RunArgsField::OutputDir,
            RunArgsField::Delay => RunArgsField::Loops,
            RunArgsField::DryRun => RunArgsField::Delay,
        }
    }
}

pub struct RunArgsEditor {
    pub focus_field: RunArgsField,
    pub editing: bool,
    pub edit_buffer: String,
    pub cursor_position: usize,
    pub temp_output: String,
    pub temp_loops: u8,
    pub temp_delay: u64,
    pub temp_dry_run: bool,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusSection {
    Session,
    Device,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StateMode {
    Normal,
    PickingConfig,
    PickingScript,
    FetchingScripts,
    PickingOutputDir,
    EditingRunArgs,
}
#[derive(Debug, Deserialize)]
struct Config {
    #[serde(alias = "experiment")]
    session: Option<Session>,
    device: HashMap<String, DeviceConfig>,
}

#[derive(Debug, Deserialize)]
struct Session {
    info: SessionInfo,
}
#[derive(Debug, Deserialize)]
pub struct DeviceConfig {
    #[serde(flatten)]
    pub device_config: HashMap<String, Value>,
}
pub struct StateTab {
    pub session_info: Option<SessionInfo>,
    pub device_configs: HashMap<String, HashMap<String, Value>>,
    pub focus: FocusSection,
    pub session_fields_state: ListState,
    pub device_list_state: ListState,
    pub device_fields_state: ListState,
    session_field_names: Vec<String>,
    session_field_values: Vec<String>,
    device_names: Vec<String>,
    device_field_names: Vec<String>,
    device_field_values: Vec<String>,
    pub editing: bool,
    pub edit_buffer: String,
    pub cursor_position: usize,
    pub editing_field_name: String,
    pub editing_is_session: bool,
    pub mode: StateMode,
    pub file_picker: Option<FilePicker>,
    pub loaded_config_path: Option<PathBuf>,
    pub loaded_script_path: Option<PathBuf>,
    pub server_script_path: Option<String>,
    pub remote: bool,
    pub rerun_handle: Option<JoinHandle<()>>,
    pub rerun_shutdown_tx: Option<broadcast::Sender<()>>,
    pub rerun_shutting_down: Option<Arc<AtomicBool>>,
    pub run_args_output: String,
    pub run_args_loops: u8,
    pub run_args_delay: u64,
    pub run_args_dry_run: bool,
    pub run_args_editor: Option<RunArgsEditor>,
}

impl StateTab {
    pub fn new(remote: bool) -> Self {
        let mut session_fields_state = ListState::default();
        session_fields_state.select(Some(0));

        let mut device_list_state = ListState::default();
        device_list_state.select(None);

        let mut device_fields_state = ListState::default();
        device_fields_state.select(None);
        let default_output = if remote {
            if let Ok(config) = crate::data_handler::get_configuration() {
                let allowed_dirs = config.get_allowed_output_dirs();
                allowed_dirs
                    .first()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| {
                        std::env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."))
                            .to_string_lossy()
                            .to_string()
                    })
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .to_string_lossy()
                    .to_string()
            }
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .to_string_lossy()
                .to_string()
        };
        StateTab {
            session_info: None,
            device_configs: HashMap::new(),
            focus: FocusSection::Session,
            session_fields_state,
            device_list_state,
            device_fields_state,
            session_field_names: vec![],
            session_field_values: vec![],
            device_names: vec![],
            device_field_names: vec![],
            device_field_values: vec![],
            editing: false,
            edit_buffer: String::new(),
            cursor_position: 0,
            editing_field_name: String::new(),
            editing_is_session: false,
            mode: StateMode::Normal,
            file_picker: None,
            loaded_config_path: None,
            loaded_script_path: None,
            server_script_path: None,
            remote: remote,
            rerun_handle: None,
            rerun_shutdown_tx: None,
            rerun_shutting_down: None,

            run_args_output: default_output,
            run_args_loops: 1,
            run_args_delay: 0,
            run_args_dry_run: false,
            run_args_editor: None,
        }
    }
    pub fn set_remote_scripts(&mut self, base_dir: PathBuf, files: Vec<PathBuf>) {
        self.file_picker = Some(FilePicker::new_remote(
            base_dir,
            files,
            vec![".py".to_string(), ".rs".to_string(), ".m".to_string()],
            "Select Allowed Script".to_string(),
        ));
        self.mode = StateMode::PickingScript;
    }
    pub fn start_config_picker(&mut self) {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.file_picker = Some(FilePicker::new(
            current_dir,
            vec![".toml".to_string()],
            "Select Config File".to_string(),
        ));
        self.mode = StateMode::PickingConfig;
    }

    pub fn handle_file_picker_key(
        &mut self,
        key: crossterm::event::KeyCode,
        transport: TransportType,
    ) -> bool {
        if let Some(ref mut picker) = self.file_picker {
            match key {
                crossterm::event::KeyCode::PageUp => {
                    picker.navigate_up();
                    return false;
                }
                crossterm::event::KeyCode::PageDown => {
                    picker.navigate_down();
                    return false;
                }
                crossterm::event::KeyCode::Char(c) => {
                    picker.handle_char(c);
                    false
                }
                crossterm::event::KeyCode::Backspace => {
                    picker.handle_backspace();
                    false
                }
                crossterm::event::KeyCode::Delete => {
                    picker.handle_delete();
                    false
                }
                crossterm::event::KeyCode::Left => {
                    picker.move_cursor_left();
                    false
                }
                crossterm::event::KeyCode::Right => {
                    picker.move_cursor_right();
                    false
                }
                crossterm::event::KeyCode::Down => {
                    picker.next_item();
                    false
                }
                crossterm::event::KeyCode::Up => {
                    picker.previous_item();
                    false
                }
                crossterm::event::KeyCode::Enter => {
                    if let Some(selected) = picker.get_selected() {
                        match self.mode {
                            StateMode::PickingConfig => {
                                self.loaded_config_path = Some(selected.clone());
                                log::info!("Selected config: {:?}", selected);
                                if let Err(e) = self.load_config_from_file(&selected) {
                                    log::error!("Failed to load config: {}", e);
                                }

                                self.file_picker = None;

                                if transport == TransportType::Http {
                                    self.mode = StateMode::FetchingScripts;
                                    return true;
                                } else {
                                    let current_dir = std::env::current_dir()
                                        .unwrap_or_else(|_| PathBuf::from("."));
                                    self.file_picker = Some(FilePicker::new(
                                        current_dir,
                                        vec![
                                            ".py".to_string(),
                                            ".rs".to_string(),
                                            ".m".to_string(),
                                        ],
                                        "Select Script File".to_string(),
                                    ));
                                    self.mode = StateMode::PickingScript;
                                    return false;
                                }
                            }
                            StateMode::PickingScript => {
                                self.loaded_script_path = Some(selected.clone());
                                log::info!("Selected script file: {:?}", selected);
                                self.file_picker = None;
                                self.mode = StateMode::Normal;
                                log::info!("Ready to run with config and script file");
                                return false;
                            }
                            _ => {}
                        }
                    }
                    false
                }
                crossterm::event::KeyCode::Esc => {
                    self.file_picker = None;
                    self.mode = StateMode::Normal;
                    log::info!("File picker cancelled");
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    fn load_config_from_file(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;

        if let Some(session) = config.session {
            self.session_info = Some(session.info);
            self.refresh_session_lists();
        }

        self.update_device_configs(
            config
                .device
                .into_iter()
                .map(|(name, cfg)| (name, cfg.device_config))
                .collect(),
        );

        log::info!("Successfully loaded config from {:?}", path);
        Ok(())
    }

    pub fn update_from_json(&mut self, json: &str) -> Result<(), Box<dyn std::error::Error>> {
        let summary: Summary = serde_json::from_str(json)?;
        self.session_info = Some(summary.entities.info);
        self.update_device_configs(summary.devices);

        if !summary.run_file.is_empty() {
            self.server_script_path = Some(summary.run_file);
        }

        self.refresh_session_lists();
        Ok(())
    }

    pub fn update_device_configs(&mut self, configs: HashMap<String, HashMap<String, Value>>) {
        self.device_configs = configs;
        self.refresh_device_lists();

        if self.device_list_state.selected().is_none() && !self.device_names.is_empty() {
            self.device_list_state.select(Some(0));
            self.refresh_device_field_lists();
        }
    }

    fn refresh_session_lists(&mut self) {
        if let Some(ref info) = self.session_info {
            self.session_field_names = vec![
                "Name".to_string(),
                "Email".to_string(),
                "Session Name".to_string(),
                "Description".to_string(),
            ];

            self.session_field_values = vec![
                info.name.clone(),
                info.email.clone(),
                info.session_name.clone(),
                info.session_description.clone(),
            ];

            if let Some(ref meta) = info.meta {
                let mut meta_fields: Vec<(&String, &Value)> = meta.meta.iter().collect();
                meta_fields.sort_by(|a, b| a.0.cmp(&b.0));
                for (key, value) in meta_fields {
                    self.session_field_names.push(format!("meta.{}", key));
                    self.session_field_values.push(format_value(value));
                }
            }

            if self.session_fields_state.selected().is_none() {
                self.session_fields_state.select(Some(0));
            }
        }
    }

    fn refresh_device_lists(&mut self) {
        self.device_names = self.device_configs.keys().cloned().collect();
        self.device_names.sort();
    }

    fn refresh_device_field_lists(&mut self) {
        if let Some(selected_idx) = self.device_list_state.selected() {
            if selected_idx < self.device_names.len() {
                let device_name = &self.device_names[selected_idx];
                if let Some(config) = self.device_configs.get(device_name) {
                    let mut fields: Vec<(String, String)> = config
                        .iter()
                        .map(|(k, v)| (k.clone(), format_value(v)))
                        .collect();
                    fields.sort_by(|a, b| a.0.cmp(&b.0));

                    self.device_field_names = fields.iter().map(|(k, _)| k.clone()).collect();
                    self.device_field_values = fields.iter().map(|(_, v)| v.clone()).collect();

                    if self.device_fields_state.selected().is_none()
                        && !self.device_field_names.is_empty()
                    {
                        self.device_fields_state.select(Some(0));
                    }
                    return;
                }
            }
        }
        self.device_field_names.clear();
        self.device_field_values.clear();
        self.device_fields_state.select(None);
    }

    pub fn next_primary(&mut self) {
        match self.focus {
            FocusSection::Session => {
                let len = self.session_field_names.len();
                if len == 0 {
                    return;
                }
                let i = self.session_fields_state.selected().unwrap_or(0);
                self.session_fields_state.select(Some((i + 1) % len));
            }
            FocusSection::Device => {
                let len = self.device_names.len();
                if len == 0 {
                    return;
                }
                let i = self.device_list_state.selected().unwrap_or(0);
                self.device_list_state.select(Some((i + 1) % len));
                self.refresh_device_field_lists();
            }
        }
    }

    pub fn previous_primary(&mut self) {
        match self.focus {
            FocusSection::Session => {
                let len = self.session_field_names.len();
                if len == 0 {
                    return;
                }
                let i = self.session_fields_state.selected().unwrap_or(0);
                self.session_fields_state
                    .select(Some(if i == 0 { len - 1 } else { i - 1 }));
            }
            FocusSection::Device => {
                let len = self.device_names.len();
                if len == 0 {
                    return;
                }
                let i = self.device_list_state.selected().unwrap_or(0);
                self.device_list_state
                    .select(Some(if i == 0 { len - 1 } else { i - 1 }));
                self.refresh_device_field_lists();
            }
        }
    }

    pub fn next_secondary(&mut self) {
        if self.focus == FocusSection::Device {
            let len = self.device_field_names.len();
            if len == 0 {
                return;
            }
            let i = self.device_fields_state.selected().unwrap_or(0);
            self.device_fields_state.select(Some((i + 1) % len));
        }
    }

    pub fn previous_secondary(&mut self) {
        if self.focus == FocusSection::Device {
            let len = self.device_field_names.len();
            if len == 0 {
                return;
            }
            let i = self.device_fields_state.selected().unwrap_or(0);
            self.device_fields_state
                .select(Some(if i == 0 { len - 1 } else { i - 1 }));
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            FocusSection::Session => FocusSection::Device,
            FocusSection::Device => FocusSection::Session,
        };
    }

    pub fn start_edit(&mut self) {
        match self.focus {
            FocusSection::Session => {
                if let Some(idx) = self.session_fields_state.selected() {
                    if idx < self.session_field_values.len() {
                        self.editing = true;
                        self.editing_is_session = true;
                        self.editing_field_name = self.session_field_names[idx].clone();
                        self.edit_buffer = self.session_field_values[idx].clone();
                        self.cursor_position = self.edit_buffer.len();
                    }
                }
            }
            FocusSection::Device => {
                if let Some(idx) = self.device_fields_state.selected() {
                    if idx < self.device_field_values.len() {
                        self.editing = true;
                        self.editing_is_session = false;
                        self.editing_field_name = self.device_field_names[idx].clone();
                        self.edit_buffer = self.device_field_values[idx].clone();
                        self.cursor_position = self.edit_buffer.len();
                    }
                }
            }
        }
    }

    pub fn handle_edit_input(&mut self, c: char) {
        self.edit_buffer.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    pub fn handle_edit_backspace(&mut self) {
        if self.cursor_position > 0 {
            self.edit_buffer.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    pub fn handle_edit_delete(&mut self) {
        if self.cursor_position < self.edit_buffer.len() {
            self.edit_buffer.remove(self.cursor_position);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.edit_buffer.len() {
            self.cursor_position += 1;
        }
    }

    pub fn move_cursor_start(&mut self) {
        self.cursor_position = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_position = self.edit_buffer.len();
    }

    pub fn commit_edit(&mut self) {
        if self.editing_is_session {
            if let Some(idx) = self.session_fields_state.selected() {
                if let Some(ref mut info) = self.session_info {
                    match idx {
                        0 => info.name = self.edit_buffer.clone(),
                        1 => info.email = self.edit_buffer.clone(),
                        2 => info.session_name = self.edit_buffer.clone(),
                        3 => info.session_description = self.edit_buffer.clone(),
                        _ => {
                            let field_name = &self.session_field_names[idx];
                            if let Some(meta_key) = field_name.strip_prefix("meta.") {
                                if info.meta.is_none() {
                                    info.meta = Some(SessionMetadata {
                                        meta: HashMap::new(),
                                    });
                                }
                                if let Some(ref mut meta) = info.meta {
                                    let new_value = parse_value(&self.edit_buffer);
                                    meta.meta.insert(meta_key.to_string(), new_value);
                                }
                            }
                        }
                    }
                    self.refresh_session_lists();
                    log::info!("Updated session field: {}", self.editing_field_name);
                }
            }
        } else {
            if let Some(device_idx) = self.device_list_state.selected() {
                if let Some(field_idx) = self.device_fields_state.selected() {
                    if device_idx < self.device_names.len()
                        && field_idx < self.device_field_names.len()
                    {
                        let device_name = self.device_names[device_idx].clone();
                        let field_name = self.device_field_names[field_idx].clone();

                        if let Some(config) = self.device_configs.get_mut(&device_name) {
                            let new_value = parse_value(&self.edit_buffer);
                            config.insert(field_name.clone(), new_value);
                            self.refresh_device_field_lists();
                            log::info!("Updated device '{}' field: {}", device_name, field_name);
                        }
                    }
                }
            }
        }

        self.editing = false;
        self.edit_buffer.clear();
        self.editing_field_name.clear();
        self.cursor_position = 0;
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
        self.editing_field_name.clear();
        self.cursor_position = 0;
        log::info!("Cancelled edit");
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, show_popup: bool, theme: &AppTheme) {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_session_section(f, vertical_chunks[0], theme);

        let device_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(vertical_chunks[1]);

        self.render_device_list(f, device_chunks[0], theme);
        self.render_device_fields(f, device_chunks[1], theme);

        if let Some(ref mut picker) = self.file_picker {
            picker.render(f, area, theme);
            return;
        }

        if self.editing {
            self.render_edit_popup(f, area, theme);
        }

        if self.mode == StateMode::EditingRunArgs {
            self.render_run_args_popup(f, area, theme);
        }

        if show_popup {
            render_state_help_popup(f, area, theme);
        }
    }

    fn render_session_section(&mut self, f: &mut Frame, area: Rect, theme: &AppTheme) {
        let is_active = self.focus == FocusSection::Session;
        let border_style = if is_active {
            theme.active_border()
        } else {
            theme.inactive_border()
        };

        let mut title = "Session Info (↑↓ to navigate, e to edit)".to_string();
        if self.loaded_config_path.is_some() {
            title.push_str(" [Config from file]");
        }
        if self.loaded_script_path.is_some() {
            title.push_str(" [Script from file]");
        } else if self.server_script_path.is_some() {
            title.push_str(" [Script from server]");
        }

        let items: Vec<ListItem> = self
            .session_field_names
            .iter()
            .zip(self.session_field_values.iter())
            .map(|(name, value)| {
                let content = format!("{}: {}", name, value);
                ListItem::new(content).style(theme.info())
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(theme.highlight())
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.session_fields_state);
    }

    fn render_device_list(&mut self, f: &mut Frame, area: Rect, theme: &AppTheme) {
        let is_active = self.focus == FocusSection::Device;
        let border_style = if is_active {
            theme.active_border()
        } else {
            theme.inactive_border()
        };

        let items: Vec<ListItem> = self
            .device_names
            .iter()
            .map(|name| ListItem::new(name.as_str()).style(theme.success()))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Devices (↑↓ to navigate)")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(theme.highlight())
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.device_list_state);
    }

    fn render_device_fields(&mut self, f: &mut Frame, area: Rect, theme: &AppTheme) {
        let is_active = self.focus == FocusSection::Device;
        let border_style = if is_active {
            theme.active_border()
        } else {
            theme.inactive_border()
        };

        if self.device_field_names.is_empty() {
            let paragraph = Paragraph::new("Select a device")
                .style(theme.muted())
                .block(
                    Block::default()
                        .title("Device Config")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
            f.render_widget(paragraph, area);
            return;
        }

        let items: Vec<ListItem> = self
            .device_field_names
            .iter()
            .zip(self.device_field_values.iter())
            .map(|(name, value)| {
                let content = format!("{}: {}", name, value);
                ListItem::new(content).style(theme.accent())
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Device Config (←→ to navigate, e to edit)")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(theme.highlight())
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.device_fields_state);
    }
    fn save_config_to_temp_file(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("rex_rerun_{}.toml", uuid::Uuid::new_v4()));

        let mut toml_content = String::new();

        toml_content.push_str("[session.info]\n");
        if let Some(ref info) = self.session_info {
            toml_content.push_str(&format!("name = \"{}\"\n", escape_toml_string(&info.name)));
            toml_content.push_str(&format!(
                "email = \"{}\"\n",
                escape_toml_string(&info.email)
            ));
            toml_content.push_str(&format!(
                "session_name = \"{}\"\n",
                escape_toml_string(&info.session_name)
            ));
            toml_content.push_str(&format!(
                "session_description = \"{}\"\n",
                escape_toml_string(&info.session_description)
            ));

            if let Some(ref meta) = info.meta {
                if !meta.meta.is_empty() {
                    toml_content.push_str("\n[session.info.meta]\n");
                    for (key, value) in &meta.meta {
                        toml_content.push_str(&format!(
                            "{} = {}\n",
                            key,
                            toml_value_to_string(value)
                        ));
                    }
                }
            }
        }

        for (device_name, config) in &self.device_configs {
            toml_content.push_str(&format!("\n[device.{}]\n", device_name));
            for (key, value) in config {
                toml_content.push_str(&format!("{} = {}\n", key, toml_value_to_string(value)));
            }
        }

        let mut file = std::fs::File::create(&temp_file)?;
        file.write_all(toml_content.as_bytes())?;

        log::info!("Saved config to: {:?}", temp_file);
        Ok(temp_file)
    }
    pub fn can_rerun(&self) -> bool {
        let has_config = self.session_info.is_some() && !self.device_configs.is_empty();
        // WIP will eventually be token based for running remote python scripts but allow for local search of allowed scritps
        if self.remote {
            has_config && self.server_script_path.is_some()
        } else {
            let has_script = self.loaded_script_path.is_some() || self.server_script_path.is_some();
            has_config && has_script
        }
    }

    fn get_script_path(&self) -> Option<PathBuf> {
        if self.remote {
            self.loaded_script_path
                .clone()
                .or_else(|| self.server_script_path.as_ref().map(|s| PathBuf::from(s)))
        } else {
            self.loaded_script_path
                .clone()
                .or_else(|| self.server_script_path.as_ref().map(|s| PathBuf::from(s)))
        }
    }
    pub fn build_run_args(&self) -> Result<crate::cli_tool::RunArgs, Box<dyn std::error::Error>> {
        let temp_config = self.save_config_to_temp_file()?;
        let script_path = self.get_script_path().ok_or("No script file available")?;

        Ok(crate::cli_tool::RunArgs {
            path: script_path,
            config: Some(temp_config.to_string_lossy().to_string()),
            output: self.run_args_output.clone(),
            dry_run: self.run_args_dry_run,
            email: None,
            delay: self.run_args_delay,
            loops: self.run_args_loops,
            interactive: false,
            port: None,
            meta_json: None,
        })
    }

    pub fn build_config_json(&self) -> Result<String, Box<dyn std::error::Error>> {
        let session_info = self.session_info.as_ref().ok_or("No session info")?;

        let devices = if !self.device_configs.is_empty() {
            let mut device_map = HashMap::new();
            for (name, config) in &self.device_configs {
                device_map.insert(
                    name.clone(),
                    crate::cli_tool::DeviceConfig {
                        config: config.clone(),
                    },
                );
            }
            Some(device_map)
        } else {
            None
        };

        let minimal_info = crate::cli_tool::MinimalSessionInfo {
            name: session_info.name.clone(),
            email: session_info.email.clone(),
            session_name: session_info.session_name.clone(),
            session_description: session_info.session_description.clone(),
            devices,
        };

        Ok(serde_json::to_string(&minimal_info)?)
    }

    pub fn build_http_run_args(
        &self,
    ) -> Result<crate::cli_tool::RunArgs, Box<dyn std::error::Error>> {
        let script_path = self.get_script_path().ok_or("No script file available")?;
        let config_json = self.build_config_json()?;

        let allowed_dir = get_allowed_scripts_dir()
            .map_err(|e| format!("Cannot verify script location: {}", e))?;

        if !script_path.starts_with(&allowed_dir) {
            return Err("Script must be from allowed scripts directory".into());
        }

        Ok(crate::cli_tool::RunArgs {
            path: script_path,
            config: Some(config_json),
            output: self.run_args_output.clone(),
            dry_run: self.run_args_dry_run,
            email: None,
            delay: self.run_args_delay,
            loops: self.run_args_loops,
            interactive: false,
            port: None,
            meta_json: None,
        })
    }
    fn render_edit_popup(&self, f: &mut Frame, area: Rect, theme: &AppTheme) {
        let popup_area = centered_rect(60, 20, area);

        let before_cursor = &self.edit_buffer[..self.cursor_position];
        let after_cursor = &self.edit_buffer[self.cursor_position..];

        let text = vec![
            Line::from(vec![
                Span::styled("Editing: ", theme.info()),
                Span::styled(&self.editing_field_name, theme.accent()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(before_cursor, theme.success()),
                Span::styled("█", theme.accent()),
                Span::styled(after_cursor, theme.success()),
            ]),
            Line::from(""),
            Line::from(Span::styled("Enter to save | Esc to cancel", theme.muted())),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .title("Edit Value")
                    .borders(Borders::ALL)
                    .border_style(theme.active_border()),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(Clear, popup_area);
        f.render_widget(paragraph, popup_area);
    }
    pub fn start_run_args_editor(&mut self) {
        self.run_args_editor = Some(RunArgsEditor {
            focus_field: RunArgsField::OutputDir,
            editing: false,
            edit_buffer: String::new(),
            cursor_position: 0,
            temp_output: self.run_args_output.clone(),
            temp_loops: self.run_args_loops,
            temp_delay: self.run_args_delay,
            temp_dry_run: self.run_args_dry_run,
        });
        self.mode = StateMode::EditingRunArgs;
        log::info!("Opened run args editor");
    }

    pub fn run_args_next_field(&mut self) {
        if let Some(ref mut editor) = self.run_args_editor {
            editor.focus_field = editor.focus_field.next();
        }
    }

    pub fn run_args_previous_field(&mut self) {
        if let Some(ref mut editor) = self.run_args_editor {
            editor.focus_field = editor.focus_field.previous();
        }
    }

    pub fn run_args_edit_current(&mut self) -> bool {
        if let Some(ref mut editor) = self.run_args_editor {
            match editor.focus_field {
                RunArgsField::OutputDir => {
                    self.mode = StateMode::PickingOutputDir;

                    return true;
                }
                RunArgsField::Loops => {
                    editor.editing = true;
                    editor.edit_buffer = editor.temp_loops.to_string();
                    editor.cursor_position = editor.edit_buffer.len();
                }
                RunArgsField::Delay => {
                    editor.editing = true;
                    editor.edit_buffer = editor.temp_delay.to_string();
                    editor.cursor_position = editor.edit_buffer.len();
                }
                RunArgsField::DryRun => {
                    editor.temp_dry_run = !editor.temp_dry_run;
                }
            }
        }
        false
    }

    pub fn run_args_edit_input(&mut self, c: char) {
        if let Some(ref mut editor) = self.run_args_editor {
            if editor.editing {
                editor.edit_buffer.insert(editor.cursor_position, c);
                editor.cursor_position += 1;
            }
        }
    }

    pub fn run_args_edit_backspace(&mut self) {
        if let Some(ref mut editor) = self.run_args_editor {
            if editor.editing && editor.cursor_position > 0 {
                editor.edit_buffer.remove(editor.cursor_position - 1);
                editor.cursor_position -= 1;
            }
        }
    }

    pub fn run_args_edit_delete(&mut self) {
        if let Some(ref mut editor) = self.run_args_editor {
            if editor.editing && editor.cursor_position < editor.edit_buffer.len() {
                editor.edit_buffer.remove(editor.cursor_position);
            }
        }
    }

    pub fn run_args_commit_edit(&mut self) {
        if let Some(ref mut editor) = self.run_args_editor {
            if editor.editing {
                match editor.focus_field {
                    RunArgsField::Loops => {
                        if let Ok(value) = editor.edit_buffer.parse::<u8>() {
                            editor.temp_loops = value.clamp(1, 255);
                        }
                    }
                    RunArgsField::Delay => {
                        if let Ok(value) = editor.edit_buffer.parse::<u64>() {
                            editor.temp_delay = value.clamp(0, 3600);
                        }
                    }
                    _ => {}
                }
                editor.editing = false;
                editor.edit_buffer.clear();
                editor.cursor_position = 0;
            }
        }
    }

    pub fn run_args_cancel_edit(&mut self) {
        if let Some(ref mut editor) = self.run_args_editor {
            editor.editing = false;
            editor.edit_buffer.clear();
            editor.cursor_position = 0;
        }
    }

    pub fn run_args_confirm(&mut self) {
        if let Some(editor) = self.run_args_editor.take() {
            self.run_args_output = editor.temp_output;
            self.run_args_loops = editor.temp_loops;
            self.run_args_delay = editor.temp_delay;
            self.run_args_dry_run = editor.temp_dry_run;

            self.mode = StateMode::Normal;
            log::info!(
                "Run args confirmed: output={}, loops={}, delay={}, dry_run={}",
                self.run_args_output,
                self.run_args_loops,
                self.run_args_delay,
                self.run_args_dry_run
            );
        }
    }

    pub fn run_args_cancel(&mut self) {
        self.run_args_editor = None;
        self.mode = StateMode::Normal;
        log::info!("Run args cancelled");
    }

    pub fn set_output_dir(&mut self, path: PathBuf) {
        if let Some(ref mut editor) = self.run_args_editor {
            editor.temp_output = path.to_string_lossy().to_string();
        }
        self.mode = StateMode::EditingRunArgs;
    }
    fn render_run_args_popup(&self, f: &mut Frame, area: Rect, theme: &AppTheme) {
        let editor = match &self.run_args_editor {
            Some(e) => e,
            None => return,
        };

        let popup_area = centered_rect(70, 50, area);

        let block = Block::default()
            .title("Run Configuration")
            .borders(Borders::ALL)
            .border_style(theme.active_border());

        let inner_area = block.inner(popup_area);
        f.render_widget(Clear, popup_area);
        f.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner_area);

        let output_focused = matches!(editor.focus_field, RunArgsField::OutputDir);
        let output_indicator = if output_focused { ">> " } else { "   " };
        let output_style = if output_focused {
            theme.accent().add_modifier(Modifier::BOLD)
        } else {
            theme.fg()
        };

        let output = Paragraph::new(vec![
            Line::from("Output Directory:"),
            Line::from(Span::styled(
                format!("{}{}", output_indicator, editor.temp_output),
                output_style,
            )),
            Line::from(Span::styled("(e to browse)", theme.muted())),
        ]);
        f.render_widget(output, chunks[0]);

        let loops_focused = matches!(editor.focus_field, RunArgsField::Loops);
        let loops_indicator = if loops_focused { ">> " } else { "   " };

        let loops_value_line = if editor.editing && loops_focused {
            let before = &editor.edit_buffer[..editor.cursor_position];
            let after = &editor.edit_buffer[editor.cursor_position..];
            Line::from(vec![
                Span::raw(loops_indicator),
                Span::styled(before, theme.success()),
                Span::styled("█", theme.accent()),
                Span::styled(after, theme.success()),
            ])
        } else {
            let style = if loops_focused {
                theme.accent().add_modifier(Modifier::BOLD)
            } else {
                theme.fg()
            };
            Line::from(Span::styled(
                format!("{}{}", loops_indicator, editor.temp_loops),
                style,
            ))
        };

        let loops = Paragraph::new(vec![
            Line::from("Loops:"),
            loops_value_line,
            Line::from(Span::styled("(e to edit)", theme.muted())),
        ]);
        f.render_widget(loops, chunks[1]);

        let delay_focused = matches!(editor.focus_field, RunArgsField::Delay);
        let delay_indicator = if delay_focused { ">> " } else { "   " };

        let delay_value_line = if editor.editing && delay_focused {
            let before = &editor.edit_buffer[..editor.cursor_position];
            let after = &editor.edit_buffer[editor.cursor_position..];
            Line::from(vec![
                Span::raw(delay_indicator),
                Span::styled(before, theme.success()),
                Span::styled("█", theme.accent()),
                Span::styled(after, theme.success()),
            ])
        } else {
            let style = if delay_focused {
                theme.accent().add_modifier(Modifier::BOLD)
            } else {
                theme.fg()
            };
            Line::from(Span::styled(
                format!("{}{}", delay_indicator, editor.temp_delay),
                style,
            ))
        };

        let delay = Paragraph::new(vec![
            Line::from("Delay (seconds):"),
            delay_value_line,
            Line::from(Span::styled("(e to edit)", theme.muted())),
        ]);
        f.render_widget(delay, chunks[2]);

        let dry_run_focused = matches!(editor.focus_field, RunArgsField::DryRun);
        let dry_run_indicator = if dry_run_focused { ">> " } else { "   " };
        let dry_run_style = if dry_run_focused {
            theme.accent().add_modifier(Modifier::BOLD)
        } else {
            theme.fg()
        };

        let dry_run = Paragraph::new(vec![
            Line::from("Dry Run:"),
            Line::from(Span::styled(
                format!(
                    "{}[{}] {}",
                    dry_run_indicator,
                    if editor.temp_dry_run { "X" } else { " " },
                    if editor.temp_dry_run { "Yes" } else { "No" }
                ),
                dry_run_style,
            )),
            Line::from(Span::styled("(e to toggle)", theme.muted())),
        ]);
        f.render_widget(dry_run, chunks[3]);

        // Help text
        let help =
            Paragraph::new("↑↓ Navigate  e Edit  Enter Confirm  Esc Cancel").style(theme.muted());
        f.render_widget(help, chunks[5]);
    }
}

fn format_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
        toml::Value::Table(_) => "{...}".to_string(),
        toml::Value::Datetime(dt) => dt.to_string(),
    }
}
fn parse_value(s: &str) -> toml::Value {
    if let Ok(i) = s.parse::<i64>() {
        toml::Value::Integer(i)
    } else if let Ok(f) = s.parse::<f64>() {
        toml::Value::Float(f)
    } else if let Ok(b) = s.parse::<bool>() {
        toml::Value::Boolean(b)
    } else {
        toml::Value::String(s.to_string())
    }
}
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_state_help_popup(f: &mut Frame, area: Rect, theme: &AppTheme) {
    let popup_area = centered_rect(60, 60, area);

    let text = vec![
        Line::from(Span::styled(
            "State Tab Controls",
            theme.accent().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Navigation:", theme.bold())),
        Line::from("f - Toggle between Session/Device sections"),
        Line::from("↑/↓ - Navigate session fields OR devices"),
        Line::from("←/→ - Navigate device config fields"),
        Line::from(""),
        Line::from(Span::styled("File Management:", theme.bold())),
        Line::from("l - Load config and script files"),
        Line::from("    TCP: browse local files"),
        Line::from("    HTTP: browse server's registered scripts"),
        Line::from(""),
        Line::from(Span::styled("Running:", theme.bold())),
        Line::from("n - Start new run with current config"),
        Line::from("    TCP: runs locally with loaded script"),
        Line::from("    HTTP: dispatches to server via /run endpoint"),
        Line::from(""),
        Line::from(Span::styled("Editing (when disconnected):", theme.bold())),
        Line::from("e - Edit selected field"),
        Line::from("Enter - Save changes"),
        Line::from("Esc - Cancel edit"),
        Line::from(""),
        Line::from(Span::styled("Server Control:", theme.bold())),
        Line::from("k - Kill server"),
        Line::from("p - Pause server"),
        Line::from("r - Resume server"),
        Line::from(""),
        Line::from(Span::styled("General:", theme.bold())),
        Line::from("Tab - Switch between Chart/State tabs"),
        Line::from("m - Toggle this help"),
        Line::from("q - Quit"),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(theme.fg()),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}

fn toml_value_to_string(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => format!("\"{}\"", escape_toml_string(s)),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(toml_value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        toml::Value::Table(_) => "{}".to_string(),
        toml::Value::Datetime(dt) => format!("\"{}\"", dt),
    }
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
fn get_allowed_scripts_dir() -> Result<PathBuf, String> {
    configurable_dir_path("XDG_CONFIG_HOME", dirs::config_dir)
        .map(|mut path| {
            path.push("rex");
            path.push("scripts");
            path
        })
        .ok_or("Failed to get config directory".to_string())
}
