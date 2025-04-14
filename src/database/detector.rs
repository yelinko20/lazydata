use color_eyre::eyre::Result;
use std::process::Command;

#[derive(Debug)]
pub struct DatabaseChecker {
    pub name: &'static str,
    pub command: &'static str,
    pub args: &'static [&'static str],
}

pub fn get_installed_databases() -> Result<Vec<String>> {
    let db_tools = [
        DatabaseChecker {
            name: "PostgresSQL",
            command: "pg_isready",
            args: &[],
        },
        DatabaseChecker {
            name: "MySQL",
            command: "mysql",
            args: &["--version"],
        },
        DatabaseChecker {
            name: "SQLite",
            command: "sqlite3",
            args: &["--version"],
        },
    ];

    let mut found = Vec::new();

    for tool in db_tools.iter() {
        match Command::new(tool.command).args(tool.args).output() {
            Ok(output) if output.status.success() => {
                found.push(tool.name.to_string());
            }
            Ok(output) => {
                eprintln!(
                    "⚠️ {} found but returned err: {}",
                    tool.name,
                    String::from_utf8_lossy(&output.stderr)
                )
            }
            Err(e) => {
                eprintln!("❗ Failed to run {}: {}", tool.name, e)
            }
        }
    }

    Ok(found)
}
