---
number: 001
story: null   # ad-hoc spec — drafted directly, not decomposed from a story
status: ready
base_branch: map-and-walkable
depends_on: []
scope_files:
  - src/map.rs
  - src/app.rs
  - assets/castle.map
---

# Feature: Text/ASCII map format

## Summary
Levels are currently authored as `vec![W, W, O, ...]` token grids in
[map.rs](../../../src/map.rs), with player/key/door positions hardcoded
separately in [`App::new`](../../../src/app.rs). Editing a level means
counting commas, aligning single-char tokens by eye, and hand-syncing entity
coordinates against the grid. This feature replaces that with **ASCII-art map
files**: a level is drawn as a block of text that looks like the map, with
entity positions living in the art itself. A parser turns that text into the
existing `Map` plus the three entity positions. The castle is converted to this
format, stored as `assets/castle.map`, baked into the binary via `include_str!`,
and loaded at startup. The author's only interaction is editing a `.map` text
file; the running game is visually and behaviorally unchanged.

---

## Requirements
- A map can be authored as a block of text using this legend:
  - `#` → Wall
  - `.` → Floor
  - (space) → Outside (the void)
  - `@` → Player, standing on a Floor tile
  - `k` → Key, on a Floor tile
  - `D` → Door, on a Floor tile
- Parsing produces a `Level`: the `Map` plus player, key, and door positions.
- Entity characters (`@`/`k`/`D`) yield a `Floor` tile underneath and record the
  entity's `(x, y)` position.
- Rows shorter than the widest row are right-padded with Outside, so hand-edited
  art need not be space-perfect on every line.
- Parsing fails (returns an error, does not panic) when the text contains:
  - a character outside the legend,
  - zero or more than one of any required entity (`@`, `k`, or `D`).
- Exactly one `@`, one `k`, and one `D` are required for a successful parse.
- The castle, expressed in this format at `assets/castle.map`, parses to the
  same starting state the game has today: player `(10, 8)`, key `(3, 4)`,
  door `(10, 9)`, and an identical wall/floor/outside layout.
- The game's runtime behavior (movement, collision, key pickup, door/about
  reveal) is unchanged.

---

## Scope

### In Scope
- A `Level` struct bundling `Map` + player/key/door positions.
- A parser (`Level::from_str` / `FromStr`) implementing the legend, padding, and
  validation above.
- A `ParseError` type covering: unknown character, missing entity, duplicate
  entity.
- `assets/castle.map` — the current castle as ASCII art.
- Rewiring `App::new` to load `assets/castle.map` via `include_str!` and parse it
  into the `App`'s map + entity positions.
- Removing the `vec![...]`-based `castle()` builder once the format replaces it.
- New parser unit tests, added **inline** in map.rs's existing
  `#[cfg(test)] mod tests` block (matching the codebase's current convention).

### Out of Scope
- Serialization (`Level` → text). Only reading text → `Level` is built here.
- Any interactive in-game level designer / editor UI.
- Multiple maps or level switching (vision defers this to v2).
- Loading `.map` files from disk at runtime — the map is `include_str!`'d at
  build time so it also works on the WASM target (no runtime filesystem).
- New tile or entity types beyond the existing three tiles + three entities.
- Migrating the existing test fixtures (`demo_castle()`, `test_map()`) — they
  remain token grids. They may only be touched if a test genuinely cannot
  otherwise exercise correct behavior.

---

## Technical Approach
- **Entry point:** `App::new()` keeps its `-> Self` signature and the
  no-argument public surface. Internally it does
  `include_str!("../assets/castle.map")`, parses it into a `Level`, and
  populates `map`, `player_pos`, `key_pos`, `door_pos`. The
  [`app()`](../../../src/app.rs#L143) entry loop and the renderer are untouched.

- **Error handling at startup:** the castle map is a compile-time constant, so a
  parse failure there is a bug, not a runtime condition. `App::new` resolves the
  parse with `.expect("built-in castle map should parse")` (documented
  invariant). The fallible surface is the parser itself
  (`Result<Level, ParseError>`); a dedicated test guards that the bundled castle
  parses, so the `expect` can never fire in practice.

- **Key modules:**
  - map.rs owns `Tile`, `Map`, the new `Level`, `ParseError`, and the parser.
    `Level` is map-domain, so it lives here alongside `Map`.
  - app.rs's `App::new` is the only consumer change.

- **Data model:**
  ```rust
  pub struct Level {
      pub map: Map,
      pub player: (u16, u16),
      pub key: (u16, u16),
      pub door: (u16, u16),
  }

  pub enum ParseError {
      UnknownChar(char),
      MissingEntity(&'static str),   // "player" | "key" | "door"
      DuplicateEntity(&'static str),
  }
  ```
  (Exact error variant shape is the implementer's call as long as the three
  failure modes are distinguishable and tested.)

- **Parsing algorithm:** split on `\n`; compute `width` as the longest row;
  for each row, map each char per the legend into a `Tile`, padding the row out
  to `width` with `Outside`; when an entity char is seen, place `Floor` and
  record its `(x, y)`; reject unknown chars; after the full pass, require exactly
  one of each entity. Build the `Map` via the existing `Map::new(rows)`.

- **Key design decisions:**
  - *Entities sit on floor* — keeps every entity on walkable ground and keeps the
    grid valid by construction.
  - *`Level` bundles map + positions* — the text file is the single source where
    all four are defined together; threading them separately would re-introduce
    the sync problem this feature removes.
  - *`include_str!` over `std::fs`* — editable as a plain file yet WASM-safe.
  - *Fail fast on malformed art* — a typo should error at parse, not silently
    yield a broken level.

---

## Success Criteria
- [ ] A `Level::from_str` parses the legend into `Map` + player/key/door
      positions, with entity tiles resolving to `Floor`.
- [ ] Ragged (short) rows are padded with Outside to the widest row's width.
- [ ] Parsing an unknown character returns a `ParseError` (no panic).
- [ ] Parsing with a missing or duplicated `@`/`k`/`D` returns a `ParseError`.
- [ ] `assets/castle.map` exists, is hand-editable ASCII art, and parses to
      player `(10, 8)`, key `(3, 4)`, door `(10, 9)` with a layout matching the
      old `castle()`.
- [ ] `App::new()` builds its state from `assets/castle.map`; the app launches
      and plays identically to before (movement, key pickup, door → about).
- [ ] No `vec![...]` map literal remains in non-test code (`castle()` removed).
- [ ] All pre-existing tests still pass.

---

## Tasks
Ordered by dependency.

- [ ] **Parser + `Level`/`ParseError` (RED → GREEN):** Scaffold failing tests
      first (per TDD workflow), then implement. Add `Level`, `ParseError`, and
      the parser to map.rs. Tests go inline in the existing
      `#[cfg(test)] mod tests`, driven by small inline `&str` fixtures (not the
      real castle), covering: the three tile chars; entity chars recording
      positions over Floor; ragged-row padding; unknown char → error; missing
      entity → error; duplicate entity → error. Must be unit-tested and green
      before the next task.
- [ ] **Author `assets/castle.map`:** Translate the current `castle()` grid into
      ASCII art, placing `@` at (10,8), `k` at (3,4), `D` at (10,9), preserving
      the exact wall/floor/outside layout. Add an inline test that
      `include_str!("../assets/castle.map")` parses and yields exactly those
      three positions and dimensions — this guards the conversion. Depends on the
      parser task.
- [ ] **Rewire `App::new` and remove `castle()`:** Change `App::new` to
      `include_str!` + parse `assets/castle.map` into `map`/`player_pos`/
      `key_pos`/`door_pos` using `.expect(...)`; delete the `castle()` builder
      and its import. The existing `app_initial_state` test (asserting player
      `(10,8)`, no key, etc.) must still pass unchanged. Depends on the castle
      file.
- [ ] **End-to-end wiring check:** Run `cargo test` (all pre-existing +
      new tests green) and launch the app (`cargo run`) to confirm it starts from
      the parsed map and the move → key → door → about loop still works. A green
      suite alone is not sufficient evidence the binary is wired — confirm it
      actually runs.

---

## Considerations
- **Door tile underneath.** In today's grid the door sits on the Floor at
  `(10, 9)` in an otherwise-wall row; in the art that cell becomes `D`, which the
  parser resolves to Floor. Verify the surrounding wall row is otherwise intact
  after conversion.
- **Trailing newline / final row.** A trailing `\n` in the file would produce an
  empty final row; decide deterministically (e.g. ignore a single trailing empty
  line, or `trim_end_matches('\n')`) so `Map` height stays correct. Pick one and
  test it implicitly via the castle's height.
- **Leading/embedded spaces are significant** — space means Outside, so the art
  cannot be re-indented. Note this near the file (a comment in the `.map` is not
  possible under the legend, so document the legend in a doc-comment at the
  `include_str!` site or in `Level`'s docs).
- **Width derivation.** `Map::new` derives width from the first row; padding must
  happen *before* `Map::new` so the first row already equals the max width
  (otherwise a short first row would truncate the derived width).
- **No new dependencies** — `include_str!` is std; parsing is hand-rolled over
  `&str`. Keep it dependency-free, consistent with the current map module.
