use ratatui::style::{Color, Modifier, Style};

use crate::app::Focus;

pub trait StyleProvider {
    fn border_style(&self, current: Focus) -> Style;
    fn block_style(&self) -> Style;
    fn text_style(&self) -> Style;
    fn highlight_style(&self) -> Style;
}

pub struct DefaultStyle {
    pub focus: Focus,
}

impl StyleProvider for DefaultStyle {
    fn border_style(&self, current: Focus) -> Style {
        if self.focus == current {
            Style::default()
                .fg(Color::Rgb(137, 220, 235))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(88, 91, 112))
        }
    }

    fn block_style(&self) -> Style {
        Style::default().bg(Color::Rgb(30, 30, 46))
    }

    fn text_style(&self) -> Style {
        Style::default().fg(Color::Rgb(205, 214, 244))
    }

    fn highlight_style(&self) -> Style {
        Style::default()
            .bg(Color::Rgb(137, 220, 235))
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }
}
