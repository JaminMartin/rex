use crate::data_handler::transport::Transport;
use crate::tui_tool::app::{App, TabView};
use crate::tui_tool::keybindings::handle_key_event;
use crate::tui_tool::tabs::chart;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, Frame};
use std::{
    io,
    time::{Duration, Instant},
};

pub async fn run_tui<T: Transport>(transport: T, remote: bool) -> tokio::io::Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let tick_rate = Duration::from_millis(100);
    let app = App::new(remote, transport);
    let res = run_app(&mut terminal, app, tick_rate, remote);

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

fn run_app<T: Transport>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App<T>,
    tick_rate: Duration,
    remote: bool,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let CrosstermEvent::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if handle_key_event(&mut app, key, remote) {
                        return Ok(());
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

pub fn ui<T: Transport>(f: &mut Frame, app: &mut App<T>) {
    let area = f.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    render_tab_bar(f, app, main_chunks[0]);

    match app.active_tab {
        TabView::Chart => chart::render_chart_tab(f, app, main_chunks[1]),
        TabView::State => app.state_tab.render(f, main_chunks[1], app.show_popup),
    }
}

fn render_tab_bar<T: Transport>(f: &mut Frame, app: &App<T>, area: Rect) {
    let tab_titles = vec!["Data", "Session Info"];

    let tabs: Vec<Line> = tab_titles
        .iter()
        .enumerate()
        .map(|(i, title)| {
            let is_active = match (i, app.active_tab) {
                (0, TabView::Chart) => true,
                (1, TabView::State) => true,
                _ => false,
            };

            let style = if is_active {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            Line::from(vec![
                Span::raw("  "),
                Span::styled(*title, style),
                Span::raw("  "),
            ])
        })
        .collect();

    let tabs_text: Vec<Span> = tabs.into_iter().flat_map(|line| line.spans).collect();

    let paragraph = ratatui::widgets::Paragraph::new(Line::from(tabs_text))
        .block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Tabs (Tab to switch)"),
        )
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}
