use color_eyre::eyre::Result;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};
use std::fmt;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use tui_textarea::{CursorMove, Input, Key, Scrolling, TextArea};

use crate::app::Focus;
use crate::style::{DefaultStyle, StyleProvider};
use crate::utils::highlighter::highlight_sql;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Operator(char),
}

impl Mode {
    fn block<'a>(&self, current_focus: &Focus) -> Block<'a> {
        let style = DefaultStyle {
            focus: current_focus.clone(),
        };
        let help = match self {
            Self::Normal => "type i to enter insert mode",
            Self::Insert => "type Esc to back to normal mode",
            Self::Visual => "type y to yank, type d to delete, type Esc to back to normal mode",
            Self::Operator(_) => "move cursor to apply operator",
        };
        let title = format!("{} MODE ({})", self, help);
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(style.border_style(Focus::Editor))
            .style(style.block_style())
    }

    fn cursor_style(&self) -> Style {
        let color = match self {
            Self::Normal => Color::Reset,
            Self::Insert => Color::LightBlue,
            Self::Visual => Color::LightYellow,
            Self::Operator(_) => Color::LightGreen,
        };
        Style::default().fg(color).add_modifier(Modifier::REVERSED)
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Visual => write!(f, "VISUAL"),
            Self::Operator(c) => write!(f, "OPERATOR({})", c),
        }
    }
}

pub enum Transition {
    Nop,
    Mode(Mode),
    Pending(Input),
}

pub struct QueryEditor {
    pub mode: Mode,
    pub pending: Input,
    pub textarea: TextArea<'static>,
}

impl QueryEditor {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            pending: Input::default(),
            textarea: TextArea::default(),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, current_focus: Focus) {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let theme = &ts.themes["base16-ocean.dark"];

        let text = self.textarea.lines().join("\n");
        let cursor = self.textarea.cursor();
        self.textarea.set_cursor_style(self.mode.cursor_style());
        let highlighted_lines = highlight_sql(
            &text,
            &ps,
            theme,
            cursor.0,
            cursor.1,
            self.mode.cursor_style(),
        );

        let block = self.mode.block(&current_focus);

        let paragraph = Paragraph::new(Text::from(highlighted_lines))
            .block(block)
            .style(
                DefaultStyle {
                    focus: current_focus.clone(),
                }
                .block_style(),
            );

        frame.render_widget(paragraph, area);
        let cursor_x = area.x + cursor.1 as u16 + 1;
        let cursor_y = area.y + cursor.0 as u16 + 1;

        if cursor_y < area.y + area.height && cursor_x < area.x + area.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    pub fn handle_keys(&mut self, input: Input) -> Transition {
        if input.key == Key::Null {
            return Transition::Nop;
        }

        match self.mode {
            Mode::Normal | Mode::Visual | Mode::Operator(_) => {
                match input {
                    Input {
                        key: Key::Char('h'),
                        ..
                    } => self.textarea.move_cursor(CursorMove::Back),
                    Input {
                        key: Key::Char('j'),
                        ..
                    } => self.textarea.move_cursor(CursorMove::Down),
                    Input {
                        key: Key::Char('k'),
                        ..
                    } => self.textarea.move_cursor(CursorMove::Up),
                    Input {
                        key: Key::Char('l'),
                        ..
                    } => self.textarea.move_cursor(CursorMove::Forward),
                    Input {
                        key: Key::Char('w'),
                        ..
                    } => self.textarea.move_cursor(CursorMove::WordForward),
                    Input {
                        key: Key::Char('e'),
                        ctrl: false,
                        ..
                    } => {
                        self.textarea.move_cursor(CursorMove::WordEnd);
                        if matches!(self.mode, Mode::Operator(_)) {
                            self.textarea.move_cursor(CursorMove::Forward);
                        }
                    }
                    Input {
                        key: Key::Char('b'),
                        ctrl: false,
                        ..
                    } => self.textarea.move_cursor(CursorMove::WordBack),
                    Input {
                        key: Key::Char('^'),
                        ..
                    } => self.textarea.move_cursor(CursorMove::Head),
                    Input {
                        key: Key::Char('$'),
                        ..
                    } => self.textarea.move_cursor(CursorMove::End),
                    Input {
                        key: Key::Char('D'),
                        ..
                    } => {
                        self.textarea.delete_line_by_end();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('C'),
                        ..
                    } => {
                        self.textarea.delete_line_by_end();
                        self.textarea.cancel_selection();
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('p'),
                        ..
                    } => {
                        self.textarea.paste();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('u'),
                        ctrl: false,
                        ..
                    } => {
                        self.textarea.undo();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('r'),
                        ctrl: true,
                        ..
                    } => {
                        self.textarea.redo();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('x'),
                        ..
                    } => {
                        self.textarea.delete_next_char();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('i'),
                        ..
                    } => {
                        self.textarea.cancel_selection();
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('a'),
                        ..
                    } => {
                        self.textarea.cancel_selection();
                        self.textarea.move_cursor(CursorMove::Forward);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('A'),
                        ..
                    } => {
                        self.textarea.cancel_selection();
                        self.textarea.move_cursor(CursorMove::End);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('o'),
                        ..
                    } => {
                        self.textarea.move_cursor(CursorMove::End);
                        self.textarea.insert_newline();
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('O'),
                        ..
                    } => {
                        self.textarea.move_cursor(CursorMove::Head);
                        self.textarea.insert_newline();
                        self.textarea.move_cursor(CursorMove::Up);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('I'),
                        ..
                    } => {
                        self.textarea.cancel_selection();
                        self.textarea.move_cursor(CursorMove::Head);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('e'),
                        ctrl: true,
                        ..
                    } => self.textarea.scroll((1, 0)),
                    Input {
                        key: Key::Char('y'),
                        ctrl: true,
                        ..
                    } => self.textarea.scroll((-1, 0)),
                    Input {
                        key: Key::Char('d'),
                        ctrl: true,
                        ..
                    } => self.textarea.scroll(Scrolling::HalfPageDown),
                    Input {
                        key: Key::Char('u'),
                        ctrl: true,
                        ..
                    } => self.textarea.scroll(Scrolling::HalfPageUp),
                    Input {
                        key: Key::Char('f'),
                        ctrl: true,
                        ..
                    } => self.textarea.scroll(Scrolling::PageDown),
                    Input {
                        key: Key::Char('b'),
                        ctrl: true,
                        ..
                    } => self.textarea.scroll(Scrolling::PageUp),
                    Input {
                        key: Key::Char('v'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        self.textarea.start_selection();
                        return Transition::Mode(Mode::Visual);
                    }
                    Input {
                        key: Key::Char('V'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        self.textarea.move_cursor(CursorMove::Head);
                        self.textarea.start_selection();
                        self.textarea.move_cursor(CursorMove::End);
                        return Transition::Mode(Mode::Visual);
                    }
                    Input { key: Key::Esc, .. }
                    | Input {
                        key: Key::Char('v'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        self.textarea.cancel_selection();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('g'),
                        ctrl: false,
                        ..
                    } if matches!(
                        self.pending,
                        Input {
                            key: Key::Char('g'),
                            ctrl: false,
                            ..
                        }
                    ) =>
                    {
                        self.textarea.move_cursor(CursorMove::Top)
                    }
                    Input {
                        key: Key::Char('G'),
                        ctrl: false,
                        ..
                    } => self.textarea.move_cursor(CursorMove::Bottom),
                    Input {
                        key: Key::Char(c),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Operator(c) => {
                        // Handle yy, dd, cc. (This is not strictly the same behavior as Vim)
                        self.textarea.move_cursor(CursorMove::Head);
                        self.textarea.start_selection();
                        let cursor = self.textarea.cursor();
                        self.textarea.move_cursor(CursorMove::Down);
                        if cursor == self.textarea.cursor() {
                            self.textarea.move_cursor(CursorMove::End); // At the last line, move to end of the line instead
                        }
                    }
                    Input {
                        key: Key::Char(op @ ('y' | 'd' | 'c')),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        self.textarea.start_selection();
                        return Transition::Mode(Mode::Operator(op));
                    }
                    Input {
                        key: Key::Char('y'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        self.textarea.move_cursor(CursorMove::Forward); // Vim's text selection is inclusive
                        self.textarea.copy();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('d'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        self.textarea.move_cursor(CursorMove::Forward); // Vim's text selection is inclusive
                        self.textarea.cut();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('c'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        self.textarea.move_cursor(CursorMove::Forward); // Vim's text selection is inclusive
                        self.textarea.cut();
                        return Transition::Mode(Mode::Insert);
                    }
                    input => return Transition::Pending(input),
                }

                // Handle the pending operator
                match self.mode {
                    Mode::Operator('y') => {
                        self.textarea.copy();
                        Transition::Mode(Mode::Normal)
                    }
                    Mode::Operator('d') => {
                        self.textarea.cut();
                        Transition::Mode(Mode::Normal)
                    }
                    Mode::Operator('c') => {
                        self.textarea.cut();
                        Transition::Mode(Mode::Insert)
                    }
                    _ => Transition::Nop,
                }
            }
            Mode::Insert => match input {
                Input { key: Key::Esc, .. }
                | Input {
                    key: Key::Char('c'),
                    ctrl: true,
                    ..
                } => Transition::Mode(Mode::Normal),
                input => {
                    self.textarea.input(input);
                    Transition::Mode(Mode::Insert)
                }
            },
        }
    }
}
