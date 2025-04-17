use crate::{
    app::Focus,
    style::{DefaultStyle, StyleProvider},
};
use ratatui::widgets::{Block, Borders, List, ListItem};

pub struct Sidebar {
    items: Vec<String>,
    focus: Focus,
}

impl Sidebar {
    pub fn new(items: Vec<String>, focus: Focus) -> Self {
        Self { items, focus }
    }

    pub fn update_focus(&mut self, new_focus: Focus) {
        self.focus = new_focus;
    }

    pub fn update_items(&mut self, new_items: Vec<String>) {
        self.items = new_items;
    }

    pub fn render(&self) -> List {
        let style = DefaultStyle {
            focus: self.focus.clone(),
        };

        let sidebar_items: Vec<ListItem> = self
            .items
            .iter()
            .map(|label| ListItem::new(label.clone()))
            .collect();

        let sidebar_block = Block::default()
            .title("Tables")
            .borders(Borders::ALL)
            .border_style(style.border_style(Focus::Sidebar))
            .style(style.block_style());

        List::new(sidebar_items)
            .block(sidebar_block)
            .highlight_style(style.highlight_style())
            .highlight_symbol("")
            .style(style.text_style())
    }
}
