use ratatui::widgets::{Block, Borders, Paragraph};

use crate::{
    app::{App, Focus},
    style::{DefaultStyle, StyleProvider},
};

pub fn render_query_editor(app: &App) -> Paragraph {
    let style = DefaultStyle {
        focus: app.focus.clone(),
    };

    Paragraph::new(app.query.clone())
        .block(
            Block::default()
                .title("Query Editor (Press Tab to switch)")
                .borders(Borders::ALL)
                .border_style(style.border_style(Focus::Editor))
                .style(style.block_style()),
        )
        .style(style.text_style())
}
