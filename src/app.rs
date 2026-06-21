use crate::{map::{self, demo_castle}, theme::Theme};
use ratatui::DefaultTerminal;
use crate::render::ui;

pub struct App {
    map: map::Map,
    player_pos: (u16, u16), // x, y position on the map.
    has_key: bool,
    door_open: bool,
    show_about: bool,
    theme: Theme,
}

pub enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    ToggleAbout,
    // etc.
}

impl App {
    pub fn new() -> Self {
        Self {
            map: demo_castle(),
            player_pos: (3, 4), // bottom-middle of the interior floor
            has_key: false,
            door_open: false,
            show_about: false,
            theme: Theme::default(),
        }
    }

    pub fn update(&mut self, action: Action) {
        let (x, y) = self.player_pos;

        // saturating_sub clamps to 0 instead of underflowing (which would panic).
        let new_pos = match action {
            Action::MoveUp => Some((x, y.saturating_sub(1))),
            Action::MoveDown => Some((x, y + 1)),
            Action::MoveLeft => Some((x.saturating_sub(1), y)),
            Action::MoveRight => Some((x + 1, y)),
            Action::ToggleAbout => {
                self.show_about = !self.show_about;
                None
            }
        };

        if let Some((nx, ny)) = new_pos
            && self.map.walkable(nx, ny)
        {
            self.player_pos = (nx, ny);
        }
    }
}

pub fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    loop {
        terminal.draw(ui)?;
        if crossterm::event::read()?.is_key_press() {
            break Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_at(pos: (u16, u16)) -> App {
        App {
            map: demo_castle(),
            player_pos: pos,
            has_key: false,
            door_open: false,
            show_about: false,
            theme: Theme::default(),
        }
    }

    #[test]
    fn app_initial_state() {
        let app = App::new();
        assert_eq!(app.player_pos, (3, 4));
        assert!(!app.has_key);
        assert!(!app.door_open);
        assert!(!app.show_about);
    }

    #[test]
    fn app_update_toggles_about() {
        let mut app = App::new();
        app.update(Action::ToggleAbout);
        assert!(app.show_about);
    }

    // Moving onto a floor tile updates the player's position.
    #[test]
    fn move_onto_floor_updates_position() {
        let mut app = app_at((2, 2)); // floor; (3,2) is also floor
        app.update(Action::MoveRight);
        assert_eq!(app.player_pos, (3, 2));
    }

    // Moving into a wall leaves the player where they were.
    #[test]
    fn move_into_wall_is_blocked() {
        let mut app = app_at((2, 3)); // floor; (3,3) to the right is the center wall
        app.update(Action::MoveRight);
        assert_eq!(app.player_pos, (2, 3));
    }

    // Moving past the edge (coordinate 0) must not underflow or move.
    #[test]
    fn move_off_edge_is_blocked() {
        let mut app = app_at((0, 0));
        app.update(Action::MoveLeft); // would be (-1, 0) -> underflow if unguarded
        assert_eq!(app.player_pos, (0, 0));
    }
}
