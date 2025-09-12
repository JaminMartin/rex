use clap::error::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::widgets::Paragraph;
extern crate log;
use itertools::Itertools;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    prelude::*,
    widgets::{Axis, Block, Borders, Chart, Clear, Dataset, GraphType, List, ListItem, ListState},
    Frame,
};
use std::{
    io,
    time::{Duration, Instant},
};

use crate::data_handler::DeviceData;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use tui_logger::*;
pub async fn run_tui(address: &str, remote: bool) -> tokio::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(100);
    let app = App::new();

    let res = run_app(&mut terminal, app, tick_rate, address, remote);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerResponse {
    #[serde(flatten)]
    response: HashMap<String, DeviceData>,
}

struct StreamReference {
    device_index: usize,
    stream_index: usize,
}

struct DataStream {
    name: String,
    points: Vec<(f64, f64)>,
}

struct Device {
    name: String,
    streams: Vec<DataStream>,
}

struct App {
    devices: Vec<Device>,
    devices_state: ListState,
    streams_state: ListState,
    x_axis_stream: Option<StreamReference>,
    y_axis_stream: Option<StreamReference>,
    log_messages: Vec<String>,
    tcp_stream: Option<TcpStream>,
    connection_status: bool,
    current_device_streams: Vec<String>,
    show_popup: bool,
    has_warned_disconnected: bool,
}

impl App {
    fn new() -> App {
        let mut devices_state = ListState::default();
        devices_state.select(Some(0));

        let devices: Vec<Device> = vec![];

        let current_device_streams = if !devices.is_empty() {
            devices[0].streams.iter().map(|s| s.name.clone()).collect()
        } else {
            vec![]
        };

        App {
            devices,
            devices_state,
            streams_state: ListState::default(),
            x_axis_stream: None,
            y_axis_stream: None,
            log_messages: vec!["System initialized".to_string()],
            current_device_streams,
            connection_status: true,
            show_popup: false,
            tcp_stream: None,
            has_warned_disconnected: false,
        }
    }

    pub fn connect_to_data_server(&mut self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        match TcpStream::connect(addr) {
            Ok(stream) => {
                self.tcp_stream = Some(stream);
                self.connection_status = true;
                if self.has_warned_disconnected {
                    self.clear_chart_state();
                }
                self.has_warned_disconnected = false;
                log::info!("Connected to server at {addr}");
                Ok(())
            }
            Err(e) => {
                self.tcp_stream = None;
                self.connection_status = false;

                if !self.has_warned_disconnected {
                    log::warn!("Failed to connect to {addr}: {e}");
                    self.has_warned_disconnected = true;
                }

                Err(e.into())
            }
        }
    }
    fn check_connection(&mut self, addr: &str) -> bool {
        // Check if we have a connection
        if self.tcp_stream.is_none() {
            if let Err(_) = self.connect_to_data_server(addr) {
                return false;
            }
        }
        true
    }
    fn clear_chart_state(&mut self) {
        self.x_axis_stream = None;
        self.y_axis_stream = None;
        self.devices.clear();
        self.devices_state.select(if !self.devices.is_empty() {
            Some(0)
        } else {
            None
        });
        self.streams_state.select(None);
        self.current_device_streams.clear();
        log::info!("Cleared chart state due to connection change");
    }
    fn send_command(
        &mut self,
        addr: &str,
        command: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if !self.check_connection(addr) {
            return Err("Not connected to server".into());
        }

        if let Some(ref mut stream) = self.tcp_stream {
            // Send command
            match stream.write_all(command.as_bytes()) {
                Ok(_) => {
                    if let Err(e) = stream.flush() {
                        log::error!("Failed to flush stream: {e}");
                        self.tcp_stream = None;
                        self.connection_status = false;
                        return Err(e.into());
                    }
                }
                Err(e) => {
                    log::error!("Failed to write command: {e}");
                    self.tcp_stream = None;
                    self.connection_status = false;
                    return Err(e.into());
                }
            }

            // Read response
            let mut reader = BufReader::new(stream);
            let mut response = String::new();

            match reader.read_line(&mut response) {
                Ok(0) => {
                    log::info!("Server closed connection");
                    self.tcp_stream = None;
                    self.connection_status = false;
                    Err("Server closed connection".into())
                }
                Ok(_) => Ok(response.trim().to_string()),
                Err(e) => {
                    log::error!("Failed to read response: {e}");
                    self.tcp_stream = None;
                    self.connection_status = false;
                    Err(e.into())
                }
            }
        } else {
            Err("No connection available".into())
        }
    }
    pub fn fetch_server_data(&mut self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        match self.send_command(addr, "GET_DATASTREAM\n") {
            Ok(response) => {
                if !response.is_empty() {
                    match serde_json::from_str::<ServerResponse>(&response) {
                        Ok(server_response) => {
                            self.devices = server_response
                                .response
                                .into_iter()
                                .sorted_by_key(|(device_key, _)| device_key.clone())
                                .map(|(device_key, device_data)| {
                                    let streams: Vec<DataStream> = device_data
                                        .measurements
                                        .into_iter()
                                        .sorted_by_key(|(stream_name, _)| stream_name.clone())
                                        .map(|(stream_name, values)| DataStream {
                                            name: stream_name,
                                            points: values
                                                .into_iter()
                                                .enumerate()
                                                .map(|(idx, value)| (idx as f64, value))
                                                .collect(),
                                        })
                                        .collect();

                                    Device {
                                        name: device_key,
                                        streams,
                                    }
                                })
                                .collect();
                        }
                        Err(e) => log::error!("JSON Deserialization Error: {e}"),
                    }
                }
                Ok(())
            }

            Err(_e) => {
                if self.connection_status {
                    log::warn!(
                        "Not connected to address {addr}. Data server is not running."
                    );
                    self.connection_status = false;
                }
                Ok(())

            }
        }
    }
    fn kill_server(&mut self, addr: &str) {
        match self.send_command(addr, "KILL\n") {
            Ok(response) => {
                log::info!("Kill command response: {response}");
            }
            Err(_e) => {
                if self.connection_status {
                    log::warn!(
                        "Not connected to address {addr}. Data server is not running."
                    );
                    self.connection_status = false;
                }

            }
        }
    }
    fn pause_server(&mut self, addr: &str) {
        match self.send_command(addr, "PAUSE_STATE\n") {
            Ok(response) => {
                log::info!("Pause command response: {response}");
            }
            Err(_e) => {
                if self.connection_status {
                    log::warn!(
                        "Not connected to address {addr}. Data server is not running."
                    );
                    self.connection_status = false;
                }
            }
        }
    }
    fn disconnect(&mut self) {
        if let Some(stream) = self.tcp_stream.take() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        self.connection_status = false;
    }
    fn resume_server(&mut self, addr: &str) {
        match self.send_command(addr, "RESUME_STATE\n") {
            Ok(response) => {
                log::info!("Resume command response: {response}");
            }
            Err(_e) => {
                if self.connection_status {
                    log::warn!(
                        "Not connected to address {addr}. Data server is not running."
                    );
                    self.connection_status = false;
                }
            }
        }
    }
    fn set_x_axis(&mut self) {
        if let Some(device_idx) = self.devices_state.selected() {
            if let Some(stream_idx) = self.streams_state.selected() {
                self.x_axis_stream = Some(StreamReference {
                    device_index: device_idx,
                    stream_index: stream_idx,
                });

                let device = &self.devices[device_idx];
                let stream = &device.streams[stream_idx];

                self.log_messages
                    .push(format!("Set X-axis: {} - {}", device.name, stream.name));
            }
        }
    }

    fn set_y_axis(&mut self) {
        if let Some(device_idx) = self.devices_state.selected() {
            if let Some(stream_idx) = self.streams_state.selected() {
                self.y_axis_stream = Some(StreamReference {
                    device_index: device_idx,
                    stream_index: stream_idx,
                });

                let device = &self.devices[device_idx];
                let stream = &device.streams[stream_idx];

                self.log_messages
                    .push(format!("Set Y-axis: {} - {}", device.name, stream.name));
            }
        }
    }

    fn on_tick(&mut self, address: &str) {
        let _ = self.fetch_server_data(address);
    }

    fn next_device(&mut self) {
        let i = match self.devices_state.selected() {
            Some(i) => {
                if i >= self.devices.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.devices_state.select(Some(i));
        self.update_current_device_streams();
    }

    fn update_current_device_streams(&mut self) {
        if let Some(device_idx) = self.devices_state.selected() {
            if !self.devices.is_empty() && device_idx < self.devices.len() {
                self.current_device_streams = self.devices[device_idx]
                    .streams
                    .iter()
                    .map(|s| s.name.clone())
                    .collect();

                self.streams_state
                    .select(if !self.current_device_streams.is_empty() {
                        Some(0)
                    } else {
                        None
                    });
            } else {
                self.current_device_streams = vec![];
                self.streams_state.select(None);
            }
        }
    }

    fn next_stream(&mut self) {
        if let Some(device_idx) = self.devices_state.selected() {
            let num_streams = self.devices[device_idx].streams.len();

            if num_streams == 0 {
                return;
            }

            let i = match self.streams_state.selected() {
                Some(i) => {
                    if i >= num_streams - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.streams_state.select(Some(i));
        }
    }

    fn previous_stream(&mut self) {
        if let Some(device_idx) = self.devices_state.selected() {
            let num_streams = self.devices[device_idx].streams.len();

            if num_streams == 0 {
                return;
            }

            let i = match self.streams_state.selected() {
                Some(i) => {
                    if i == 0 {
                        num_streams - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.streams_state.select(Some(i));
        }
    }

    fn previous_device(&mut self) {
        let i = match self.devices_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.devices.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.devices_state.select(Some(i));
        self.update_current_device_streams();
    }
}
impl Drop for App {
    fn drop(&mut self) {
        self.disconnect();
    }
}
fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
    address: &str,
    remote: bool,
) -> io::Result<()> {
    let _ = app.connect_to_data_server(address);
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => match remote {
                            true => return Ok(()),

                            false => {
                                app.kill_server(address);
                                return Ok(());
                            }
                        },
                        KeyCode::Down => app.next_device(),
                        KeyCode::Up => app.previous_device(),
                        KeyCode::Right => app.next_stream(),
                        KeyCode::Left => app.previous_stream(),
                        KeyCode::Char('x') => app.set_x_axis(),
                        KeyCode::Char('k') => app.kill_server(address),
                        KeyCode::Char('y') => app.set_y_axis(),
                        KeyCode::Char('m') => app.show_popup = !app.show_popup,
                        KeyCode::Char('c') => {
                            app.x_axis_stream = None;
                            app.y_axis_stream = None;
                            log::info!("Cleared axis selections");
                        }
                        KeyCode::Char('p') => {
                            app.pause_server(address);
                        }
                        KeyCode::Char('r') => {
                            app.resume_server(address);
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick(address);
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(f.area());

    let lists_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    if let (Some(x_ref), Some(y_ref)) = (&app.x_axis_stream, &app.y_axis_stream) {
        let x_stream = &app.devices[x_ref.device_index].streams[x_ref.stream_index];
        let y_stream = &app.devices[y_ref.device_index].streams[y_ref.stream_index];

        let points: Vec<(f64, f64)> = x_stream
            .points
            .iter()
            .zip(y_stream.points.iter())
            .map(|((_, x), (_, y))| (*x, *y))
            .collect();

        if !points.is_empty() {
            let datasets = vec![Dataset::default()
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(&points)];

            let x_values: Vec<f64> = points.iter().map(|(x, _)| *x).collect();
            let y_values: Vec<f64> = points.iter().map(|(_, y)| *y).collect();

            let x_min = x_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let x_max = x_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let y_min = y_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let y_max = y_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

            let x_margin = (x_max - x_min) * 0.1;
            let y_margin = (y_max - y_min) * 0.1;

            let x_labels: Vec<Span> = (0..=4)
                .map(|i| {
                    let val = x_min + (i as f64) * (x_max - x_min) / 4.0;
                    Span::styled(format_axis(val), Style::default().fg(Color::White))
                })
                .collect();

            let y_labels: Vec<Span> = (0..=4)
                .map(|i| {
                    let val = y_min + (i as f64) * (y_max - y_min) / 4.0;
                    Span::styled(format_axis(val), Style::default().fg(Color::White))
                })
                .collect();
            let chart = Chart::new(datasets)
                .block(
                    Block::default()
                        .title(format!("{} vs {}", x_stream.name, y_stream.name))
                        .borders(Borders::ALL),
                )
                .x_axis(
                    Axis::default()
                        .title(x_stream.name.clone())
                        .bounds([x_min - x_margin, x_max + x_margin])
                        .labels(x_labels),
                )
                .y_axis(
                    Axis::default()
                        .title(y_stream.name.clone())
                        .bounds([y_min - y_margin, y_max + y_margin])
                        .labels(y_labels),
                );
            f.render_widget(chart, chunks[0]);
        }
    } else {
        let block = Block::default()
            .title("Select X and Y axes to view data")
            .borders(Borders::ALL);
        f.render_widget(block, chunks[0]);
    }

    let devices: Vec<ListItem> = app
        .devices
        .iter()
        .enumerate()
        .map(|(idx, device)| {
            let prefix = match (app.x_axis_stream.as_ref(), app.y_axis_stream.as_ref()) {
                (Some(x_ref), Some(y_ref))
                    if x_ref.device_index == idx && y_ref.device_index == idx =>
                {
                    "X,Y"
                }
                (Some(x_ref), _) if x_ref.device_index == idx => "X",
                (_, Some(y_ref)) if y_ref.device_index == idx => "Y",
                _ => "  ",
            };
            ListItem::new(format!("[{}] {}", prefix, device.name))
                .style(Style::default().fg(Color::Green))
        })
        .collect();

    let devices_list = List::new(devices)
        .block(
            Block::default()
                .title("Connected Devices (↑↓ to navigate)")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(devices_list, lists_chunk[0], &mut app.devices_state);

    if let Some(device_idx) = app.devices_state.selected() {
        let device = &app.devices[device_idx];
        let streams: Vec<ListItem> = device
            .streams
            .iter()
            .enumerate()
            .map(|(idx, stream)| {
                let prefix = match (app.x_axis_stream.as_ref(), app.y_axis_stream.as_ref()) {
                    (Some(x_ref), Some(y_ref))
                        if x_ref.device_index == device_idx
                            && x_ref.stream_index == idx
                            && y_ref.device_index == device_idx
                            && y_ref.stream_index == idx =>
                    {
                        "X,Y"
                    }
                    (Some(x_ref), _)
                        if x_ref.device_index == device_idx && x_ref.stream_index == idx =>
                    {
                        "X"
                    }
                    (_, Some(y_ref))
                        if y_ref.device_index == device_idx && y_ref.stream_index == idx =>
                    {
                        "Y"
                    }
                    _ => "  ",
                };
                ListItem::new(format!("[{}] {}", prefix, stream.name))
                    .style(Style::default().fg(Color::Yellow))
            })
            .collect();

        let streams_list = List::new(streams)
            .block(
                Block::default()
                    .title("Data Streams (←→ to navigate, x/y to set axes)")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(streams_list, lists_chunk[1], &mut app.streams_state);
    }

    let tui_logger = TuiLoggerWidget::default()
        .style_error(Style::default().fg(Color::Red))
        .style_debug(Style::default().fg(Color::Green))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_trace(Style::default().fg(Color::Magenta))
        .style_info(Style::default().fg(Color::Cyan))
        .block(Block::default().title("System Log").borders(Borders::ALL));
    f.render_widget(tui_logger, chunks[2]);

    let controls = create_controls_widget();
    if app.show_popup {
        let block = Block::bordered().title("Popup");
        let area = popup_area(area, 60, 40);
        f.render_widget(Clear, area); //this clears out the background
        f.render_widget(block, area);
        f.render_widget(controls, area);
    }
}
fn create_controls_widget() -> impl Widget {
    let control_text = vec![
        vec![Span::styled(
            "Navigation:",
            Style::default().add_modifier(Modifier::BOLD),
        )],
        vec![Span::raw("↑/↓     - Navigate devices")],
        vec![Span::raw("←/→     - Navigate streams")],
        vec![Span::raw("")],
        vec![Span::styled(
            "Actions:",
            Style::default().add_modifier(Modifier::BOLD),
        )],
        vec![Span::raw("c      - Clear Plot")],
        vec![Span::raw("x      - Set x-axis stream")],
        vec![Span::raw("y      - Set y-axis stream")],
        vec![Span::raw(
            "k      - Kill Python proces (end the experiment)",
        )],
        vec![Span::raw("p      - pause the currently running experiment")],
        vec![Span::raw(
            "r      - resume the currently running experiment",
        )],
        vec![Span::raw("")],
        vec![Span::styled(
            "System:",
            Style::default().add_modifier(Modifier::BOLD),
        )],
        vec![Span::raw("q  - Quit Experiment / Exit remote viewer")],
    ];

    let text: Vec<Line> = control_text.into_iter().map(Line::from).collect();

    Paragraph::new(text)
        .block(Block::default().title("Controls").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn format_axis(val: f64) -> String {
    let abs_val = val.abs();
    if abs_val == 0.0 {
        "0".to_string()
    } else if !(0.01..1000.0).contains(&abs_val) {
        format!("{val:.2e}")
    } else {
        format!("{val:.2}")
    }
}
