mod app;
mod database;
mod layout;
mod style;

use app::App;
use color_eyre::eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let mut app = App::default();
    app.init().await?;
    Ok(())
}
