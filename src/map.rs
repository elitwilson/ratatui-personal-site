#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    Outside,
}

pub struct Map {
    tiles: Vec<Vec<Tile>>,
    width: usize,
    height: usize,
}

impl Map {
    /// Build a map from rows of tiles. Width/height are derived from the data.
    pub fn new(tiles: Vec<Vec<Tile>>) -> Self {
        let height = tiles.len();
        let width = tiles.first().map_or(0, |row| row.len());
        Self {
            tiles,
            width,
            height,
        }
    }

    pub fn width(&self) -> u16 {
        self.width as u16
    }

    pub fn height(&self) -> u16 {
        self.height as u16
    }

    /// The tile at (x, y). Callers must stay in bounds (the renderer iterates
    /// over width/height, so it always is).
    pub fn tile(&self, x: u16, y: u16) -> Tile {
        self.tiles[y as usize][x as usize]
    }

    pub fn walkable(&self, x: i32, y: i32) -> bool {
        // Any coordinate off the grid — negative or too large — is not walkable.
        // Guarding here also keeps the index below from panicking.
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }
        matches!(self.tiles[y as usize][x as usize], Tile::Floor)
    }
}

/// The playable castle: a battlemented (crenellated) top, a rectangular
/// interior, and a gated entrance at the bottom center.
pub fn castle() -> Map {
    use Tile::{Floor as F, Outside as O, Wall as W};
    Map::new(vec![
        vec![W, W, O, W, O, W, O, W, O, F, F, F, O, W, O, W, O, W, W, W, W],
        vec![W, W, W, W, W, W, W, W, F, F, F, F, O, W, O, W, O, W, W, W, W],
        vec![W, F, F, F, F, F, F, F, F, F, F, F, F, F, W, W, W, W, F, F, W],
        vec![W, F, F, F, F, F, F, F, F, F, F, F, F, F, W, W, W, W, F, F, W],
        vec![W, F, F, F, F, F, F, F, F, F, F, F, F, F, W, W, W, W, F, F, W],
        vec![W, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, W],
        vec![W, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, W],
        vec![W, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, W],
        vec![W, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, F, W],
        vec![W, W, W, W, W, W, W, W, W, W, F, W, W, W, W, W, W, W, W, W, W],
        vec![O, O, O, O, O, O, O, O, O, O, F, O, O, O, O, O, O, O, O, O, O],
    ])
}

/// A parsed level: the map grid plus entity starting positions.
///
/// The legend used in `.map` text files:
/// - `#` → Wall
/// - `.` → Floor
/// - ` ` (space) → Outside (the void)
/// - `@` → Player, placed on a Floor tile
/// - `k` → Key, placed on a Floor tile
/// - `D` → Door, placed on a Floor tile
///
/// Leading spaces are significant — they denote Outside tiles, so `.map`
/// files must not be re-indented.
pub struct Level {
    pub map: Map,
    pub player: (u16, u16),
    pub key: (u16, u16),
    pub door: (u16, u16),
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnknownChar(char),
    MissingEntity(&'static str),   // "player" | "key" | "door"
    DuplicateEntity(&'static str),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownChar(c) => write!(f, "unknown map character: {:?}", c),
            ParseError::MissingEntity(e) => write!(f, "missing required entity: {}", e),
            ParseError::DuplicateEntity(e) => write!(f, "duplicate entity: {}", e),
        }
    }
}

impl std::error::Error for ParseError {}

impl std::str::FromStr for Level {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Strip a trailing newline so files with or without one parse identically.
        let s = s.trim_end_matches('\n');

        let lines: Vec<&str> = s.split('\n').collect();
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);

        let mut player: Option<(u16, u16)> = None;
        let mut key: Option<(u16, u16)> = None;
        let mut door: Option<(u16, u16)> = None;

        let mut rows: Vec<Vec<Tile>> = Vec::with_capacity(lines.len());

        for (y, line) in lines.iter().enumerate() {
            let mut row: Vec<Tile> = Vec::with_capacity(width);

            for (x, ch) in line.chars().enumerate() {
                let tile = match ch {
                    '#' => Tile::Wall,
                    '.' => Tile::Floor,
                    ' ' => Tile::Outside,
                    '@' => {
                        if player.is_some() {
                            return Err(ParseError::DuplicateEntity("player"));
                        }
                        player = Some((x as u16, y as u16));
                        Tile::Floor
                    }
                    'k' => {
                        if key.is_some() {
                            return Err(ParseError::DuplicateEntity("key"));
                        }
                        key = Some((x as u16, y as u16));
                        Tile::Floor
                    }
                    'D' => {
                        if door.is_some() {
                            return Err(ParseError::DuplicateEntity("door"));
                        }
                        door = Some((x as u16, y as u16));
                        Tile::Floor
                    }
                    c => return Err(ParseError::UnknownChar(c)),
                };
                row.push(tile);
            }

            // Pad short rows with Outside to reach the max width.
            while row.len() < width {
                row.push(Tile::Outside);
            }

            rows.push(row);
        }

        let player = player.ok_or(ParseError::MissingEntity("player"))?;
        let key = key.ok_or(ParseError::MissingEntity("key"))?;
        let door = door.ok_or(ParseError::MissingEntity("door"))?;

        Ok(Level {
            map: Map::new(rows),
            player,
            key,
            door,
        })
    }
}

/// A small fixture castle used by tests, with a known wall at its center.
#[cfg(test)]
pub fn demo_castle() -> Map {
    use Tile::{Floor as F, Outside as O, Wall as W};
    Map::new(vec![
        vec![O, O, O, O, O, O, O],
        vec![O, W, W, W, W, W, O],
        vec![O, W, F, F, F, W, O],
        vec![O, W, F, W, F, W, O],
        vec![O, W, F, F, F, W, O],
        vec![O, W, W, W, W, W, O],
        vec![O, O, O, O, O, O, O],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    // Local aliases so the grid reads like ASCII art.
    const O: Tile = Tile::Outside;
    const W: Tile = Tile::Wall;
    const F: Tile = Tile::Floor;

    fn test_map() -> Map {
        Map::new(vec![
            vec![O, O, O, O, O, O, O],
            vec![O, W, W, W, W, W, O],
            vec![O, W, F, F, F, W, O],
            vec![O, W, F, W, F, W, O],
            vec![O, W, F, F, F, W, O],
            vec![O, W, W, W, W, W, O],
            vec![O, O, O, O, O, O, O],
        ])
    }

    // A floor tile is something the player can stand on.
    #[test]
    fn floor_tile_is_walkable() {
        let can_walk = test_map().walkable(2, 2);
        assert!(can_walk);
    }

    // A wall tile is solid — the player cannot move onto it.
    #[test]
    fn wall_tile_is_not_walkable() {
        // Arrange: a map with a known wall tile at some (x, y)
        // Act / Assert: walkable(...) returns false
        let can_walk = test_map().walkable(1, 2);
        assert!(!can_walk);
    }

    // "Outside" is the black void around the castle — also not walkable.
    #[test]
    fn outside_tile_is_not_walkable() {
        let can_walk = test_map().walkable(0, 0);
        assert!(!can_walk);
    }

    // Coordinates beyond the edges of the map must not be walkable
    // (this is what keeps the player from walking off the grid).
    #[test]
    fn out_of_bounds_is_not_walkable() {
        // e.g. an x or y >= the map's dimensions
        let can_walk = test_map().walkable(10, 10);
        assert!(!can_walk);
    }

    // Negative coordinates are off the grid too (moving left/up past the edge).
    #[test]
    fn negative_coords_are_not_walkable() {
        assert!(!test_map().walkable(-1, 0));
        assert!(!test_map().walkable(0, -1));
    }

    // --- Level parser tests ---

    // '#' parses to a Wall tile. Use a complete map (all three entities present).
    #[test]
    fn parse_wall_tile() {
        let level: Level = "#@kD".parse().unwrap();
        assert_eq!(level.map.tile(0, 0), Tile::Wall);
    }

    // '.' parses to a Floor tile.
    #[test]
    fn parse_floor_tile() {
        let level: Level = ".@kD".parse().unwrap();
        assert_eq!(level.map.tile(0, 0), Tile::Floor);
    }

    // ' ' (space) parses to an Outside tile.
    #[test]
    fn parse_outside_tile() {
        // Row 0: space at (0,0), then the three required entities.
        let level: Level = " @kD".parse().unwrap();
        assert_eq!(level.map.tile(0, 0), Tile::Outside);
    }

    // '@', 'k', and 'D' each place a Floor tile at their position.
    #[test]
    fn entity_chars_place_floor_underneath() {
        let level: Level = "@kD".parse().unwrap();
        assert_eq!(level.map.tile(0, 0), Tile::Floor); // @ -> Floor
        assert_eq!(level.map.tile(1, 0), Tile::Floor); // k -> Floor
        assert_eq!(level.map.tile(2, 0), Tile::Floor); // D -> Floor
    }

    // Entity positions are recorded at their (x, y) coordinates.
    #[test]
    fn entity_positions_are_recorded() {
        let level: Level = "@kD".parse().unwrap();
        assert_eq!(level.player, (0, 0));
        assert_eq!(level.key, (1, 0));
        assert_eq!(level.door, (2, 0));
    }

    // Rows shorter than the widest row are right-padded with Outside.
    #[test]
    fn ragged_rows_are_padded_with_outside() {
        // Row 0 is 3 wide (the widest); row 1 has only 1 char — padded to width 3.
        let level: Level = "@kD\n#".parse().unwrap();
        assert_eq!(level.map.width(), 3);
        assert_eq!(level.map.tile(1, 1), Tile::Outside); // padded
        assert_eq!(level.map.tile(2, 1), Tile::Outside); // padded
    }

    // A character not in the legend returns UnknownChar.
    #[test]
    fn unknown_char_returns_error() {
        let result = "@kD\n?".parse::<Level>();
        assert!(matches!(result, Err(ParseError::UnknownChar('?'))));
    }

    // Missing '@' returns MissingEntity("player").
    #[test]
    fn missing_player_returns_error() {
        let result = ".kD".parse::<Level>();
        assert!(matches!(result, Err(ParseError::MissingEntity("player"))));
    }

    // Missing 'k' returns MissingEntity("key").
    #[test]
    fn missing_key_returns_error() {
        let result = "@.D".parse::<Level>();
        assert!(matches!(result, Err(ParseError::MissingEntity("key"))));
    }

    // Missing 'D' returns MissingEntity("door").
    #[test]
    fn missing_door_returns_error() {
        let result = "@k.".parse::<Level>();
        assert!(matches!(result, Err(ParseError::MissingEntity("door"))));
    }

    // More than one '@' returns DuplicateEntity("player").
    #[test]
    fn duplicate_player_returns_error() {
        let result = "@@kD".parse::<Level>();
        assert!(matches!(result, Err(ParseError::DuplicateEntity("player"))));
    }

    // More than one 'k' returns DuplicateEntity("key").
    #[test]
    fn duplicate_key_returns_error() {
        let result = "@kkD".parse::<Level>();
        assert!(matches!(result, Err(ParseError::DuplicateEntity("key"))));
    }

    // More than one 'D' returns DuplicateEntity("door").
    #[test]
    fn duplicate_door_returns_error() {
        let result = "@kDD".parse::<Level>();
        assert!(matches!(result, Err(ParseError::DuplicateEntity("door"))));
    }

    // A trailing newline does not produce an extra empty row.
    #[test]
    fn trailing_newline_does_not_add_empty_row() {
        let with_newline: Level = "@kD\n".parse().unwrap();
        let without_newline: Level = "@kD".parse().unwrap();
        assert_eq!(with_newline.map.height(), without_newline.map.height());
    }

    // Guards that the bundled castle.map parses to the expected entity positions
    // and dimensions. This is the single test that makes the include_str! .expect()
    // in App::new safe in practice.
    #[test]
    fn castle_map_parses_to_expected_state() {
        let src = include_str!("../assets/castle.map");
        let level: Level = src.parse().expect("castle.map should parse");
        assert_eq!(level.player, (10, 8));
        assert_eq!(level.key, (3, 4));
        assert_eq!(level.door, (10, 9));
        assert_eq!(level.map.width(), 21);
        assert_eq!(level.map.height(), 11);
    }
}
