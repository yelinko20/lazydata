use crate::{
    app::{App, Focus},
    style::{DefaultStyle, StyleProvider},
};
use ratatui::widgets::{Block, Borders, List, ListItem};

pub fn render_sidebar(app: &App) -> List {
    let sidebar_items: Vec<ListItem> = app
        .sidebar_items
        .iter()
        .map(|label| ListItem::new(label.clone()))
        .collect();

    let style = DefaultStyle {
        focus: app.focus.clone(),
    };

    let sidebar_block = Block::default()
        .title("Tables")
        .borders(Borders::ALL)
        .border_style(style.border_style(Focus::Sidebar))
        .style(style.block_style());

    List::new(sidebar_items)
        .block(sidebar_block)
        .highlight_style(style.highlight_style())
        .highlight_symbol("") // no prefix symbol
        .style(style.text_style())
}
