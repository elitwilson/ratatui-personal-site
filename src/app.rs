use crate::{map::{self, demo_castle}, theme::Theme};
use ratatui::DefaultTerminal;
use crate::render::ui;

pub struct App {
    map: map::Map,
    player_pos: (u16, u16), // x, y position on the map.
    key_pos: (u16, u16),    // where the key sits on the map.
    door_pos: (u16, u16),   // where the door sits on the map.
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
            key_pos: (2, 3),    // left side, like the original game
            door_pos: (3, 2),   // top-middle, the way out
            has_key: false,
            door_open: false,
            show_about: false,
            theme: Theme::default(),
        }
    }

    pub fn update(&mut self, action: Action) {
        let moved = match action {
            Action::MoveUp => self.try_move(0, -1),
            Action::MoveDown => self.try_move(0, 1),
            Action::MoveLeft => self.try_move(-1, 0),
            Action::MoveRight => self.try_move(1, 0),
            Action::ToggleAbout => {
                self.show_about = !self.show_about;
                false
            }
        };

        // Only react to the destination tile if the player actually moved onto it.
        if moved {
            self.enter_tile();
        }
    }

    /// Move the player by a signed delta if the target is walkable.
    /// Returns whether the move happened.
    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let nx = self.player_pos.0 as i32 + dx;
        let ny = self.player_pos.1 as i32 + dy;

        // walkable rejects negative and out-of-range coords, so if it passes
        // we know nx/ny are valid, non-negative, and safe to store as u16.
        if self.map.walkable(nx, ny) {
            self.player_pos = (nx as u16, ny as u16);
            true
        } else {
            false
        }
    }

    /// React to whatever occupies the tile the player just stepped onto.
    fn enter_tile(&mut self) {
        if self.player_pos == self.key_pos {
            self.has_key = true;
        }

        if self.player_pos == self.door_pos && self.has_key {
            self.door_open = true;
            self.show_about = true; // reaching the door with the key is the win
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

    fn app_with(player: (u16, u16), key: (u16, u16)) -> App {
        App {
            map: demo_castle(),
            player_pos: player,
            key_pos: key,
            door_pos: (1, 1), // parked on a wall, unreachable, for tests that ignore it
            has_key: false,
            door_open: false,
            show_about: false,
            theme: Theme::default(),
        }
    }

    // Movement tests don't care about the key, so park it out of the way.
    fn app_at(pos: (u16, u16)) -> App {
        app_with(pos, (4, 4))
    }

    // Builder for door tests: place the door and choose whether we hold the key.
    fn app_door(player: (u16, u16), door: (u16, u16), has_key: bool) -> App {
        App {
            map: demo_castle(),
            player_pos: player,
            key_pos: (1, 1), // parked; door tests set has_key directly
            door_pos: door,
            has_key,
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

    // Stepping onto the key's tile picks it up.
    #[test]
    fn stepping_onto_key_picks_it_up() {
        // player at (2,2), key one step to the right at (3,2) (both floor)
        let mut app = app_with((2, 2), (3, 2));
        assert!(!app.has_key); // sanity: not collected yet
        app.update(Action::MoveRight);
        assert_eq!(app.player_pos, (3, 2)); // actually moved onto it
        assert!(app.has_key); // and picked it up
    }

    // Stepping onto an ordinary floor tile (not the key) doesn't grant the key.
    #[test]
    fn stepping_onto_floor_does_not_pick_up_key() {
        let mut app = app_with((2, 2), (4, 4)); // key is elsewhere
        app.update(Action::MoveRight); // moves onto (3,2), a plain floor tile
        assert!(!app.has_key);
    }

    // Once collected, the key stays collected after the player moves away.
    #[test]
    fn key_stays_collected_after_moving_away() {
        let mut app = app_with((2, 2), (3, 2));
        app.update(Action::MoveRight); // pick up key at (3,2)
        assert!(app.has_key);
        app.update(Action::MoveRight); // move on to (4,2)
        assert_eq!(app.player_pos, (4, 2));
        assert!(app.has_key); // still have it
    }

    // Stepping onto the door while holding the key opens it and reveals About.
    #[test]
    fn stepping_onto_door_with_key_opens_it() {
        // player at (2,2), door one step right at (3,2), key already in hand
        let mut app = app_door((2, 2), (3, 2), true);
        app.update(Action::MoveRight);
        assert_eq!(app.player_pos, (3, 2)); // moved onto the door
        assert!(app.door_open);
        assert!(app.show_about);
    }

    // Stepping onto the door without the key does nothing.
    #[test]
    fn stepping_onto_door_without_key_does_nothing() {
        let mut app = app_door((2, 2), (3, 2), false);
        app.update(Action::MoveRight);
        assert!(!app.door_open);
        assert!(!app.show_about);
    }
}
