use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthChar;

use crate::{
    app::Focus,
    style::{DefaultStyle, StyleProvider},
};

pub enum InputMode {
    Normal,
    Editing,
}

pub struct QueryEditor {
    input: String,
    character_index: usize,
    quries: Vec<String>,
    input_mode: InputMode,
}

impl QueryEditor {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            character_index: 0,
            quries: Vec::new(),
            input_mode: InputMode::Normal,
        }
    }

    pub fn start_editing(&mut self) {
        self.input_mode = InputMode::Editing;
    }

    pub fn stop_editing(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn is_editing(&self) -> bool {
        matches!(self.input_mode, InputMode::Editing)
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    pub fn submit_query(&mut self) {
        self.quries.push(self.input.clone());
        self.input.clear();
        self.reset_cursor();
    }

    pub fn delete_char(&mut self) {
        if self.character_index != 0 {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before = self.input.chars().take(from_left_to_current_index);
            let after = self.input.chars().skip(current_index);

            self.input = before.chain(after).collect();
            self.move_cursor_left();
        }
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, current_focus: Focus) {
        let style = DefaultStyle {
            focus: current_focus,
        };
        let block = Block::default()
            .title("Query Editor")
            .borders(Borders::ALL)
            .border_style(style.border_style(Focus::Editor))
            .style(style.block_style());

        let text = Text::from(self.input.clone())
            .patch_style(Style::default().fg(style.text_style().fg.unwrap_or(Color::Reset)));
        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);

        let max_width = area.width.saturating_sub(2);
        let input_up_to_cursor = self.input.chars().take(self.character_index);

        let mut x = 0;
        let mut y = 0;
        let mut current_line_width = 0;

        for ch in input_up_to_cursor {
            let ch_width = ch.width().unwrap_or(0) as u16;

            if current_line_width + ch_width > max_width {
                y += 1;
                current_line_width = 0;
            }

            current_line_width += ch_width;
        }

        x += current_line_width;

        match self.input_mode {
            InputMode::Editing => {
                frame.set_cursor_position((area.x + 1 + x, area.y + 1 + y));
            }
            InputMode::Normal => {}
        }
    }
}
