use crate::tui_tool::theme::AppTheme;
use nucleo_matcher::{Config, Matcher, Utf32String};
use ratatui::{
    layout::Rect,
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct FilePicker {
    current_dir: PathBuf,
    all_files: Vec<PathBuf>,
    filtered_files: Vec<PathBuf>,
    query: String,
    cursor_position: usize,
    selected_index: usize,
    list_state: ListState,
    matcher: Matcher,
    extensions: Vec<String>,
    title: String,
    max_depth: usize,
    restricted_mode: bool,
    remote_mode: bool,
    dir_only: bool,
}

impl FilePicker {
    pub fn new(start_dir: PathBuf, extensions: Vec<String>, title: String) -> Self {
        let mut picker = FilePicker {
            current_dir: start_dir.clone(),
            all_files: vec![],
            filtered_files: vec![],
            query: String::new(),
            cursor_position: 0,
            selected_index: 0,
            list_state: ListState::default(),
            matcher: Matcher::new(Config::DEFAULT),
            extensions,
            title,
            max_depth: 4,
            restricted_mode: false,
            remote_mode: false,
            dir_only: false,
        };
        picker.scan_directory();
        picker.update_filtered_files();
        picker
    }
    pub fn new_remote(
        base_dir: PathBuf,
        files: Vec<PathBuf>,
        extensions: Vec<String>,
        title: String,
    ) -> Self {
        let mut picker = FilePicker {
            current_dir: base_dir,
            all_files: files,
            filtered_files: vec![],
            query: String::new(),
            cursor_position: 0,
            selected_index: 0,
            list_state: ListState::default(),
            matcher: Matcher::new(Config::DEFAULT),
            extensions,
            title,
            max_depth: 4,
            restricted_mode: true,
            remote_mode: true,
            dir_only: false,
        };
        picker.update_filtered_files();
        picker
    }
    pub fn new_dir_only(start_dir: PathBuf, title: String) -> Self {
        let mut picker = FilePicker {
            current_dir: start_dir.clone(),
            all_files: vec![],
            filtered_files: vec![],
            query: String::new(),
            cursor_position: 0,
            selected_index: 0,
            list_state: ListState::default(),
            matcher: Matcher::new(Config::DEFAULT),
            extensions: vec![],
            title,
            max_depth: 4,
            restricted_mode: false,
            remote_mode: false,
            dir_only: true,
        };
        picker.scan_directory();
        picker.update_filtered_files();
        picker
    }

    pub fn new_remote_dirs(base_dir: PathBuf, dirs: Vec<PathBuf>, title: String) -> Self {
        let mut picker = FilePicker {
            current_dir: base_dir,
            all_files: dirs,
            filtered_files: vec![],
            query: String::new(),
            cursor_position: 0,
            selected_index: 0,
            list_state: ListState::default(),
            matcher: Matcher::new(Config::DEFAULT),
            extensions: vec![],
            title,
            max_depth: 3,
            restricted_mode: true,
            remote_mode: true,
            dir_only: true,
        };
        picker.update_filtered_files();
        picker
    }
    fn scan_directory(&mut self) {
        if self.remote_mode {
            return;
        }

        self.all_files.clear();

        for entry in WalkDir::new(&self.current_dir)
            .max_depth(self.max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if self.dir_only {
                if path.is_dir() && path != self.current_dir {
                    self.all_files.push(path.to_path_buf());
                }
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if self.extensions.iter().any(|e| {
                        ext.to_string_lossy().to_lowercase()
                            == e.trim_start_matches('.').to_lowercase()
                    }) {
                        self.all_files.push(path.to_path_buf());
                    }
                }
            }
        }

        self.all_files.sort();
    }

    fn update_filtered_files(&mut self) {
        if self.query.is_empty() {
            self.filtered_files = self.all_files.clone();
        } else {
            let query_utf32 = Utf32String::from(self.query.as_str());

            let mut scored_files: Vec<(PathBuf, u16)> = self
                .all_files
                .iter()
                .filter_map(|path| {
                    let display_path = path
                        .strip_prefix(&self.current_dir)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();

                    let haystack_utf32 = Utf32String::from(display_path.as_str());

                    let score = self
                        .matcher
                        .fuzzy_match(haystack_utf32.slice(..), query_utf32.slice(..))?;

                    Some((path.clone(), score))
                })
                .collect();

            scored_files.sort_by(|a, b| b.1.cmp(&a.1));

            self.filtered_files = scored_files.into_iter().map(|(path, _)| path).collect();
        }

        self.selected_index = 0;
        if !self.filtered_files.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    pub fn handle_char(&mut self, c: char) {
        self.query.insert(self.cursor_position, c);
        self.cursor_position += 1;
        self.update_filtered_files();
    }

    pub fn handle_backspace(&mut self) {
        if self.cursor_position > 0 {
            self.query.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
            self.update_filtered_files();
        }
    }

    pub fn handle_delete(&mut self) {
        if self.cursor_position < self.query.len() {
            self.query.remove(self.cursor_position);
            self.update_filtered_files();
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.query.len() {
            self.cursor_position += 1;
        }
    }

    pub fn next_item(&mut self) {
        if self.filtered_files.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.filtered_files.len();
        self.list_state.select(Some(self.selected_index));
    }

    pub fn previous_item(&mut self) {
        if self.filtered_files.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.filtered_files.len() - 1;
        } else {
            self.selected_index -= 1;
        }
        self.list_state.select(Some(self.selected_index));
    }

    pub fn navigate_up(&mut self) {
        if self.restricted_mode {
            return;
        }
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.query.clear();
            self.cursor_position = 0;
            self.scan_directory();
            self.update_filtered_files();
        }
    }

    pub fn navigate_down(&mut self) {
        if self.restricted_mode {
            return;
        }
        if let Some(selected) = self.get_selected() {
            if let Some(parent) = selected.parent() {
                if parent != self.current_dir && parent.starts_with(&self.current_dir) {
                    self.current_dir = parent.to_path_buf();
                    self.query.clear();
                    self.cursor_position = 0;
                    self.scan_directory();
                    self.update_filtered_files();
                }
            }
        }
    }

    pub fn change_directory(&mut self, new_dir: PathBuf) {
        if new_dir.is_dir() {
            self.current_dir = new_dir;
            self.query.clear();
            self.cursor_position = 0;
            self.scan_directory();
            self.update_filtered_files();
        }
    }

    pub fn current_directory(&self) -> &PathBuf {
        &self.current_dir
    }

    pub fn set_max_depth(&mut self, depth: usize) {
        self.max_depth = depth;
    }

    pub fn get_selected(&self) -> Option<PathBuf> {
        if self.selected_index < self.filtered_files.len() {
            Some(self.filtered_files[self.selected_index].clone())
        } else {
            None
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &AppTheme) {
        let popup_area = centered_rect(80, 60, area);

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3),
                ratatui::layout::Constraint::Min(0),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(popup_area);

        let before_cursor = &self.query[..self.cursor_position];
        let after_cursor = &self.query[self.cursor_position..];

        let input_text = vec![Line::from(vec![
            Span::styled(before_cursor, theme.success()),
            Span::styled("█", theme.accent()),
            Span::styled(after_cursor, theme.success()),
        ])];

        let input = Paragraph::new(input_text).block(
            Block::default()
                .title(format!(
                    "{} (Esc: cancel, PgUp/PgDn: navigate dirs)",
                    self.title
                ))
                .borders(Borders::ALL)
                .border_style(theme.active_border()),
        );

        f.render_widget(Clear, popup_area);
        f.render_widget(input, chunks[0]);

        let items: Vec<ListItem> = self
            .filtered_files
            .iter()
            .map(|path| {
                let display = if self.remote_mode && self.dir_only {
                    path.to_string_lossy().to_string()
                } else {
                    path.strip_prefix(&self.current_dir)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string()
                };

                let display = if display.is_empty() {
                    ".".to_string()
                } else {
                    display
                };

                ListItem::new(display).style(theme.fg())
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!("{} matches", self.filtered_files.len()))
                    .borders(Borders::ALL)
                    .border_style(theme.active_border()),
            )
            .highlight_style(theme.highlight())
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        let current_dir_display = self.current_dir.to_string_lossy().to_string();
        let status = Paragraph::new(format!("Dir: {}", current_dir_display)).style(theme.muted());
        f.render_widget(status, chunks[2]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
