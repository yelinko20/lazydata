use super::pool::DbPool;
use color_eyre::eyre::Result;
use sqlx::{Row, mysql::MySqlRow, postgres::PgRow, sqlite::SqliteRow};

pub enum TableRows {
    Postgres(Vec<PgRow>),
    MySQL(Vec<MySqlRow>),
    SQLite(Vec<SqliteRow>),
}

impl TableRows {
    pub fn table_names(&self) -> Vec<String> {
        match self {
            TableRows::Postgres(rows) => rows
                .iter()
                .map(|row| row.get::<String, _>("table_name"))
                .collect(),
            TableRows::MySQL(rows) => rows.iter().map(|row| row.get::<String, _>(0)).collect(),
            TableRows::SQLite(rows) => rows
                .iter()
                .map(|row| row.get::<String, _>("name"))
                .collect(),
        }
    }

    /// Returns the number of tables.
    pub fn len(&self) -> usize {
        match self {
            TableRows::Postgres(rows) => rows.len(),
            TableRows::MySQL(rows) => rows.len(),
            TableRows::SQLite(rows) => rows.len(),
        }
    }

    /// Checks if there are no tables.
    pub fn is_empty(&self) -> bool {
        match self {
            TableRows::Postgres(rows) => rows.is_empty(),
            TableRows::MySQL(rows) => rows.is_empty(),
            TableRows::SQLite(rows) => rows.is_empty(),
        }
    }
}

pub async fn get_tables(pool: DbPool) -> Result<TableRows> {
    match pool {
        DbPool::Postgres(pg_pool) => {
            let rows = sqlx::query(
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
            )
            .fetch_all(&pg_pool)
            .await?;
            Ok(TableRows::Postgres(rows))
        }
        DbPool::MySQL(mysql_pool) => {
            let rows = sqlx::query("SHOW TABLES").fetch_all(&mysql_pool).await?;
            Ok(TableRows::MySQL(rows))
        }
        DbPool::SQLite(sqlite_pool) => {
            let rows = sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table'")
                .fetch_all(&sqlite_pool)
                .await?;
            Ok(TableRows::SQLite(rows))
        }
    }
}
