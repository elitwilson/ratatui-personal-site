mod app;
mod effects;
mod map;
mod particle_render;
mod particles;
mod render;
mod rng;
mod sandbox;
mod theme;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    // Temporary detour: sandbox exercises the particle epic end-to-end.
    // Restore the game with a one-line swap: replace sandbox::sandbox with app::app.
    ratatui::run(sandbox::sandbox)?;
    Ok(())
}
