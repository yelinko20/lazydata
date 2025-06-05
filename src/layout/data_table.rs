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

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

const ITEM_HEIGHT: usize = 4;

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
}

impl DynamicData {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let column_widths = Self::calculate_column_widths(&headers, &rows);
        Self {
            headers,
            rows,
            column_widths,
        }
    }

    pub fn from_query_results(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        Self::new(headers, rows)
    }

    fn calculate_column_widths(headers: &[String], rows: &[Vec<String>]) -> Vec<u16> {
        let mut widths = vec![0; headers.len()];

        for (i, header) in headers.iter().enumerate() {
            widths[i] = header.width() as u16;
        }

        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let cell_width = cell.width() as u16;
                    if cell_width > widths[i] {
                        widths[i] = cell_width;
                    }
                }
            }
        }

        // Add minimal padding and ensure minimum width
        widths.iter().map(|&w| std::cmp::max(w + 1, 3)).collect()
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
}

pub struct DataTable<'a> {
    state: TableState,
    data: DynamicData,
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
                data.column_widths.len().saturating_add(1),
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

    pub fn update_data(&mut self, headers: Vec<String>, rows: Vec<Vec<String>>) {
        if headers.is_empty() || rows.is_empty() {
            self.tabs.set_index(1);
        }
        self.data = DynamicData::from_query_results(headers, rows);
        self.state
            .select(if self.is_empty() { None } else { Some(0) });
        self.vertical_scroll_state =
            ScrollbarState::new((self.data.len().saturating_sub(1)) * ITEM_HEIGHT);
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

    pub fn build_status_paragraph(&self, title: &'a str, style: &DefaultStyle) -> Paragraph<'a> {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .border_style(style.border_style(Focus::Table)) // Assuming Table focus for status
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
                // "Results" tab
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
                    ) // Example focus
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
            _ => {} // Handle other tabs or default
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

        let numbering_col_width = 4; // Width for "No." column (e.g., "  1 ", " 12 ", etc.)
        let mut visible_columns = 0;
        let mut total_width = numbering_col_width; // Include "No." column in width calculation
        let available_width = area.width.saturating_sub(1); // Account for scrollbar

        // First pass: calculate how many columns can fit
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

        // Second pass: adjust column widths if needed
        let mut adjusted_widths = vec![numbering_col_width]; // Prepend "No." column width
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

        // Create header row, without a header cell for the "No." column
        let header = std::iter::once(Cell::from("")) // Empty header for "No." column
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
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
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
                horizontal: 1,
            }),
            &mut self.vertical_scroll_state,
        );

        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::HorizontalBottom)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_symbol("━━━"),
            area.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
            &mut self.horizontal_scroll_state,
        );
    }
}
