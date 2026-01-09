use crate::data_handler::transport::Transport;
use crate::tui_tool::app::App;
use crate::tui_tool::widgets::popup;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    prelude::*,
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem},
    Frame,
};
use tui_logger::*;

pub struct ChartTab {}

impl ChartTab {
    pub fn new() -> Self {
        ChartTab {}
    }
}

pub fn render_chart_tab<T: Transport>(f: &mut Frame, app: &mut App<T>, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let lists_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    render_chart(f, app, chunks[0]);
    render_device_list(f, app, lists_chunk[0]);
    render_stream_list(f, app, lists_chunk[1]);
    render_log(f, chunks[2]);

    if app.show_popup {
        popup::render_controls_popup(f, area);
    }
}

fn render_chart<T: Transport>(f: &mut Frame, app: &App<T>, area: Rect) {
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

            f.render_widget(chart, area);
        }
    } else {
        let block = Block::default()
            .title("Select X and Y axes to view data")
            .borders(Borders::ALL);
        f.render_widget(block, area);
    }
}

fn render_device_list<T: Transport>(f: &mut Frame, app: &mut App<T>, area: Rect) {
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
                _ => " ",
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

    f.render_stateful_widget(devices_list, area, &mut app.devices_state);
}

fn render_stream_list<T: Transport>(f: &mut Frame, app: &mut App<T>, area: Rect) {
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
                    _ => " ",
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

        f.render_stateful_widget(streams_list, area, &mut app.streams_state);
    }
}

fn render_log(f: &mut Frame, area: Rect) {
    let tui_logger = TuiLoggerWidget::default()
        .style_error(Style::default().fg(Color::Red))
        .style_debug(Style::default().fg(Color::Green))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_trace(Style::default().fg(Color::Magenta))
        .style_info(Style::default().fg(Color::Cyan))
        .block(Block::default().title("System Log").borders(Borders::ALL));

    f.render_widget(tui_logger, area);
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
