use crate::map::Map;
#[cfg(test)]
use crate::map::demo_castle;
use crate::render;
use crate::theme::Theme;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use std::time::{Duration, Instant};

const FRAME_TIME: Duration = Duration::from_millis(16);

pub struct App {
    map: Map,
    player_pos: (u16, u16), // x, y position on the map.
    key_pos: (u16, u16),    // where the key sits on the map.
    door_pos: (u16, u16),   // where the door sits on the map.
    has_key: bool,
    door_open: bool,
    show_about: bool,
    theme: Theme,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    ToggleAbout,
}

/// What a keypress resolves to: either drive the game, or quit.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Game(Action),
    Quit,
}

/// Pure mapping from a key to a command. No IO — this is the testable seam
/// between the event loop and the game.
pub fn map_key(code: KeyCode) -> Option<Command> {
    let command = match code {
        KeyCode::Up | KeyCode::Char('w') => Command::Game(Action::MoveUp),
        KeyCode::Down | KeyCode::Char('s') => Command::Game(Action::MoveDown),
        KeyCode::Left | KeyCode::Char('a') => Command::Game(Action::MoveLeft),
        KeyCode::Right | KeyCode::Char('d') => Command::Game(Action::MoveRight),
        KeyCode::Char('i') => Command::Game(Action::ToggleAbout),
        KeyCode::Char('q') | KeyCode::Esc => Command::Quit,
        _ => return None,
    };
    Some(command)
}

impl App {
    pub fn new() -> Self {
        // The castle map is a compile-time constant baked in via include_str!.
        // A parse failure here is a bug in the map file, not a runtime condition,
        // so .expect() is the correct resolution — the guard test in map.rs
        // ensures this never fires in practice.
        let level = include_str!("../assets/castle.map")
            .parse::<crate::map::Level>()
            .expect("built-in castle map should parse");
        Self {
            map: level.map,
            player_pos: level.player,
            key_pos: level.key,
            door_pos: level.door,
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

    // --- Read surface ---
    //
    // The renderer lives in a different module, so Rust's privacy rules hide
    // App's fields from it. Rather than make the fields `pub` (which would also
    // let outside code *mutate* them and would leak our internal representation),
    // we expose read-only accessors: `Copy` state is returned by value, heavier
    // state (`Map`, `Theme`) by shared reference. The renderer can observe game
    // state but cannot change it — only `update`/`try_move` mutate, which keeps
    // every state transition in one place.
    pub fn map(&self) -> &Map {
        &self.map
    }
    pub fn theme(&self) -> &Theme {
        &self.theme
    }
    pub fn player_pos(&self) -> (u16, u16) {
        self.player_pos
    }
    pub fn key_pos(&self) -> (u16, u16) {
        self.key_pos
    }
    pub fn door_pos(&self) -> (u16, u16) {
        self.door_pos
    }
    pub fn has_key(&self) -> bool {
        self.has_key
    }
    pub fn show_about(&self) -> bool {
        self.show_about
    }

    /// Per-frame hook called once per loop iteration with the real elapsed
    /// time since the last frame. No-op today; the seam exists so future
    /// time-based animation has somewhere to live.
    pub fn tick(&mut self, _dt: Duration) {}
}

pub fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut app = App::new();
    let mut last = Instant::now();
    loop {
        terminal.draw(|frame| render::ui(frame, &app))?;

        let now = Instant::now();
        let dt = now - last;
        last = now;

        if event::poll(FRAME_TIME)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match map_key(key.code) {
                Some(Command::Quit) => return Ok(()),
                Some(Command::Game(action)) => app.update(action),
                None => {}
            }
        }

        app.tick(dt);
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

    // --- input mapping (pure key -> command) ---

    #[test]
    fn arrow_keys_map_to_movement() {
        assert_eq!(map_key(KeyCode::Up), Some(Command::Game(Action::MoveUp)));
        assert_eq!(
            map_key(KeyCode::Down),
            Some(Command::Game(Action::MoveDown))
        );
        assert_eq!(
            map_key(KeyCode::Left),
            Some(Command::Game(Action::MoveLeft))
        );
        assert_eq!(
            map_key(KeyCode::Right),
            Some(Command::Game(Action::MoveRight))
        );
    }

    #[test]
    fn wasd_keys_map_to_movement() {
        assert_eq!(
            map_key(KeyCode::Char('w')),
            Some(Command::Game(Action::MoveUp))
        );
        assert_eq!(
            map_key(KeyCode::Char('s')),
            Some(Command::Game(Action::MoveDown))
        );
        assert_eq!(
            map_key(KeyCode::Char('a')),
            Some(Command::Game(Action::MoveLeft))
        );
        assert_eq!(
            map_key(KeyCode::Char('d')),
            Some(Command::Game(Action::MoveRight))
        );
    }

    #[test]
    fn i_toggles_about() {
        assert_eq!(
            map_key(KeyCode::Char('i')),
            Some(Command::Game(Action::ToggleAbout))
        );
    }

    #[test]
    fn q_and_esc_quit() {
        assert_eq!(map_key(KeyCode::Char('q')), Some(Command::Quit));
        assert_eq!(map_key(KeyCode::Esc), Some(Command::Quit));
    }

    #[test]
    fn unmapped_keys_do_nothing() {
        assert_eq!(map_key(KeyCode::Char('z')), None);
        assert_eq!(map_key(KeyCode::Enter), None);
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
        assert_eq!(app.player_pos, (10, 8));
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
