mod about;
mod app;
mod block_font;
mod effects;
mod input;
mod map;
mod menu;
mod particle_render;
mod particles;
mod render;
mod rng;
mod router;
mod sandbox;
mod sandbox_config;
mod theme;
mod victory;

#[cfg(target_arch = "wasm32")]
mod web;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Screen {
    Menu,
    Game,
    About,
    Sandbox,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Nav {
    To(Screen),
    /// Open an external URL (e.g. the GitHub link). Each runner handles this in
    /// a target-appropriate way and stays on the current screen.
    OpenUrl(&'static str),
    Quit,
}

// --- Native (terminal) entry point ---

#[cfg(not(target_arch = "wasm32"))]
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(native_run)?;
    Ok(())
}

/// Native runner: a blocking loop over crossterm events with `Instant`-based
/// timing. Drives the shared [`router::Router`]; `Nav::Quit` exits the process.
#[cfg(not(target_arch = "wasm32"))]
fn native_run(terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
    use crate::app::FRAME_TIME;
    use crossterm::event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseButton,
        MouseEventKind,
    };
    use crossterm::execute;
    use std::io::stdout;
    use std::time::{Duration, Instant};

    let mut router = router::Router::new();

    // Opt into mouse events for the whole session; only the sandbox consumes
    // them, the other screens ignore the reports.
    execute!(stdout(), EnableMouseCapture)?;
    let mut last = Instant::now();

    let result = loop {
        terminal.draw(|frame| {
            router.render(frame);
        })?;

        let now = Instant::now();
        let dt = now - last;
        last = now;

        // Block up to one frame for the first event (this paces the loop), then
        // drain the rest so rapid mouse motion doesn't lag or back up the queue.
        let mut pending_nav = None;
        if event::poll(FRAME_TIME)? {
            loop {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if let Some(nav) = router.handle_key(key.code) {
                            pending_nav = Some(nav);
                            break;
                        }
                    }
                    Event::Mouse(me) => {
                        router.set_mouse((me.column, me.row));
                        if matches!(me.kind, MouseEventKind::Down(MouseButton::Left)) {
                            if let Some(nav) = router.handle_click((me.column, me.row)) {
                                pending_nav = Some(nav);
                                break;
                            }
                        }
                    }
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }

        if let Some(nav) = pending_nav {
            match nav {
                Nav::Quit => break Ok(()),
                Nav::To(screen) => router.goto(screen),
                // Launch the OS browser; stay on the current screen. A failure
                // to open (no browser, headless) is non-fatal — just ignore it.
                Nav::OpenUrl(url) => {
                    let _ = open::that(url);
                }
            }
        }

        router.tick(dt);
    };

    let _ = execute!(stdout(), DisableMouseCapture);
    result
}

// --- Web (WASM) entry point ---

#[cfg(target_arch = "wasm32")]
fn main() -> std::io::Result<()> {
    web::run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    // --- Nav/Screen enum shapes ---

    #[test]
    fn nav_quit_is_distinct_from_to() {
        let q = Nav::Quit;
        let t = Nav::To(Screen::Menu);
        assert_ne!(q, t);
    }

    #[test]
    fn screen_variants_are_distinct() {
        assert_ne!(Screen::Menu, Screen::Game);
        assert_ne!(Screen::Game, Screen::About);
        assert_ne!(Screen::About, Screen::Sandbox);
    }

    #[test]
    fn nav_to_carries_the_target_screen() {
        let nav = Nav::To(Screen::Game);
        assert_eq!(nav, Nav::To(Screen::Game));
    }

    // --- Router-level pure transition seams ---

    // menu::activate routes each item to the right Nav
    #[test]
    fn menu_activate_play_routes_to_game() {
        assert_eq!(menu::activate(menu::MenuItem::Play), Nav::To(Screen::Game));
    }

    #[test]
    fn menu_activate_about_routes_to_about() {
        assert_eq!(
            menu::activate(menu::MenuItem::About),
            Nav::To(Screen::About)
        );
    }

    #[test]
    fn menu_activate_quit_routes_to_quit() {
        assert_eq!(menu::activate(menu::MenuItem::Quit), Nav::Quit);
    }

    // game: Esc/q → Command::Quit (which the loop translates to Nav::To(Menu))
    #[test]
    fn game_esc_maps_to_quit_command() {
        assert_eq!(app::map_key(KeyCode::Esc), Some(app::Command::Quit));
    }

    #[test]
    fn game_q_maps_to_quit_command() {
        assert_eq!(app::map_key(KeyCode::Char('q')), Some(app::Command::Quit));
    }

    // game: p → Command::Switch (which the loop translates to Nav::To(Sandbox))
    #[test]
    fn game_p_maps_to_switch_command() {
        assert_eq!(app::map_key(KeyCode::Char('p')), Some(app::Command::Switch));
    }

    // about: Esc/q → AboutCommand::Back (which the loop translates to Nav::To(Menu))
    #[test]
    fn about_esc_maps_to_back() {
        assert_eq!(
            about::map_about_key(KeyCode::Esc),
            about::AboutCommand::Back
        );
    }

    #[test]
    fn about_q_maps_to_back() {
        assert_eq!(
            about::map_about_key(KeyCode::Char('q')),
            about::AboutCommand::Back
        );
    }

    // sandbox: Esc/q → SandboxCommand::Quit (loop → Nav::To(Menu))
    #[test]
    fn sandbox_esc_maps_to_quit_command() {
        assert_eq!(
            sandbox::map_sandbox_key(KeyCode::Esc),
            sandbox::SandboxCommand::Quit
        );
    }

    #[test]
    fn sandbox_q_maps_to_quit_command() {
        assert_eq!(
            sandbox::map_sandbox_key(KeyCode::Char('q')),
            sandbox::SandboxCommand::Quit
        );
    }

    // sandbox: p → SandboxCommand::Switch (loop → Nav::To(Game))
    #[test]
    fn sandbox_p_maps_to_switch_command() {
        assert_eq!(
            sandbox::map_sandbox_key(KeyCode::Char('p')),
            sandbox::SandboxCommand::Switch
        );
    }
}
