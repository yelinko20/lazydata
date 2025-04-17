use crate::layout::{data_table::DataTable, sidebar::Sidebar};
use crate::{
    database::{
        connector::{ConnectionDetails, DatabaseType, get_connection_details},
        detector::get_installed_databases,
        fetch::get_tables,
        pool::pool,
    },
    layout::query_editor::QueryEditor,
};
use color_eyre::eyre::Result;
use crossterm::{
    ExecutableCommand,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
};
use inquire::Select;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
};
use std::{io::stdout, time::Duration};

#[derive(PartialEq, Debug, Clone)]
pub enum Focus {
    Sidebar,
    Editor,
    Table,
}

impl Focus {
    fn next(self) -> Self {
        match self {
            Focus::Sidebar => Focus::Editor,
            Focus::Editor => Focus::Table,
            Focus::Table => Focus::Sidebar,
        }
    }
}

pub struct App {
    pub focus: Focus,
    pub query: String,
    pub exit: bool,
    pub data_table: DataTable,
    pub query_editor: QueryEditor,
    pub sidebar: Sidebar,
}

impl App {
    pub fn default() -> Self {
        Self {
            focus: Focus::Sidebar,
            query: String::new(),
            exit: false,
            data_table: DataTable::new(vec![], vec![]),
            query_editor: QueryEditor::new(),
            sidebar: Sidebar::new(vec![], Focus::Sidebar),
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        let databases = get_installed_databases()?;

        if databases.is_empty() {
            println!("❌ No databases detected!");
            return Ok(());
        }

        let selected = Select::new("🚀 Select a Database", databases.clone())
            .with_help_message("Use ↑ ↓ arrows, Enter to select")
            .prompt();

        if let Ok(db_name) = selected {
            if let Some(db_type) = Self::map_db_name_to_type(&db_name) {
                self.setup_and_run_app(db_type).await?;
            } else {
                println!("❌ Unsupported database.");
            }
        } else {
            println!("\n👋 Exited without selection.");
        }

        Ok(())
    }

    fn map_db_name_to_type(name: &str) -> Option<DatabaseType> {
        match name.to_lowercase().as_str() {
            "postgresql" => Some(DatabaseType::PostgreSQL),
            "mysql" => Some(DatabaseType::MySQL),
            "sqlite" => Some(DatabaseType::SQLite),
            _ => None,
        }
    }

    async fn setup_and_run_app(&mut self, db_type: DatabaseType) -> Result<()> {
        let details: ConnectionDetails = get_connection_details(db_type)?;
        let pool = pool(db_type, &details).await?;
        let tables = get_tables(pool).await?;

        if tables.is_empty() {
            println!("❌ No tables found in the database.");
            return Ok(());
        }

        println!("✅ Found {} tables", tables.len());

        self.setup_ui(tables.table_names());
        stdout().execute(EnableMouseCapture)?;
        let terminal = ratatui::init();
        self.run(terminal)?;
        ratatui::restore();
        stdout().execute(DisableMouseCapture)?;

        Ok(())
    }

    fn setup_ui(&mut self, sidebar_items: Vec<String>) {
        self.focus = Focus::Sidebar;
        self.query = String::new();
        self.sidebar.update_items(sidebar_items);
        self.sidebar.update_focus(Focus::Sidebar);

        self.data_table = DataTable::new(
            vec!["ID".into(), "Name".into(), "Value".into()],
            vec![
                vec!["1".into(), "Item A".into(), "100".into()],
                vec!["2".into(), "Item B".into(), "200".into()],
            ],
        );
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|f| self.render_ui(f))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    match key_event.code {
                        KeyCode::Char('q') => {
                            self.exit = true;
                        }
                        KeyCode::Tab => {
                            self.toggle_focus();
                        }
                        key => match self.focus {
                            Focus::Editor => self.handle_query_editor_keys(key),
                            Focus::Table => self.handle_data_table_keys(key),
                            _ => {}
                        },
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_query_editor_keys(&mut self, key: KeyCode) {
        use KeyCode::*;
        match key {
            Char(c) => self.query_editor.enter_char(c),
            Backspace => self.query_editor.delete_char(),
            Left => self.query_editor.move_cursor_left(),
            Right => self.query_editor.move_cursor_right(),
            _ => {}
        }
    }

    fn handle_data_table_keys(&mut self, key: KeyCode) {
        use KeyCode::*;
        match key {
            Char('j') | Down => self.data_table.next_row(),
            Char('k') | Up => self.data_table.previous_row(),
            Char('l') | Right => self.data_table.next_column(),
            Char('h') | Left => self.data_table.previous_column(),
            _ => {}
        }
    }

    fn render_ui(&mut self, f: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(f.area());

        f.render_widget(self.sidebar.render(), layout[0]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout[1]);

        self.query_editor.draw(f, right[0], self.focus.clone());
        self.data_table.draw(f, right[1]);
    }

    fn toggle_focus(&mut self) {
        self.focus = self.focus.clone().next();
        self.sidebar.update_focus(self.focus.clone());
    }
}
