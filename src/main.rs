mod database;

use color_eyre::eyre::Result;
use database::{
    connector::{ConnectionDetails, DatabaseType, get_connection_details},
    detector::get_installed_databases,
};
use inquire::Select;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let databases = get_installed_databases()?;

    if databases.is_empty() {
        println!("❌ No databases detected!");
        return Ok(());
    }

    let selected = Select::new("🚀 Select a Database", databases.clone())
        .with_help_message("Use ↑ ↓ arrows, Enter to select")
        .prompt();

    if let Ok(db_name) = selected {
        let db_type = match db_name.to_lowercase().as_str() {
            "postgresql" => DatabaseType::PostgresSQL,
            "mysql" => DatabaseType::MySQL,
            "sqlite" => DatabaseType::SQLite,
            _ => {
                println!("❌ Unsupported database.");
                return Ok(());
            }
        };
        let details: ConnectionDetails = get_connection_details(db_type)?;
        println!("\n✅ Connection Details:\n{:#?}", details);
    } else {
        println!("\n👋 Exited without selection.")
    }

    Ok(())
}
