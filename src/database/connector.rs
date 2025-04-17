use color_eyre::eyre::Result;
use inquire::{Password, Text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
}

#[derive(Debug, PartialEq, Eq)]
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
            DatabaseType::PostgreSQL => "PostgreSQL",
            DatabaseType::MySQL => "MySQL",
            DatabaseType::SQLite => "SQLite",
        };
        write!(f, "{s}")
    }
}

impl ConnectionDetails {
    pub fn connection_string(&self) -> String {
        match self.db_type {
            DatabaseType::PostgreSQL => format!(
                "postgres://{}:{}@{}:{}/{}",
                self.username.as_deref().unwrap_or(""),
                self.password.as_deref().unwrap_or(""),
                self.host.as_deref().unwrap_or("localhost"),
                self.port.unwrap_or(5432),
                self.database.as_deref().unwrap_or("")
            ),
            DatabaseType::MySQL => format!(
                "mysql://{}:{}@{}:{}/{}",
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_database_type_display() {
        assert_eq!(DatabaseType::PostgreSQL.to_string(), "PostgreSQL");
        assert_eq!(DatabaseType::MySQL.to_string(), "MySQL");
        assert_eq!(DatabaseType::SQLite.to_string(), "SQLite")
    }

    #[test]
    fn test_postgres_connection_string() {
        let details = ConnectionDetails {
            db_type: DatabaseType::PostgreSQL,
            host: Some("localhost".to_string()),
            port: Some(5432),
            username: Some("user".to_string()),
            password: Some("P@ssw0rd!".to_string()),
            database: Some("db".to_string()),
            file_path: None,
        };
        assert_eq!(
            details.connection_string(),
            "postgres://user:P@ssw0rd!@localhost:5432/db"
        )
    }

    #[test]
    fn test_mysql_connection_string() {
        let details = ConnectionDetails {
            db_type: DatabaseType::MySQL,
            host: Some("192.168.1.100".to_string()),
            port: Some(3306),
            username: Some("admin".to_string()),
            password: Some("secure_password".to_string()),
            database: Some("mydb".to_string()),
            file_path: None,
        };
        assert_eq!(
            details.connection_string(),
            "mysql://admin:secure_password@192.168.1.100:3306/mydb"
        )
    }

    #[test]
    fn test_sqlite_connection_string() {
        let details = ConnectionDetails {
            db_type: DatabaseType::SQLite,
            host: None,
            port: None,
            username: None,
            password: None,
            database: None,
            file_path: Some("/path/to/my.db".to_string()),
        };
        assert_eq!(details.connection_string(), "/path/to/my.db")
    }

    #[test]
    fn test_postgres_with_missing_fields() {
        let details = ConnectionDetails {
            db_type: DatabaseType::PostgreSQL,
            host: None,
            port: None,
            username: None,
            password: None,
            database: None,
            file_path: None,
        };
        assert_eq!(details.connection_string(), "postgres://:@localhost:5432/");
    }

    #[test]
    fn test_mysql_with_missing_fields() {
        let details = ConnectionDetails {
            db_type: DatabaseType::MySQL,
            host: None,
            port: None,
            username: None,
            password: None,
            database: None,
            file_path: None,
        };
        assert_eq!(details.connection_string(), "mysql://:@localhost:3306/");
    }

    #[test]
    fn test_sqlite_empty_path() {
        let details = ConnectionDetails {
            db_type: DatabaseType::SQLite,
            host: None,
            port: None,
            username: None,
            password: None,
            database: None,
            file_path: None,
        };
        assert_eq!(details.connection_string(), "");
    }

    #[test]
    fn test_postgres_with_empty_credentials() {
        let details = ConnectionDetails {
            db_type: DatabaseType::PostgreSQL,
            host: Some("localhost".to_string()),
            port: Some(5432),
            username: Some("".to_string()),
            password: Some("".to_string()),
            database: Some("testdb".to_string()),
            file_path: None,
        };
        assert_eq!(
            details.connection_string(),
            "postgres://:@localhost:5432/testdb"
        );
    }

    #[test]
    fn test_mysql_custom_port() {
        let details = ConnectionDetails {
            db_type: DatabaseType::MySQL,
            host: Some("127.0.0.1".to_string()),
            port: Some(3307),
            username: Some("root".to_string()),
            password: Some("1234".to_string()),
            database: Some("example".to_string()),
            file_path: None,
        };
        assert_eq!(
            details.connection_string(),
            "mysql://root:1234@127.0.0.1:3307/example"
        );
    }
}
