use crate::tui_tool::theme::AppTheme;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render_controls_popup(f: &mut Frame, area: Rect, theme: &AppTheme) {
    let popup_area = popup_area(area, 60, 50);
    let controls = create_controls_widget(theme);

    f.render_widget(Clear, popup_area);
    f.render_widget(controls, popup_area);
}

fn create_controls_widget(theme: &AppTheme) -> impl Widget {
    let bold = theme.bold();

    let text = vec![
        Line::from(Span::styled(
            "Chart Tab Controls",
            theme.accent().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Navigation:", bold)),
        Line::from("↑/↓ - Navigate devices"),
        Line::from("←/→ - Navigate streams"),
        Line::from("Tab - Switch between Chart and State tabs"),
        Line::from("f - Cycle focus (State tab only)"),
        Line::from(""),
        Line::from(Span::styled("Actions:", bold)),
        Line::from("c - Clear Plot"),
        Line::from("x - Set x-axis stream"),
        Line::from("y - Set y-axis stream"),
        Line::from("e - Toggle edit mode (State tab, when not collecting data)"),
        Line::from("l - Load config file"),
        Line::from("n - Start a new run"),
        Line::from("k - Kill script process (end the experiment)"),
        Line::from("p - Pause the currently running experiment"),
        Line::from("r - Resume the currently running experiment"),
        Line::from(""),
        Line::from(Span::styled("System:", bold)),
        Line::from("m - Toggle this help menu"),
        Line::from("q - Quit Experiment / Exit remote viewer"),
    ];

    Paragraph::new(text)
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(theme.fg()),
        )
        .wrap(Wrap { trim: false })
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
