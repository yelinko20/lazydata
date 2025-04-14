mod database;

use std::{
    io::{stdout, Stdout, Write},
    time::Duration,
};

use color_eyre::eyre::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use database::detector::get_installed_databases;

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), Show);
    }
}

fn drain_initial_events() -> Result<()> {
    while event::poll(std::time::Duration::from_millis(1))? {
        if let Event::Key(_) = event::read()? {}
    }
    Ok(())
}

fn render_menu<W: Write>(out: &mut W, databases: &[String], selected: usize) -> Result<()> {
    execute!(out, MoveTo(0, 0), Clear(ClearType::All))?;

    let title = "🚀 Select a Database";
    let instructions = "(Use ↑ ↓ arrows, Enter to select, q to quit)\n";

    writeln!(out, "{:^80}", title)?;
    writeln!(out, "{:^80}", instructions)?;

    for (idx, db) in databases.iter().enumerate() {
        if idx == selected {
            execute!(
                out,
                SetForegroundColor(Color::Green),
                SetAttribute(Attribute::Bold),
                Print(format!("> {}\n", db)),
                ResetColor,
                SetAttribute(Attribute::Reset)
            )?;
        } else {
            writeln!(out, "  {}", db)?;
        }
    }

    out.flush()?;
    Ok(())
}

fn handle_input(
    stdout: &mut Stdout,
    databases: &[String],
    selected_index: &mut usize,
) -> Result<Option<usize>> {
    loop {
        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    match key_event.code {
                        KeyCode::Up => {
                            if *selected_index > 0 {
                                *selected_index -= 1;
                                let _ = render_menu(stdout, databases, *selected_index);
                            }
                        }
                        KeyCode::Down => {
                            if *selected_index < databases.len().saturating_sub(1) {
                                *selected_index += 1;
                                let _ = render_menu(stdout, databases, *selected_index);
                            }
                        }
                        KeyCode::Enter => {
                            execute!(stdout, Show)?;
                            return Ok(Some(*selected_index));
                        }
                        KeyCode::Char('q') => {
                            execute!(stdout, Show)?;
                            return Ok(None);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let _guard = RawModeGuard;
    enable_raw_mode()?;
    execute!(stdout(), Hide)?;

    let mut stdout = stdout();
    let mut selected_index = 0;
    let databases = get_installed_databases()?;
    drain_initial_events()?;

    render_menu(&mut stdout, &databases, selected_index)?;
    match handle_input(&mut stdout, &databases, &mut selected_index)? {
        Some(index) => println!("\n✅ You selected: {}", databases[index]),
        None => println!("\n👋 Exited without selection."),
    }

    Ok(())
}