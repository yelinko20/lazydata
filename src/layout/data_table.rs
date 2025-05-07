use ratatui::Frame;
use ratatui::layout::{Constraint, Margin, Rect};
use ratatui::style::palette::tailwind;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{
    Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
};
use unicode_width::UnicodeWidthStr;

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

        // Calculate max width for each column based on headers
        for (i, header) in headers.iter().enumerate() {
            widths[i] = header.width() as u16;
        }

        // Calculate max width for each column based on row data
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

        // Add some padding
        widths.iter().map(|&w| w + 2).collect()
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
        self.rows.is_empty()
    }
}

pub struct DataTable {
    state: TableState,
    data: DynamicData,
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
}
#[allow(dead_code)]
impl DataTable {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let data = DynamicData::from_query_results(headers, rows);
        Self {
            state: TableState::default().with_selected(if data.is_empty() {
                None
            } else {
                Some(0)
            }),
            scroll_state: ScrollbarState::new((data.len().saturating_sub(1)) * ITEM_HEIGHT),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            data,
        }
    }

    pub fn update_data(&mut self, headers: Vec<String>, rows: Vec<Vec<String>>) {
        self.data = DynamicData::from_query_results(headers, rows);
        self.state
            .select(if self.data.is_empty() { None } else { Some(0) });
        self.scroll_state = ScrollbarState::new((self.data.len().saturating_sub(1)) * ITEM_HEIGHT);
    }

    pub fn next_row(&mut self) {
        if self.data.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) if i >= self.data.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
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
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
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

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        self.set_colors();

        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        if self.data.is_empty() {
            return;
        }

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

        let header = self
            .data
            .headers()
            .iter()
            .map(|h| Cell::from(h.clone()))
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.data.rows().iter().enumerate().map(|(i, row)| {
            let color = if i % 2 == 0 {
                self.colors.normal_row_color
            } else {
                self.colors.alt_row_color
            };
            Row::new(
                row.iter()
                    .map(|text| Cell::from(Text::from(format!("\n{text}\n")))),
            )
            .style(Style::new().fg(self.colors.row_fg).bg(color))
            .height(ITEM_HEIGHT as u16)
        });

        let bar = " █ ";
        let constraints = self
            .data
            .column_widths()
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
            .highlight_spacing(HighlightSpacing::Always);

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
            &mut self.scroll_state,
        );
    }
}
