use std::collections::HashMap;
use std::time::Duration;

use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::palette::tailwind;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, TableState, Tabs,
};
use ratatui::{Frame, symbols};
use unicode_width::UnicodeWidthStr;

use crate::app::Focus;
use crate::components::tabs::StatefulTabs;
use crate::style::theme::COLOR_BLOCK_BG;
use crate::style::{DefaultStyle, StyleProvider};
use arboard::Clipboard;
use serde_json::Value;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

const ITEM_HEIGHT: usize = 3;

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DynamicData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub column_widths: Vec<u16>,
    pub min_column_widths: Vec<u16>,
}

impl DynamicData {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let column_widths = Self::calculate_column_widths(&headers, &rows);
        let min_column_widths = column_widths.clone();
        Self {
            headers,
            rows,
            column_widths,
            min_column_widths,
        }
    }

    pub fn from_query_results(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        Self::new(headers, rows)
    }

    fn calculate_column_widths(headers: &[String], rows: &[Vec<String>]) -> Vec<u16> {
        let mut widths: Vec<u16> = headers.iter().map(|h| h.width() as u16).collect();

        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.width() as u16);
                }
            }
        }

        widths.iter().map(|&w| w.saturating_add(2).max(3)).collect()
    }

    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    pub fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }

    pub fn column_widths(&self) -> &[u16] {
        &self.column_widths
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() || self.headers.is_empty()
    }

    pub fn adjust_column_width(&mut self, column: usize, delta: i16) {
        if column < self.column_widths.len() {
            let min_width = self.min_column_widths[column];
            let new_width = self.column_widths[column] as i16 + delta;
            self.column_widths[column] = new_width.max(min_width as i16) as u16;
        }
    }
}

pub struct DataTable<'a> {
    state: TableState,
    pub data: DynamicData,
    vertical_scroll_state: ScrollbarState,
    horizontal_scroll_state: ScrollbarState,
    horizontal_scroll: usize,
    colors: TableColors,
    color_index: usize,
    pub tabs: StatefulTabs<'a>,
    pub status_message: Option<String>,
    pub elapsed: Duration,
}

impl<'a> DataTable<'a> {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let data = DynamicData::from_query_results(headers, rows);
        let mut tabs = StatefulTabs::new(vec!["Data Output", "Messages", "Query History"]);
        if data.is_empty() {
            tabs.set_index(1);
        }
        Self {
            state: TableState::default().with_selected(if data.is_empty() {
                None
            } else {
                Some(0)
            }),
            vertical_scroll_state: ScrollbarState::new(
                (data.len().saturating_sub(1)) * ITEM_HEIGHT,
            ),
            horizontal_scroll_state: ScrollbarState::new(
                data.column_widths.iter().sum::<u16>().saturating_sub(1) as usize,
            ),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            data,
            horizontal_scroll: 0,
            tabs,
            status_message: None,
            elapsed: Duration::ZERO,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn next_row(&mut self) {
        if self.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) if i >= self.data.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.state.select(Some(i));
        self.vertical_scroll_state = self.vertical_scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous_row(&mut self) {
        if self.data.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(0) => self.data.len() - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.state.select(Some(i));
        self.vertical_scroll_state = self.vertical_scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub fn scroll_right(&mut self) {
        if self.horizontal_scroll < self.data.column_widths.len().saturating_sub(1) {
            self.horizontal_scroll = self.horizontal_scroll.saturating_add(1);
            self.horizontal_scroll_state = self
                .horizontal_scroll_state
                .position(self.horizontal_scroll);
        }
    }

    pub fn scroll_left(&mut self) {
        if self.horizontal_scroll > 0 {
            self.horizontal_scroll = self.horizontal_scroll.saturating_sub(1);
            self.horizontal_scroll_state = self
                .horizontal_scroll_state
                .position(self.horizontal_scroll);
        }
    }

    pub fn next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }

    pub fn jump_to_row(&mut self, row: usize) {
        if row < self.data.len() {
            self.state.select(Some(row));
            self.vertical_scroll_state = self.vertical_scroll_state.position(row * ITEM_HEIGHT);
        }
    }

    #[allow(dead_code)]
    pub fn jump_to_column(&mut self, col: usize) {
        if col < self.data.headers().len() {
            self.horizontal_scroll = col;
            self.horizontal_scroll_state = self.horizontal_scroll_state.position(col);
        }
    }

    #[allow(dead_code)]
    pub fn search_in_table(&mut self, query: &str) -> Option<(usize, usize)> {
        for (row_idx, row) in self.data.rows().iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                if cell.to_lowercase().contains(&query.to_lowercase()) {
                    return Some((row_idx, col_idx));
                }
            }
        }
        None
    }

    pub fn copy_selected_cell(&self) -> Option<String> {
        let content = match (self.state.selected(), self.state.selected_column()) {
            (Some(row_idx), Some(col_idx)) => {
                let adjusted_col = col_idx.saturating_sub(1) + self.horizontal_scroll;
                let row = self.data.rows().get(row_idx)?;

                if col_idx == 0 {
                    (row_idx + 1).to_string()
                } else if adjusted_col < row.len() {
                    row[adjusted_col].clone()
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(&content);
        }

        Some(content)
    }

    pub fn copy_selected_row(&self) -> Option<String> {
        let selected_row_index = self.state.selected()?;

        let headers = self.data.headers();
        let row_data = self.data.rows().get(selected_row_index)?;

        if headers.len() != row_data.len() {
            eprintln!(
                "Error: Headers count ({}) does not match row data count ({}) for selected row index {}. Cannot form proper JSON.",
                headers.len(),
                row_data.len(),
                selected_row_index
            );
            return None;
        }

        let row_as_json_object: HashMap<String, Value> = headers
            .iter()
            .zip(row_data.iter()) // Pair headers with cell values
            .map(|(header, cell_value)| {
                let json_value = if cell_value.eq_ignore_ascii_case("null")
                    || cell_value.eq_ignore_ascii_case("[null]")
                {
                    Value::Null
                } else {
                    Value::String(cell_value.clone())
                };
                (header.clone(), json_value)
            })
            .collect();

        let json_string = serde_json::to_string_pretty(&row_as_json_object)
            .map_err(|e| eprintln!("Error: Failed to serialize row data to JSON: {}", e))
            .ok()?;

        if let Ok(mut clipboard) = Clipboard::new() {
            if let Err(e) = clipboard.set_text(&json_string) {
                eprintln!("Warning: Could not set clipboard text: {}", e);
            }
        } else {
            eprintln!("Warning: Could not access clipboard.");
        }

        Some(json_string)
    }

    pub fn adjust_column_width(&mut self, delta: i16) {
        if let Some(col) = self.state.selected_column() {
            self.data.adjust_column_width(col, delta);
        }
    }

    pub fn build_status_paragraph(&self, title: &'a str, style: &DefaultStyle) -> Paragraph<'a> {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .border_style(style.border_style(Focus::Table))
            .style(style.block_style());

        Paragraph::new(title).block(title_block)
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, current_focus: &Focus) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let tab_area = main_layout[0];
        let content_area = main_layout[1];
        let query_info_area = main_layout[2];

        let base_style = Style::default().bg(COLOR_BLOCK_BG);
        let total_rows_str = format!("Total Rows: {}", self.data.len());
        let query_done_str = format!("Query Complete: {} ms", self.elapsed.as_millis());

        let tab_lines = [total_rows_str, query_done_str]
            .iter()
            .map(|text| Line::from(Span::styled(text.clone(), base_style)))
            .collect::<Vec<_>>();

        let query_info_tabs = Tabs::new(tab_lines)
            .select(0)
            .highlight_style(base_style)
            .divider(symbols::line::VERTICAL)
            .style(
                DefaultStyle {
                    focus: current_focus.clone(),
                }
                .block_style(),
            );
        frame.render_widget(query_info_tabs, query_info_area);

        let tabs_widget = self.tabs.widget().block(
            Block::default().border_style(
                DefaultStyle {
                    focus: current_focus.clone(),
                }
                .border_style(Focus::Table),
            ),
        );
        frame.render_widget(tabs_widget, tab_area);

        match self.tabs.index {
            0 => {
                self.set_colors();
                if self.is_empty() {
                    let message = "No data output. Execute a query to get output";
                    let status_widget = self.build_status_paragraph(
                        message,
                        &DefaultStyle {
                            focus: current_focus.clone(),
                        },
                    );
                    frame.render_widget(status_widget, content_area);
                } else {
                    self.render_table(frame, content_area, current_focus);
                    self.render_scrollbar(frame, content_area);
                }
            }
            1 => {
                let messages_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        DefaultStyle {
                            focus: current_focus.clone(),
                        }
                        .border_style(Focus::Table),
                    )
                    .style(
                        DefaultStyle {
                            focus: current_focus.clone(),
                        }
                        .block_style(),
                    );
                let message = self.status_message.clone().unwrap_or("".to_string());
                let messages_paragraph = Paragraph::new(message).block(messages_block);
                frame.render_widget(messages_paragraph, content_area);
            }
            2 => {
                let history_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        DefaultStyle {
                            focus: current_focus.clone(),
                        }
                        .border_style(Focus::Table),
                    )
                    .style(
                        DefaultStyle {
                            focus: current_focus.clone(),
                        }
                        .block_style(),
                    );
                let history_paragraph = Paragraph::new("This is where query history would appear.")
                    .block(history_block);
                frame.render_widget(history_paragraph, content_area);
            }
            _ => {}
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect, current_focus: &Focus) {
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let numbering_col_width = 4;
        let mut visible_columns = 0;
        let mut total_width = numbering_col_width;
        let available_width = area.width.saturating_sub(1);

        for width in self
            .data
            .column_widths()
            .iter()
            .skip(self.horizontal_scroll)
        {
            if total_width + width > available_width {
                break;
            }
            total_width += width;
            visible_columns += 1;
        }

        let mut adjusted_widths = vec![numbering_col_width];
        let mut remaining_width = available_width.saturating_sub(numbering_col_width);
        let columns_to_show = self
            .data
            .column_widths()
            .iter()
            .skip(self.horizontal_scroll)
            .take(visible_columns);

        for &width in columns_to_show {
            if remaining_width >= width {
                adjusted_widths.push(width);
                remaining_width -= width;
            } else {
                adjusted_widths.push(remaining_width);
                break;
            }
        }

        let visible_headers: Vec<_> = self
            .data
            .headers()
            .iter()
            .skip(self.horizontal_scroll)
            .take(visible_columns)
            .cloned()
            .collect();

        let header = std::iter::once(Cell::from("#"))
            .chain(visible_headers.iter().map(|h| Cell::from(h.clone())))
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.data.rows().iter().enumerate().map(|(i, row)| {
            let color = if i % 2 == 0 {
                self.colors.normal_row_color
            } else {
                self.colors.alt_row_color
            };

            let number_cell = Cell::from(Text::from(format!("\n{}\n", i + 1)));
            let data_cells = row
                .iter()
                .skip(self.horizontal_scroll)
                .take(visible_columns)
                .map(|text| Cell::from(Text::from(format!("\n{text}\n"))));

            Row::new(std::iter::once(number_cell).chain(data_cells))
                .style(Style::new().fg(self.colors.row_fg).bg(color))
                .height(ITEM_HEIGHT as u16)
        });

        let bar = " █ ";
        let constraints = adjusted_widths
            .iter()
            .map(|&w| Constraint::Length(w))
            .collect::<Vec<_>>();

        let t = Table::new(rows, constraints)
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(vec!["".into(), bar.into(), "".into()])
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        DefaultStyle {
                            focus: current_focus.clone(),
                        }
                        .border_style(Focus::Table),
                    )
                    .style(
                        DefaultStyle {
                            focus: current_focus.clone(),
                        }
                        .block_style(),
                    ),
            );

        frame.render_stateful_widget(t, area, &mut self.state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        if self.data.is_empty() {
            return;
        }

        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.vertical_scroll_state,
        );

        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::HorizontalBottom)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_symbol(symbols::line::THICK_HORIZONTAL),
            area.inner(Margin {
                horizontal: 1,
                vertical: 0,
            }),
            &mut self.horizontal_scroll_state,
        );
    }
}
