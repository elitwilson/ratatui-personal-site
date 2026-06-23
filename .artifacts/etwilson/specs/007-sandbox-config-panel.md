---
number: 007
story: null   # ad-hoc spec — interactive sandbox tooling, not decomposed from a story
status: complete
base_branch: feature/epic-001-particle-effects-20260622
depends_on: []
scope_files:
  - src/sandbox.rs
  - src/sandbox_config.rs
  - src/main.rs
---

# Feature: Sandbox config panel

## Summary
Adds an in-sandbox configuration overlay so fireworks parameters can be tuned live, without editing code and recompiling. Pressing `c` toggles a bordered Ratatui panel listing the adjustable knobs; the user selects a field with `↑/↓` and steps its value with `←/→`. Changes apply immediately — the sandbox keeps simulating behind the panel, so the effect of each adjustment is visible in real time. This is dev-facing tooling for the particle sandbox (which currently auto-spawns fireworks that follow the mouse); it makes the existing `FireworksParams` and spawn cadence runtime-mutable instead of hardcoded.

---

## Requirements
- Pressing `c` while the sandbox is running opens the config panel; pressing `c` or `Esc` while it is open closes it.
- While the panel is open, key input is routed to panel navigation: `↑/↓` moves the field selection (wrapping at the ends), `←/→` decrements/increments the selected field's value by a per-field step. `q`/`Tab` do NOT perform their normal sandbox actions while the panel is open (they are inert or reserved for the panel).
- The panel displays every adjustable field with its current value, and visually highlights the currently selected field.
- The following fields are adjustable: `count`, `spread`, `speed_min`, `speed_max`, `lifetime_min`, `lifetime_max`, `spawn_interval`.
- Adjustments apply live: the next auto-spawned burst (and the spawn cadence) reflects the current values with no restart.
- Each field is clamped to a sane range so values cannot become invalid (no zero/negative counts, no negative speeds/lifetimes, no zero spawn interval).
- The paired range fields maintain their invariant: `speed_min ≤ speed_max` and `lifetime_min ≤ lifetime_max` at all times, even as either side is stepped.
- While the panel is open, the sandbox continues to render and auto-spawn (live preview); the mouse-follow spawn origin continues to work.

---

## Scope

### In Scope
- A `SandboxConfig` runtime state holding the mutable `FireworksParams` plus the spawn interval, replacing the once-constructed `FireworksParams::default()` and the `SPAWN_INTERVAL` const in `sandbox.rs`.
- A `ConfigField` model + an ordered field list + pure step/clamp logic.
- Ratatui rendering of the config overlay (bordered block, field rows, selection highlight).
- Wiring into the existing sandbox loop: `c` toggle, a config-open mode that reroutes key handling, and reading config values live each frame for spawn + cadence.
- Unit tests for the pure step/clamp/selection logic.

### Out of Scope
- **Gravity** as a knob. The `FireworksParams.gravity` field is inert and the real gravity is `const GRAVITY` in `particles.rs`; exposing it requires resolving that duplication and the deferred global-vs-per-effect design decision. Left exactly as-is — deliberately deferred (see EPIC-001 Scope Out: runtime param tuning).
- **Palette and glyph** editing (color/character editing in a TUI is fiddly and low-value for v1).
- Persisting config to disk / loading config from a file.
- Any change to `particles.rs` or `effects.rs` — this spec is contained to the sandbox. The `FireworksParams` fields it mutates already exist and are public.
- Mouse interaction with the panel (panel is keyboard-driven; mouse continues to drive spawn origin only).

---

## Technical Approach

- **Entry point / interface:** the existing `sandbox()` loop in `src/sandbox.rs`. A new `config_open: bool` state gates input routing. The existing `map_sandbox_key` handles the closed-panel case (add a `ToggleConfig` command for `c`); a separate pure mapping handles the open-panel case.

- **Key modules / components:**
  - New `src/sandbox_config.rs` — owns `SandboxConfig` (the mutable params + interval), the `ConfigField` enum, the ordered `FIELDS` list, selection index, and the pure `step`/clamp logic. All unit-tested here. (Keep the heavy logic out of `sandbox.rs`.)
  - `src/sandbox.rs` — owns the loop integration and the panel *rendering* (thin; reads `SandboxConfig`). Registers the new module's input mapping.
  - `src/main.rs` — add `mod sandbox_config;`.

- **Data model:**
  - `SandboxConfig { params: FireworksParams, spawn_interval: Duration, selected: usize }` (or selection tracked separately — implementer's choice, but selection must be addressable).
  - `enum ConfigField { Count, Spread, SpeedMin, SpeedMax, LifetimeMin, LifetimeMax, SpawnInterval }` with a `const FIELDS: &[ConfigField]` defining display + navigation order.
  - A pure `fn step(config: &mut SandboxConfig, field: ConfigField, dir: i32)` where `dir` is `+1`/`-1`, applying the per-field increment and clamping. Suggested steps/clamps (tune for feel): count ±5 (min 1), spread ±(π/12) clamped to `[0, TAU]`, speed ±1.0 (min 0.0), lifetime ±0.1 (min 0.1), spawn_interval ±100ms (min 50ms). After stepping a paired field, re-establish `min ≤ max`.

- **Key design decisions:**
  - **Self-contained to the sandbox.** No edits to `particles.rs`/`effects.rs`; the panel mutates already-public `FireworksParams` fields. This keeps the spec independent and conflict-free.
  - **Live-applied, no apply/commit step.** The loop reads `config.params` and `config.spawn_interval` every frame, so edits take effect on the next spawn/cadence tick. Simplest model and best for a tuning tool.
  - **Two input maps, one mode flag.** Rather than a complex state machine, `config_open` selects which pure key-mapping function runs. Mirrors the existing `map_sandbox_key` seam (pure, testable) the sandbox already uses.
  - **Ranges as two scalar fields.** `speed`/`lifetime` are edited as independent min/max fields with an ordering invariant, avoiding a paired-range editing widget.

---

## Success Criteria
- [ ] Pressing `c` opens the panel; pressing `c` or `Esc` closes it; the sandbox keeps rendering throughout.
- [ ] The panel lists all seven fields with current values and highlights the selected one; `↑/↓` moves the highlight (wrapping).
- [ ] `←/→` on `count` changes the number of particles in the next burst (observable as denser/sparser fireworks).
- [ ] `←/→` on `spawn_interval` changes how frequently bursts fire (observable cadence change), clamped so it never reaches zero.
- [ ] Stepping `speed_min` above the current `speed_max` (or `lifetime_min` above `lifetime_max`) does not break the invariant — the values stay ordered.
- [ ] Clamps hold: `count` cannot go below 1, speeds/lifetimes cannot go negative, `spawn_interval` cannot reach 0.
- [ ] `q`/`Tab` do not quit or cycle while the panel is open.
- [ ] Unit tests cover step increments, clamps, the min/max invariant, and selection wrapping.

---

## Tasks
- [ ] **Config state + step/clamp logic:** Create `src/sandbox_config.rs` with `SandboxConfig`, `ConfigField`, the `FIELDS` order, selection movement (with wrap), and the pure `step`/clamp logic including the min/max invariant. Register `mod sandbox_config;` in `main.rs`. Unit-test all of it before wiring. Must be tested before the next task.
- [ ] **Panel rendering:** In `sandbox.rs`, render the config overlay — a centered bordered `Block` titled "Config" with one row per field (`label: value`) and the selected row highlighted. Reads `SandboxConfig`; no mutation. (Mirror the existing title/`Block` rendering style already in `sandbox.rs`.)
- [ ] **Loop integration:** Replace the `FireworksParams::default()` binding and `SPAWN_INTERVAL` const with a mutable `SandboxConfig`. Add `ToggleConfig` to the closed-panel key map and a separate open-panel key map (`↑/↓` select, `←/→` step, `c`/`Esc` close). Gate input routing on `config_open`. Have the spawn call and cadence read live from the config. Render the panel when open.
- [ ] **Smoke / integration check:** Run the sandbox (`cargo run`), open the panel with `c`, adjust `count` and `spawn_interval`, and confirm the running fireworks visibly change (density + cadence) and that `q`/`Tab` are inert while open and work again after closing. A green unit suite is not sufficient evidence the panel is wired to the live loop — verify end-to-end.

---

## Considerations
- **`base_branch` is the feature branch, not `main`.** `sandbox.rs` (and the whole particle epic) lives only on `feature/epic-001-particle-effects-20260622`. Building from `main` would have no sandbox to extend.
- **Event draining already exists.** The sandbox loop drains all pending events each frame (for mouse-follow). New key handling slots into that same drain loop — don't reintroduce single-event-per-frame reads.
- **Two pre-existing tests are `#[ignore]`d** (`castle_map_parses_to_expected_state`, `app_initial_state`) because `castle.map` is WIP and out of scope. Leave them ignored; a green run is `cargo test` with those two ignored.
- **`spawn_interval` of 0 must be impossible.** `cadence_step` divides by the interval (guards zero by returning no spawns), but a zero interval would silently stop spawning — the clamp (min 50ms) is what prevents a confusing "fireworks stopped" state.
- **Follow the existing inline test convention** in `sandbox.rs` (`#[cfg(test)] mod tests { … }`) for consistency, even though the project's general guidance prefers sibling test files.
- **Mouse capture stays enabled** while the panel is open; the panel is keyboard-only but mouse-follow spawning should keep working underneath so live preview lands where the cursor is.
