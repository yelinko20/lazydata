use std::{io::stdout, time::Duration};

use crate::layout::{
    data_table::DataTable, query_editor::render_query_editor, sidebar::render_sidebar,
};
use color_eyre::eyre::Result;
use crossterm::{
    ExecutableCommand,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::database::{
    connector::{ConnectionDetails, DatabaseType, get_connection_details},
    detector::get_installed_databases,
    fetch::get_tables,
    pool::pool,
};
use inquire::Select;

#[derive(PartialEq, Debug, Clone)]
pub enum Focus {
    Sidebar,
    Editor,
    Table,
}

pub struct App {
    pub focus: Focus,
    pub query: String,
    pub sidebar_items: Vec<String>,
    pub exit: bool,
    pub data_table: DataTable,
}

impl App {
    pub fn default() -> Self {
        Self {
            focus: Focus::Sidebar,
            query: String::new(),
            sidebar_items: vec![],
            exit: false,
            data_table: DataTable::new(vec![], vec![]),
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
        self.sidebar_items = sidebar_items;
        self.focus = Focus::Sidebar;
        self.query = String::new();
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
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => self.exit = true,
                    KeyCode::Tab => self.toggle_focus(),
                    KeyCode::Down => self.handle_down_key(),
                    KeyCode::Up => self.handle_up_key(),
                    KeyCode::Left => self.handle_left_key(),
                    KeyCode::Right => self.handle_right_key(),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn render_ui(&mut self, f: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(f.area());

        f.render_widget(render_sidebar(self), layout[0]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout[1]);

        f.render_widget(render_query_editor(self), right[0]);
        self.data_table.draw(f, right[1]);
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Sidebar => Focus::Editor,
            Focus::Editor => Focus::Table,
            Focus::Table => Focus::Sidebar,
        };
    }

    fn handle_down_key(&mut self) {
        if self.focus == Focus::Table {
            self.data_table.next_row();
        }
    }

    fn handle_up_key(&mut self) {
        if self.focus == Focus::Table {
            self.data_table.previous_row();
        }
    }

    fn handle_left_key(&mut self) {
        if self.focus == Focus::Table {
            self.data_table.previous_column();
        }
    }

    fn handle_right_key(&mut self) {
        if self.focus == Focus::Table {
            self.data_table.next_column();
        }
    }
}
