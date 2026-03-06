use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_controls_popup(f: &mut Frame, area: Rect) {
    let popup_area = popup_area(area, 60, 40);
    let controls = create_controls_widget();

    f.render_widget(Clear, popup_area); // Clear background
    f.render_widget(controls, popup_area);
}

fn create_controls_widget() -> impl Widget {
    let control_text = vec![
        vec![Span::styled(
            "Navigation:",
            Style::default().add_modifier(Modifier::BOLD),
        )],
        vec![Span::raw("↑/↓ - Navigate devices")],
        vec![Span::raw("←/→ - Navigate streams")],
        vec![Span::raw("Tab - Switch between Chart and State tabs")],
        vec![Span::raw("f - Cycle focus (State tab only)")],
        vec![Span::raw("")],
        vec![Span::styled(
            "Actions:",
            Style::default().add_modifier(Modifier::BOLD),
        )],
        vec![Span::raw("c - Clear Plot")],
        vec![Span::raw("x - Set x-axis stream")],
        vec![Span::raw("y - Set y-axis stream")],
        vec![Span::raw(
            "e - Toggle edit mode (State tab, when not collecting data)",
        )],
        vec![Span::raw("l - Load config file")],
        vec![Span::raw("n - Start a new run")],
        vec![Span::raw("k - Kill script process (end the experiment)")],
        vec![Span::raw("p - Pause the currently running experiment")],
        vec![Span::raw("r - Resume the currently running experiment")],
        vec![Span::raw("")],
        vec![Span::styled(
            "System:",
            Style::default().add_modifier(Modifier::BOLD),
        )],
        vec![Span::raw("m - Toggle this help menu")],
        vec![Span::raw("q - Quit Experiment / Exit remote viewer")],
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
