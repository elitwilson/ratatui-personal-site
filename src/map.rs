#[derive(Copy, Clone, PartialEq, Eq)]
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

    pub fn walkable(&self, x: u16, y: u16) -> bool {
        // Guard the bounds first, or the index below would panic.
        if x >= self.width as u16 || y >= self.height as u16 {
            return false;
        }
        matches!(self.tiles[y as usize][x as usize], Tile::Floor)
    }
}

/// A small placeholder castle, used until the real map is authored.
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
}
