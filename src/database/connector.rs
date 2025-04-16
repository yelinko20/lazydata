use color_eyre::eyre::Result;
use inquire::{Password, Text};

#[derive(Debug, Clone, Copy)]
pub enum DatabaseType {
    PostgresSQL,
    MySQL,
    SQLite,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ConnectionDetails {
    pub db_type: DatabaseType,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub file_path: Option<String>, // SQLite only
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DatabaseType::PostgresSQL => "PostgreSQL",
            DatabaseType::MySQL => "MySQL",
            DatabaseType::SQLite => "SQLite",
        };
        write!(f, "{s}")
    }
}

impl ConnectionDetails {
    pub fn connection_string(&self) -> String {
        match self.db_type {
            DatabaseType::PostgresSQL => format!(
                "postgres://{}:{}@{}:{}/{}",
                self.username.as_deref().unwrap_or(""),
                self.password.as_deref().unwrap_or(""),
                self.host.as_deref().unwrap_or("localhost"),
                self.port.unwrap_or(5432),
                self.database.as_deref().unwrap_or("")
            ),
            DatabaseType::MySQL => format!(
                "postgres://{}:{}@{}:{}/{}",
                self.username.as_deref().unwrap_or(""),
                self.password.as_deref().unwrap_or(""),
                self.host.as_deref().unwrap_or("localhost"),
                self.port.unwrap_or(3306),
                self.database.as_deref().unwrap_or("")
            ),
            DatabaseType::SQLite => self.file_path.as_deref().unwrap_or("").to_string(),
        }
    }
}

pub fn get_connection_details(db_type: DatabaseType) -> Result<ConnectionDetails> {
    match db_type {
        DatabaseType::SQLite => {
            let file_path = Text::new("Enter SQLite file path:")
                .with_placeholder("./data.db")
                .prompt()?;

            Ok(ConnectionDetails {
                db_type,
                host: None,
                port: None,
                username: None,
                password: None,
                database: None,
                file_path: Some(file_path),
            })
        }
        _ => {
            let host = Text::new("Enter host:")
                .with_placeholder("localhost")
                .prompt()?;
            let port_str = Text::new("Enter port:").with_placeholder("5432").prompt()?;
            let port = port_str.parse::<u16>().unwrap_or(5432);
            let username = Text::new("Enter username:").prompt()?;
            let password = Password::new("Enter password:").prompt()?;
            let database = Text::new("Enter database name:").prompt()?;

            Ok(ConnectionDetails {
                db_type,
                host: Some(host),
                port: Some(port),
                username: Some(username),
                password: Some(password),
                database: Some(database),
                file_path: None,
            })
        }
    }
}
