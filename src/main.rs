mod app;
mod map;
mod particles;
mod render;
mod theme;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(app::app)?;
    Ok(())
}
