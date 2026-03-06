use crate::data_handler::transport::Transport;
use crate::tui_tool::action::Action;
use crate::tui_tool::app::{App, TabView};
use crate::tui_tool::event::{Event, EventHandler};
use crate::tui_tool::keybindings::handle_key_event;
use crate::tui_tool::tabs::chart;
use crate::tui_tool::update::update;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, Frame};
use std::io;

pub async fn run_tui<T: Transport + Clone + Send + 'static>(
    transport: T,
    remote: bool,
) -> tokio::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut event_handler = EventHandler::new(100); // 100ms tick rate
    let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut app = App::new(remote, transport, action_tx.clone());

    let res = run_app(&mut terminal, &mut app, &mut event_handler, &mut action_rx).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

async fn run_app<T: Transport + Clone + Send + 'static>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App<T>,
    event_handler: &mut EventHandler,
    action_rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        tokio::select! {
            event_result = event_handler.next() => {
                match event_result {
                    Ok(event) => {
                        let actions = match event {
                            Event::Tick => vec![Action::Tick],
                            Event::Key(key) => handle_key_event(app, key),
                            Event::Error => vec![],
                        };

                        for action in actions {
                            let _ = app.action_tx.send(action);
                        }
                    }
                    Err(e) => {
                        log::error!("Event handler error: {}", e);
                    }
                }
            }
            Some(action) = action_rx.recv() => {
                update(app, action);
                if app.should_quit {
                    return Ok(());
                }
            }
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

    let theme = app.theme;
    match app.active_tab {
        TabView::Chart => chart::render_chart_tab(f, app, main_chunks[1]),
        TabView::State => app
            .state_tab
            .render(f, main_chunks[1], app.show_popup, &theme),
    }
}

fn render_tab_bar<T: Transport>(f: &mut Frame, app: &App<T>, area: Rect) {
    let theme = &app.theme;
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
                theme.accent_bold()
            } else {
                theme.muted()
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
