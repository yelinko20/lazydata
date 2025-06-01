use ratatui::{
    style::{Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::Tabs,
};

use crate::style::theme::{COLOR_BLACK, COLOR_FOCUS};

// --- Reusable StatefulTabs Component ---
/// A component to manage and render tabs.
pub struct StatefulTabs<'a> {
    /// Titles of the tabs.
    pub titles: Vec<&'a str>,
    /// The index of the currently selected tab.
    pub index: usize,
}

impl<'a> StatefulTabs<'a> {
    /// Creates a new `StatefulTabs` component with the given titles.
    /// The first tab is selected by default.
    pub fn new(titles: Vec<&'a str>) -> Self {
        StatefulTabs { titles, index: 0 }
    }

    /// Creates a new `StatefulTabs` component with an initial selected index.
    #[allow(dead_code)] // Example: could be used if needed
    pub fn with_initial_index(titles: Vec<&'a str>, initial_index: usize) -> Self {
        let max_index = titles.len().saturating_sub(1);
        let index = initial_index.min(max_index);
        StatefulTabs { titles, index }
    }

    /// Selects the next tab, cycling around if at the end.
    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }

    /// Selects the previous tab, cycling around if at the beginning.
    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
    }

    /// Sets the selected tab by index.
    /// If the index is out of bounds, it does nothing.
    pub fn set_index(&mut self, index: usize) {
        if index < self.titles.len() {
            self.index = index;
        }
    }

    /// Returns a `Tabs` widget configured for rendering.
    /// This method prepares the visual representation of the tabs.
    /// Note: This widget does not include a surrounding Block by default.
    /// The caller can choose to wrap it in a Block if needed.
    pub fn widget(&self) -> Tabs<'a> {
        // Map titles to Line Spans with a base style
        let titles_as_lines: Vec<Line> = self
            .titles
            .iter()
            .map(|t| Line::from(Span::styled(*t, Style::default())))
            .collect();

        Tabs::new(titles_as_lines)
            .select(self.index)
            .highlight_style(
                Style::default()
                    .fg(COLOR_FOCUS)
                    .bg(COLOR_BLACK)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .divider(symbols::line::VERTICAL)
    }
}
