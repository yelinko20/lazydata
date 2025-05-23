use crate::crud::executor::{ExecutionResult, execute_query};
use crate::database::fetch::{fetch_query, metadata_to_tree_items};
use crate::database::pool::DbPool;
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
use crossterm::execute;
use crossterm::{
    ExecutableCommand, cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    style::Print,
    terminal::{Clear, ClearType},
};
use inquire::Select;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
};
use std::io::Write;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::{io::stdout, time::Duration};
use tokio::task::JoinHandle;
use tokio::time::sleep;
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
    pub pool: Option<DbPool>,
    pub status_message: Option<String>,
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
            pool: None,
            status_message: None,
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
            println!("\n👋 Bye");
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

    fn current_query(&self) -> String {
        self.query_editor.textarea.lines().join("\n")
    }

    async fn setup_and_run_app(&mut self, db_type: DatabaseType) -> Result<()> {
        let details: ConnectionDetails = get_connection_details(db_type)?;
        let pool = pool(db_type, &details).await?;

        self.pool = Some(pool.clone());

        let (spinner_handle, loading) = self.loading().await;

        let metadata = fetch_all_table_metadata(&pool).await?;

        loading.store(false, Ordering::SeqCst);
        spinner_handle.await.unwrap();

        if metadata.is_empty() {
            println!("❌ No tables found in the database.");
            return Ok(());
        }

        println!("✅ Found {} tables", metadata.len());
        let items = metadata_to_tree_items(&metadata);
        self.setup_ui(items, pool).await?;

        stdout().execute(EnableMouseCapture)?;
        let terminal = ratatui::init();
        let _ = self.run(terminal).await;
        ratatui::restore();
        stdout().execute(DisableMouseCapture)?;
        Ok(())
    }

    pub async fn loading(&mut self) -> (JoinHandle<()>, Arc<AtomicBool>) {
        let loading = Arc::new(AtomicBool::new(true));
        let spinner_flag = loading.clone();

        let spinner_handle = tokio::spawn(async move {
            let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut i = 0;
            let mut stdout = stdout();

            while spinner_flag.load(Ordering::SeqCst) {
                let symbol = spinner[i % spinner.len()];
                execute!(
                    stdout,
                    cursor::MoveToColumn(0),
                    Clear(ClearType::CurrentLine),
                    Print(format!("🔄 Fetching tables... {}", symbol)),
                )
                .unwrap();
                stdout.flush().unwrap();
                sleep(Duration::from_millis(100)).await;
                i += 1;
            }

            execute!(
                stdout,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine),
            )
            .unwrap();
        });
        (spinner_handle, loading)
    }

    async fn setup_ui(
        &mut self,
        sidebar_items: Vec<TreeItem<'static, String>>,
        pool: DbPool,
    ) -> Result<()> {
        self.focus = Focus::Sidebar;
        self.sidebar.update_items(sidebar_items);
        self.sidebar.update_focus(Focus::Sidebar);

        let query = self.current_query();
        if !query.is_empty() {
            if let Ok(result) = fetch_query(&pool, &query).await {
                self.data_table = DataTable::new(result.headers, result.rows);
            }
        }

        Ok(())
    }

    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|f| self.render_ui(f))?;
            let _ = self.handle_events().await;
        }
        Ok(())
    }

    async fn handle_events(&mut self) -> Result<()> {
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
                        KeyCode::Enter => {
                            let query = self.current_query();
                            if !query.is_empty() {
                                self.query = query.clone();

                                if let Some(pool) = &self.pool {
                                    match execute_query(pool, &query).await {
                                        Ok(ExecutionResult::Data(result)) => {
                                            self.data_table =
                                                DataTable::new(result.headers, result.rows)
                                        }
                                        Ok(ExecutionResult::Affected(count)) => {
                                            self.status_message =
                                                Some(format!("✅ {} row(s) affected.", count));
                                        }
                                        Err(_) => todo!(),
                                    }
                                }
                            }
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
            Char('l') => self.data_table.next_column(),
            Char('h') => self.data_table.previous_column(),
            Char('x') | Right => self.data_table.scroll_right(),
            Char('X') | Left => self.data_table.scroll_left(),
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
        self.data_table.draw(
            f,
            right[1],
            &self.focus.clone(),
            self.status_message.as_deref(),
        );
    }

    fn toggle_focus(&mut self) {
        self.focus = self.focus.clone().next();
        self.sidebar.update_focus(self.focus.clone());
    }
}
