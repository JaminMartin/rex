use crate::data_handler::{SessionInfo, SessionMetadata, Summary};
use crate::tui_tool::widgets::file_picker::FilePicker;
use log::LevelFilter;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::broadcast;
use toml::Value;
use uuid::Uuid;

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
}

impl StateTab {
    pub fn new(remote: bool) -> Self {
        let mut session_fields_state = ListState::default();
        session_fields_state.select(Some(0));

        let mut device_list_state = ListState::default();
        device_list_state.select(None);

        let mut device_fields_state = ListState::default();
        device_fields_state.select(None);

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
        }
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

    pub fn start_script_picker(&mut self) {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.file_picker = Some(FilePicker::new(
            current_dir,
            vec![".py".to_string(), ".rs".to_string(), ".m".to_string()],
            "Select Script File".to_string(),
        ));
        self.mode = StateMode::PickingScript;
    }

    pub fn handle_file_picker_key(&mut self, key: crossterm::event::KeyCode) -> bool {
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
                                self.start_script_picker();
                            }
                            StateMode::PickingScript => {
                                self.loaded_script_path = Some(selected.clone());
                                log::info!("Selected script file: {:?}", selected);
                                self.file_picker = None;
                                self.mode = StateMode::Normal;
                                log::info!("Ready to run with config and script file");
                            }
                            _ => {}
                        }
                    }
                    true
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
        let config: toml::Value = toml::from_str(&contents)?;

        if let Some(session_table) = config.get("session").and_then(|v| v.as_table()) {
            if let Some(info_table) = session_table.get("info").and_then(|v| v.as_table()) {
                let mut session_info = SessionInfo {
                    name: info_table
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    email: info_table
                        .get("email")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    session_name: info_table
                        .get("session_name")
                        .or_else(|| info_table.get("experiment_name"))
                        .or_else(|| info_table.get("test_name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    session_description: info_table
                        .get("session_description")
                        .or_else(|| info_table.get("experiment_description"))
                        .or_else(|| info_table.get("test_description"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    meta: None,
                };

                if let Some(meta_table) = info_table.get("meta").and_then(|v| v.as_table()) {
                    let mut meta_map = HashMap::new();
                    for (key, value) in meta_table {
                        meta_map.insert(key.clone(), value.clone());
                    }
                    session_info.meta = Some(SessionMetadata { meta: meta_map });
                }

                self.session_info = Some(session_info);
                self.refresh_session_lists();
            }
        }

        let mut device_configs = HashMap::new();
        if let Some(device_table) = config.get("device").and_then(|v| v.as_table()) {
            for (device_name, device_value) in device_table {
                if let Some(device_config_table) = device_value.as_table() {
                    let mut device_config = HashMap::new();
                    for (config_key, config_value) in device_config_table {
                        device_config.insert(config_key.clone(), config_value.clone());
                    }
                    device_configs.insert(device_name.clone(), device_config);
                }
            }
        }

        self.update_device_configs(device_configs);

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
                for (key, value) in &meta.meta {
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

    pub fn render(&mut self, f: &mut Frame, area: Rect, show_popup: bool) {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_session_section(f, vertical_chunks[0]);

        let device_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(vertical_chunks[1]);

        self.render_device_list(f, device_chunks[0]);
        self.render_device_fields(f, device_chunks[1]);

        if let Some(ref mut picker) = self.file_picker {
            picker.render(f, area);
            return;
        }

        if self.editing {
            self.render_edit_popup(f, area);
        }

        if show_popup {
            render_state_help_popup(f, area);
        }
    }

    fn render_session_section(&mut self, f: &mut Frame, area: Rect) {
        let is_active = self.focus == FocusSection::Session;
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
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
                ListItem::new(content).style(Style::default().fg(Color::Cyan))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.session_fields_state);
    }

    fn render_device_list(&mut self, f: &mut Frame, area: Rect) {
        let is_active = self.focus == FocusSection::Device;
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let items: Vec<ListItem> = self
            .device_names
            .iter()
            .map(|name| ListItem::new(name.as_str()).style(Style::default().fg(Color::Green)))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Devices (↑↓ to navigate)")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.device_list_state);
    }

    fn render_device_fields(&mut self, f: &mut Frame, area: Rect) {
        let is_active = self.focus == FocusSection::Device;
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        if self.device_field_names.is_empty() {
            let paragraph = Paragraph::new("Select a device")
                .style(Style::default().fg(Color::DarkGray))
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
                ListItem::new(content).style(Style::default().fg(Color::Yellow))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Device Config (←→ to navigate, e to edit)")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
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

        let output_dir = self
            .loaded_config_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        Ok(crate::cli_tool::RunArgs {
            path: script_path,
            config: Some(temp_config.to_string_lossy().to_string()),
            output: output_dir.to_string_lossy().to_string(),
            dry_run: false,
            email: None,
            delay: 0,
            loops: 1,
            interactive: false,
            port: None,
            meta_json: None,
        })
    }

    // Add new method to build config as JSON string for HTTP
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

    // Add method to build HTTP-specific run args
    pub fn build_http_run_args(
        &self,
    ) -> Result<crate::cli_tool::RunArgs, Box<dyn std::error::Error>> {
        let script_path = self.get_script_path().ok_or("No script file available")?;
        let config_json = self.build_config_json()?;

        let output_dir = self
            .loaded_config_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        Ok(crate::cli_tool::RunArgs {
            path: script_path,
            config: Some(config_json), // This is now JSON, not a file path
            output: output_dir.to_string_lossy().to_string(),
            dry_run: false,
            email: None,
            delay: 0,
            loops: 1,
            interactive: false,
            port: None,
            meta_json: None,
        })
    }
    fn render_edit_popup(&self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(60, 20, area);

        let before_cursor = &self.edit_buffer[..self.cursor_position];
        let after_cursor = &self.edit_buffer[self.cursor_position..];

        let text = vec![
            Line::from(vec![
                Span::styled("Editing: ", Style::default().fg(Color::Cyan)),
                Span::styled(&self.editing_field_name, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(before_cursor, Style::default().fg(Color::Green)),
                Span::styled("█", Style::default().fg(Color::Yellow)),
                Span::styled(after_cursor, Style::default().fg(Color::Green)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Enter to save | Esc to cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .title("Edit Value")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(Clear, popup_area);
        f.render_widget(paragraph, popup_area);
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

fn render_state_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 60, area);

    let text = vec![
        Line::from(Span::styled(
            "State Tab Controls",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("f - Toggle between Session/Device sections"),
        Line::from("↑/↓ - Navigate session fields OR devices"),
        Line::from("←/→ - Navigate device config fields"),
        Line::from(""),
        Line::from(Span::styled(
            "File Management (local only):",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("l - Load config and script files from disk"),
        Line::from(""),
        Line::from(Span::styled(
            "Running:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("n - Start new run with current config"),
        Line::from("   Local: can load new scripts"),
        Line::from("   Remote: reruns server's existing script only"),
        Line::from(""),
        Line::from(Span::styled(
            "Editing (when disconnected):",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("e - Edit selected field"),
        Line::from("Enter - Save changes"),
        Line::from("Esc - Cancel edit"),
        Line::from(""),
        Line::from(Span::styled(
            "Server Control:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("k - Kill server"),
        Line::from("p - Pause server"),
        Line::from("r - Resume server"),
        Line::from(""),
        Line::from(Span::styled(
            "General:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("Tab - Switch between Chart/State tabs"),
        Line::from("m - Toggle this help"),
        Line::from("q - Quit"),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
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
        toml::Value::Table(_) => {
            // For nested tables, just serialize as "{}" for now
            "{}".to_string()
        }
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
