mod about;
mod app;
mod block_font;
mod effects;
mod map;
mod menu;
mod particle_render;
mod particles;
mod render;
mod rng;
mod sandbox;
mod sandbox_config;
mod theme;
mod victory;

use ratatui::DefaultTerminal;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Menu,
    Game,
    About,
    Sandbox,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Nav {
    To(Screen),
    Quit,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(run)?;
    Ok(())
}

fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut current = Screen::Menu;
    loop {
        let nav = match current {
            Screen::Menu => menu::menu(terminal)?,
            Screen::Game => app::app(terminal)?,
            Screen::About => about::about(terminal)?,
            Screen::Sandbox => sandbox::sandbox(terminal)?,
        };
        match nav {
            Nav::Quit => return Ok(()),
            Nav::To(screen) => current = screen,
        }
    }
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
        assert_eq!(menu::activate(menu::MenuItem::About), Nav::To(Screen::About));
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
        assert_eq!(about::map_about_key(KeyCode::Esc), about::AboutCommand::Back);
    }

    #[test]
    fn about_q_maps_to_back() {
        assert_eq!(about::map_about_key(KeyCode::Char('q')), about::AboutCommand::Back);
    }

    // sandbox: Esc/q → SandboxCommand::Quit (loop → Nav::To(Menu))
    #[test]
    fn sandbox_esc_maps_to_quit_command() {
        assert_eq!(sandbox::map_sandbox_key(KeyCode::Esc), sandbox::SandboxCommand::Quit);
    }

    #[test]
    fn sandbox_q_maps_to_quit_command() {
        assert_eq!(sandbox::map_sandbox_key(KeyCode::Char('q')), sandbox::SandboxCommand::Quit);
    }

    // sandbox: p → SandboxCommand::Switch (loop → Nav::To(Game))
    #[test]
    fn sandbox_p_maps_to_switch_command() {
        assert_eq!(sandbox::map_sandbox_key(KeyCode::Char('p')), sandbox::SandboxCommand::Switch);
    }
}
