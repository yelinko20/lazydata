use crate::database::fetch::metadata_to_tree_items;
use crate::layout::query_editor::{Mode, Transition};
use crate::layout::{data_table::DataTable, sidebar::SideBar};
use crate::{
    database::{
        connector::{ConnectionDetails, DatabaseType, get_connection_details},
        detector::get_installed_databases,
        fetch::fetch_all_table_metadata,
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
use tui_textarea::Input;
use tui_tree_widget::TreeItem;

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
    pub sidebar: SideBar,
    // pub textarea: TextArea<'static>,
}

impl App {
    pub fn default() -> Self {
        Self {
            focus: Focus::Sidebar,
            query: String::new(),
            exit: false,
            data_table: DataTable::new(vec![], vec![]),
            query_editor: QueryEditor::new(Mode::Normal),
            sidebar: SideBar::new(vec![], Focus::Sidebar),
            // textarea: TextArea::default(),
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
        let metadata = fetch_all_table_metadata(&pool).await?;

        if metadata.is_empty() {
            println!("❌ No tables found in the database.");
            return Ok(());
        }

        println!("✅ Found {} tables", metadata.len());
        let items = metadata_to_tree_items(&metadata);

        self.setup_ui(items);

        stdout().execute(EnableMouseCapture)?;
        let terminal = ratatui::init();
        self.run(terminal)?;
        ratatui::restore();
        stdout().execute(DisableMouseCapture)?;

        Ok(())
    }

    fn setup_ui(&mut self, sidebar_items: Vec<TreeItem<'static, String>>) {
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
                        _ => match self.focus {
                            Focus::Editor => {
                                let input = Input::from(key_event);
                                match self.query_editor.handle_keys(input) {
                                    Transition::Nop => {}
                                    Transition::Mode(mode) => self.query_editor.mode = mode,
                                    Transition::Pending(pending) => {
                                        self.query_editor.pending = pending
                                    }
                                }
                            }
                            Focus::Table => self.handle_data_table_keys(key_event.code),
                            Focus::Sidebar => self.handle_sidebar_keys(key_event.code),
                        },
                    }
                }
            }
        }
        Ok(())
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

    fn handle_sidebar_keys(&mut self, key: KeyCode) {
        use KeyCode::*;

        match key {
            Char('\n' | ' ') => self.sidebar.state.toggle_selected(),
            Left => self.sidebar.state.key_left(),
            Right => self.sidebar.state.key_right(),
            Down => self.sidebar.state.key_down(),
            Up => self.sidebar.state.key_up(),
            Esc => self.sidebar.state.select(Vec::new()),
            Home => self.sidebar.state.select_first(),
            End => self.sidebar.state.select_last(),
            PageDown => self.sidebar.state.scroll_down(3),
            PageUp => self.sidebar.state.scroll_up(3),
            _ => false,
        };
    }

    fn render_ui(&mut self, f: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(f.area());

        self.sidebar.render(f, layout[0]);

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
