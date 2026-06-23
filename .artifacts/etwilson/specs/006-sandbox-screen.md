---
number: 006
story: STR-004
status: complete
base_branch: main
depends_on: [STR-002, STR-003]
scope_files:
  - src/sandbox.rs
  - src/main.rs
---

# Feature: Sandbox Screen + Entry Swap

## Summary
A standalone sandbox screen that ties the particle system, effects layer, and particle renderer into something visible the moment the binary runs. The binary temporarily auto-launches into `sandbox(terminal)` instead of the game `app(terminal)` — a deliberate, one-line, fully-reversible "park the game" swap. The sandbox owns its own `ParticleSystem` and a seeded PRNG, runs its own poll loop reusing the existing dt/tick *pattern* (not `App::tick`), auto-spawns fireworks at the screen center on a recurring cadence so there's always motion with zero input, exposes a cycle-effect control that walks the `EffectKind` list (functional even at length 1), and quits cleanly on Esc/q. This is how the whole particle epic is exercised in isolation without playing the game to trigger an effect.

---

## Requirements
- Running the binary launches directly into the sandbox; the game (`app`) does not run.
- The sandbox runs its own poll loop reusing the established pattern: `event::poll(FRAME_TIME)` → compute real `dt` via `Instant` → advance the system with `tick(dt)` → draw.
- The sandbox owns a `ParticleSystem` and a seeded PRNG; it advances the system each frame and draws live particles via the STR-003 projection over a cleared/background scene.
- Fireworks spawn automatically at the screen center on a recurring cadence, producing continuous bursting motion with no key input.
- A cycle-effect control (e.g. Tab) advances the selected `EffectKind` through the available kinds and wraps; the selection is observable even with a single kind (a title/hint line shows the current effect name).
- Esc and q exit the sandbox cleanly, returning `Ok(())` so the existing harness restores the terminal.
- Restoring the game is a single, obvious one-line change at the entry point: the game code (`app()`, `App`, game state) remains intact, compiling, and present — referenced only by being one swap away, not deleted.

---

## Scope

### In Scope
- A new `src/sandbox.rs` module exposing `pub fn sandbox(terminal: &mut DefaultTerminal) -> std::io::Result<()>`.
- The sandbox poll/dt/tick loop reusing `FRAME_TIME` and the `Instant`-based dt computation from `src/app.rs`.
- Ownership of a `ParticleSystem` (STR-001) and a seeded PRNG (STR-002) inside the sandbox.
- A recurring auto-spawn of `EffectKind::Fireworks` at the draw-area center on a fixed cadence (accumulate `dt`; spawn when the accumulator crosses a spawn interval).
- A cycle-effect control over the `EffectKind` list, with current selection tracked in sandbox-local state.
- A lightweight title/hint line showing the current effect name + controls (mirroring the game's title line), so the cycle control is observable.
- The entry swap in `src/main.rs`: `ratatui::run(...)` calls `sandbox::sandbox` instead of `app::app`; add `mod sandbox;`.
- Inline `#[cfg(test)] mod tests` covering the testable seams (center computation, cadence accumulator, effect-kind cycling, quit-key mapping).

### Out of Scope
- The `ParticleSystem`, its physics, the read surface (STR-001).
- The `EffectKind` enum, `spawn` dispatch, fireworks emit logic, per-effect params, and the PRNG type (STR-002) — the sandbox *composes* these, it does not define them.
- The particle projection/draw function (STR-003) — the sandbox *calls* it.
- Win → fireworks integration and any game-mode interaction.
- Manual click/cursor placement of effects (spawn is auto-at-center).
- Persisting sandbox state or any config/tuning surface.
- Deleting or gutting `app()`, `App`, or game state.

---

## Technical Approach

### Assumed upstream interfaces (STR-001/002/003)
These stories are being specced in parallel. The sandbox is designed against the interfaces their stories describe; implementation must align with whatever STR-001/002/003 actually land (spec 003 for STR-001 already fixes part of this). State and adapt at integration time:

- **STR-001 — `ParticleSystem` (`src/particles.rs`, per spec 003):**
  - `ParticleSystem::new()` → empty system.
  - `fn tick(&mut self, dt: Duration)` advances physics + culls.
  - `fn spawn(&mut self, particle: Particle)` injection entry (called by the effects layer, not the sandbox directly).
  - A read surface, e.g. `fn particles(&self) -> impl Iterator<Item = &Particle>` / `&[Particle]`, consumed by the renderer.
- **STR-002 — effects layer + PRNG (assumed `src/effects.rs`):**
  - `enum EffectKind { Fireworks }` (length 1 for now; `derive(Clone, Copy, PartialEq)` assumed).
  - A spawn-dispatch free function, assumed shape `spawn(kind: EffectKind, origin: (f32, f32), system: &mut ParticleSystem, rng: &mut Rng)` that routes to the effect's emit logic and injects particles into `system`. The sandbox owns `rng` and threads it in (per STR-002 notes: "a single seedable instance owned by the caller (sandbox) and threaded into spawn").
  - A seedable PRNG type, assumed `Rng` with `Rng::new(seed)` / `Rng::seed(seed)`.
  - **For cycling:** the sandbox needs the ordered list of kinds. Assume either an `EffectKind::ALL: &[EffectKind]` const or a `fn next(self) -> EffectKind` that wraps. If STR-002 provides neither, the sandbox defines a local `const EFFECT_KINDS: &[EffectKind] = &[EffectKind::Fireworks];` and indexes into it — this keeps the cycle seam honest without depending on a helper STR-002 may not expose.
- **STR-003 — particle renderer (assumed `src/particle_render.rs` or a free fn in `render.rs`):**
  - A free function taking the system read surface + a `&mut Buffer`/`Frame` + target `Rect` (and optional origin offset), assumed shape `draw_particles(frame: &mut Frame, area: Rect, system: &ParticleSystem)`. The sandbox passes its body `Rect` and the system; the renderer rounds f32 positions to cells, applies fade, and writes foreground glyphs.

> If any assumed name/signature differs at integration, adjust the sandbox call sites — the sandbox is the *consumer*, so it conforms to upstream, not the reverse. Do not redefine these types in `sandbox.rs`.

### Sandbox structure
- **Entry function:** `pub fn sandbox(terminal: &mut DefaultTerminal) -> std::io::Result<()>` — same signature as `app()` so `ratatui::run` accepts it unchanged. This is what makes the entry swap one line.
- **Local state** (loop-local `let mut`, not a struct unless it earns it): `ParticleSystem`, a seeded `Rng` (fixed seed constant for reproducibility), the selected `EffectKind` (or an index into the kinds list), a `spawn_accumulator: Duration`, and `last: Instant`.
- **Loop body** (mirrors `src/app.rs:158-181`):
  1. `terminal.draw(...)`: compute the body `Rect` (split off a 1-row title like `render::ui` does), render the title/hint line (current effect name + controls), then call the STR-003 draw fn with the body area + system. Capture the body area's center for spawning — either compute center inside the draw closure and stash it, or recompute from `frame.area()`/last-known size before drawing. **Decision:** track `last_area: Rect` updated each draw; compute center from it for the next spawn, so spawn and draw agree on the same geometry. On the very first frame before any draw, default to a sane center (e.g. spawn skipped until first area is known).
  2. Compute `dt` from `Instant::now() - last`; update `last`.
  3. Poll input (`event::poll(FRAME_TIME)?`): on key press, map the key — Esc/q → `return Ok(())`; cycle key (Tab) → advance selected `EffectKind`; ignore others.
  4. Auto-spawn: `spawn_accumulator += dt`; while `spawn_accumulator >= SPAWN_INTERVAL`, call the effects `spawn(selected_kind, center, &mut system, &mut rng)` and subtract `SPAWN_INTERVAL`.
  5. `system.tick(dt)`.
- **Constants:** reuse `FRAME_TIME` from `app.rs` (import it). Add `SPAWN_INTERVAL: Duration` (sandbox-local; ~`Duration::from_millis(700)`–`1000` is fine — observable requirement is continuous motion, exact value is the architect's/implementer's call) and a fixed PRNG seed constant.

### Center computation
Particles are cell-space, so center ≈ `(area.width / 2, area.height / 2)` as `f32`, offset by the area origin: `(area.x as f32 + area.width as f32 / 2.0, area.y as f32 + area.height as f32 / 2.0)`. This mirrors the centering sense of `map_origin` in `render.rs` but is simpler (no map dimensions). Expose this as a small pure helper `fn area_center(area: Rect) -> (f32, f32)` so it's unit-testable.

### Cycle seam
Track selection as an index `usize` into the kinds list (local `EFFECT_KINDS` const if STR-002 exposes no `ALL`/`next`). Cycle = `idx = (idx + 1) % EFFECT_KINDS.len()`. With length 1 this is a no-op on the value but still exercises the dispatch path; the title line reads the current kind's name so the control is observable (selection "wraps" visibly even at length 1). Expose `fn next_kind(idx: usize, len: usize) -> usize` as a pure testable helper.

### Entry swap
`src/main.rs`: change `ratatui::run(app::app)?;` → `ratatui::run(sandbox::sandbox)?;` and add `mod sandbox;`. Leave `mod app;` in place (its public items are still referenced by the test suite and the module must keep compiling). The `app` module being unused by `main`'s runtime path is expected; if clippy/rustc flags `app` as dead-code at the binary level, suppress narrowly (e.g. `#[allow(dead_code)]` on the parked entry, or keep it referenced via tests) rather than deleting anything. Add a brief comment at the swap site noting it's a temporary detour and the one-line restore.

---

## Success Criteria
- [ ] `cargo build` and `cargo clippy` pass clean with `mod sandbox;` wired in and the entry pointed at `sandbox`.
- [ ] Running the binary launches the sandbox; the castle/game does not render.
- [ ] With no key input, fireworks burst repeatedly at the screen center — particles rise/fan out, fall under gravity, fade, and disappear (end-to-end across system + effects + rendering).
- [ ] The title/hint line shows the current effect name and controls; pressing the cycle key updates the displayed selection (observably wraps, even with one kind).
- [ ] Esc and q both exit cleanly; the terminal is restored (no garbled state).
- [ ] `app()`, `App`, and game state remain present and compiling; restoring the game is a single-line change at the `ratatui::run(...)` call.
- [ ] `cargo test` passes; the pure helpers (`area_center`, `next_kind`, spawn-cadence accumulator logic, quit/cycle key mapping) are covered deterministically.

---

## Tasks
Ordered by dependency. TDD per the project workflow — scaffold, fail, implement. STR-001/002/003 must be merged (or their interfaces final) before integration tasks; pure-helper tasks can proceed against assumed signatures.

- [ ] **Module scaffold + pure helpers (RED→GREEN):** Create `src/sandbox.rs` with the `sandbox` fn signature (stub returning `Ok(())`), `SPAWN_INTERVAL`/seed constants, and an inline `#[cfg(test)] mod tests`. Write failing tests for the pure seams — `area_center(rect)` (center math incl. area offset), `next_kind(idx, len)` (wraps, handles len 1), and a quit/cycle key-mapping helper (Esc/q → quit, Tab → cycle, others → ignore). Implement the helpers. Add `mod sandbox;` to `src/main.rs`. Confirm `cargo build` + helper tests pass. Must be green before the loop is wired.
- [ ] **Spawn cadence accumulator (RED→GREEN):** Write a failing test for the accumulator logic — given a `dt` sequence, it fires the correct number of spawns and carries the remainder (`while accumulator >= SPAWN_INTERVAL { spawn; accumulator -= SPAWN_INTERVAL }`). Factor the "how many spawns this frame" decision into a testable pure helper so it doesn't require a running terminal. Implement and verify.
- [ ] **Loop integration (no terminal test):** Wire the full poll/dt/tick loop in `sandbox()` — draw title + body, call the STR-003 particle draw fn with the body area + system, compute `dt`, handle input (quit/cycle), run the cadence spawn via the STR-002 `spawn` dispatch threading the owned `Rng`, then `system.tick(dt)`. Integrate against the real STR-001/002/003 interfaces (adapt call sites to their actual names/signatures). No unit test for the loop itself (IO-bound); rely on the helper tests + manual run.
- [ ] **Entry swap + parked game (GREEN):** Point `ratatui::run(...)` at `sandbox::sandbox` in `src/main.rs`, add the temporary-detour comment, and keep `mod app;`. Resolve any dead-code lint on the parked `app` path with a narrow `#[allow(...)]` rather than deletion. Confirm `cargo build`, `cargo clippy`, and `cargo test` are all green and that running the binary shows continuous fireworks at center.

---

## Considerations
- **Reuse the pattern, not `App::tick`:** epic decision (a) — the sandbox advances its *own* `ParticleSystem` via `system.tick(dt)`, never `App::tick`. Do not route sandbox state through `App`.
- **Center must track the live draw area:** the terminal can be resized; compute the spawn center from the most recent body `Rect` (track `last_area`), not a fixed constant, so bursts stay centered after a resize. Guard the first frame (no area yet) so an initial spawn doesn't fire at a bogus center.
- **Geometry agreement:** the renderer (STR-003) and the spawn center must use the same area. Pass the same body `Rect` to both the draw fn and `area_center`.
- **Coordinate sense:** gravity is positive-y (screen y grows downward) per spec 003 — spawning at center and letting fireworks fan out then fall is consistent; no flip needed.
- **`FRAME_TIME` source:** import the existing `crate::app::FRAME_TIME` rather than redefining, to keep one source of truth for frame pacing. (It's currently `const FRAME_TIME` in `app.rs`; if it isn't `pub`, make it `pub(crate)` as a trivial enabling change — note this is the only edit to `app.rs`'s surface and it does not alter game behavior.)
- **Deterministic PRNG seed:** use a fixed seed constant so the sandbox is reproducible run-to-run (aids any future visual diffing); randomness-of-appearance comes from the PRNG sequence, not from seeding entropy.
- **Parked game / dead-code:** `app` will be unreferenced from `main`'s runtime path. Keep it compiling; its `#[cfg(test)]` tests still run. Prefer a narrow `#[allow(dead_code)]` over `pub` churn if the linter complains — the goal is reversibility, so minimize edits to `app.rs`.
- **Don't over-engineer:** per the vision, keep the sandbox simple — loop-local state over a struct unless a struct genuinely reads better; one title line; no config surface.

---
