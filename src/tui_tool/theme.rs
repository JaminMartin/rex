use ratatui::style::{Modifier, Style};
use ratatui_themes::{ThemeName, ThemePalette};

/// Application theme derived from a `ratatui-themes` palette.
///
/// Wraps [`ThemePalette`] and provides convenience style constructors
/// for every semantic role used throughout the TUI.
#[derive(Clone, Copy, Debug)]
pub struct AppTheme {
    pub palette: ThemePalette,
}

impl AppTheme {
    pub fn new(name: ThemeName) -> Self {
        Self {
            palette: name.palette(),
        }
    }

    pub fn from_config(name: Option<&str>) -> Self {
        let theme_name = name
            .and_then(|s| s.parse::<ThemeName>().ok())
            .unwrap_or_default();
        Self::new(theme_name)
    }

    /// Primary accent – active tabs, focused borders, cursor indicators.
    pub fn accent(&self) -> Style {
        Style::default().fg(self.palette.accent)
    }

    /// Secondary accent – less prominent highlights, trace-level log.
    pub fn secondary(&self) -> Style {
        Style::default().fg(self.palette.secondary)
    }

    /// Default foreground text.
    pub fn fg(&self) -> Style {
        Style::default().fg(self.palette.fg)
    }

    /// Muted / dimmed text – hints, inactive tabs, placeholders.
    pub fn muted(&self) -> Style {
        Style::default().fg(self.palette.muted)
    }

    /// Informational – session fields, chart data line, info-level log.
    pub fn info(&self) -> Style {
        Style::default().fg(self.palette.info)
    }

    /// Success / positive – device names, edit-mode input text, debug log.
    pub fn success(&self) -> Style {
        Style::default().fg(self.palette.success)
    }

    /// Warning – warn-level log.
    pub fn warning(&self) -> Style {
        Style::default().fg(self.palette.warning)
    }

    /// Error / critical – error-level log.
    pub fn error(&self) -> Style {
        Style::default().fg(self.palette.error)
    }

    // ------------------------------------------------------------------
    // Composite styles
    // ------------------------------------------------------------------

    /// List / table highlight (selection background + bold).
    pub fn highlight(&self) -> Style {
        Style::default()
            .bg(self.palette.selection)
            .add_modifier(Modifier::BOLD)
    }

    /// Border style for the *active* (focused) panel.
    pub fn active_border(&self) -> Style {
        Style::default().fg(self.palette.accent)
    }

    /// Border style for an *inactive* panel.
    pub fn inactive_border(&self) -> Style {
        Style::default()
    }

    /// Bold accent – used for active tab titles.
    pub fn accent_bold(&self) -> Style {
        Style::default()
            .fg(self.palette.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }

    /// Bold section headers inside help / control popups.
    pub fn bold(&self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }
}

impl Default for AppTheme {
    fn default() -> Self {
        Self::new(ThemeName::default())
    }
}
