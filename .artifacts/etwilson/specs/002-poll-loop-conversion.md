---
number: 002
status: ready
base_branch: main
depends_on: []
scope_files:
  - src/app.rs
---

# Feature: Poll-based event loop with a tick seam

## Summary
The game's event loop currently blocks on `event::read()` — nothing advances until the user presses a key. That's correct for a turn-based game but makes time-based animation (e.g. a future "you won" particle effect) impossible, because there are no frames between keypresses. This spec converts the loop in `app()` from blocking to **poll-based**: it waits up to one frame's worth of time for input, handles input if any arrived, then calls a new `App::tick(dt)` hook and redraws — every iteration, on a steady cadence. This is pure infrastructure. The game plays identically; the only change is that the loop now runs continuously and exposes a per-frame seam for future animation to plug into.

---

## Requirements
- The event loop waits at most one frame interval for input rather than blocking indefinitely.
- Keyboard input drives the game exactly as it does today — movement, about-toggle, and quit behave identically and feel responsive (`q`/`Esc` quits without perceptible delay).
- Every loop iteration calls `App::tick(dt)`, where `dt` is the real elapsed wall-clock time since the previous iteration, then redraws — whether or not an input event arrived that iteration.
- `App::tick(dt)` exists as a no-op seam in this spec. It must accept the elapsed `Duration` and currently do nothing. It exists so future animation work has a per-frame hook; it deliberately changes no state here.
- The wall-clock timekeeping (`Instant`/`dt` computation) lives in the loop, not in `App`. `App::tick` receives `dt` as a parameter and never reads the clock itself.
- Non-`Press` key events (key release/repeat) must not skip the per-frame `tick`/redraw.

---

## Scope

### In Scope
- `src/app.rs` only:
  - Convert the loop in `app(terminal)` from `event::read()` to `event::poll(timeout)` + conditional `event::read()`.
  - Add a frame-interval constant (~16ms / ~60fps).
  - Add `Instant`-based `dt` bookkeeping in the loop.
  - Add the `App::tick(&mut self, dt: Duration)` no-op seam.

### Out of Scope
- **Any particle/animation logic or `App` animation state.** The `tick` body stays empty. The future particle-effects spec adds its own state and fills in `tick`.
- **Any new `App` fields** (no `elapsed` counter, no frame count, no game-phase enum). This spec adds the `tick` method signature and nothing else to `App`'s data.
- **Changes to `src/render.rs`.** The renderer is untouched; it already observes `App` and redrawing every frame is fine (Ratatui diffs the buffer, so idle redraws write almost nothing).
- **Adaptive frame rate / idle throttling.** The loop polls at a constant cadence. Throttling when idle is a later optimization that won't change this architecture.
- **`dt` capping / spiral-of-death protection.** Not needed for a toy; out of scope.

---

## Technical Approach
- **Entry point:** `pub fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()>` in [src/app.rs](src/app.rs).
- **Loop shape** (replacing the current `loop` body):
  1. `terminal.draw(|frame| render::ui(frame, &app))?`
  2. Compute `dt`: `let now = Instant::now(); let dt = now - last; last = now;` (initialize `last` to `Instant::now()` just before the loop).
  3. `if event::poll(FRAME_TIME)? { if let Event::Key(key) = event::read()? { ... } }` — the existing key-handling match goes inside, unchanged in behavior.
  4. `app.tick(dt);` — always runs, after the optional input handling.
- **Critical restructure:** the current code uses `if key.kind != KeyEventKind::Press { continue; }`. `continue` would jump past the `tick` call at the bottom of the loop. Replace it with a positive condition (e.g. only `match map_key(...)` when `key.kind == KeyEventKind::Press`) so non-Press events fall through to `tick`/redraw instead of skipping them. Quitting still returns out of the function via the existing `Command::Quit` arm.
- **New constant:** `const FRAME_TIME: Duration = Duration::from_millis(16);` (~60fps).
- **New method:**
  ```rust
  /// Per-frame hook called once per loop iteration with the real elapsed
  /// time since the last frame. No-op today; the seam exists so future
  /// time-based animation has somewhere to live.
  pub fn tick(&mut self, _dt: Duration) {}
  ```
- **Imports:** add `std::time::{Duration, Instant}`. `event::poll` is already reachable via the existing `crossterm::event` import.
- **Key design decision:** `dt` is computed in the loop and passed into `tick`, keeping `App` clock-agnostic and free of IO — so when `tick` eventually gains real logic, it's unit-testable by calling `tick(some_duration)` directly with no terminal or wall clock involved.

---

## !! Testing approach — read before writing any tests !!

**This is a wiring change. There are NO new unit tests, and that is correct and intended. Do not treat the absence of unit tests as a gap to fix.**

- The only new logic is the `app()` loop, which is pure IO — it calls `terminal.draw`, `event::poll`, `event::read`, and `Instant::now`. None of that is unit-testable without a live terminal and real input, and it must not be refactored, abstracted, or wrapped solely to manufacture testability. Do not introduce traits, injected clocks, fake event sources, or extracted "pure" helpers just so something can be asserted. The wiring is the deliverable; verify it by running the app, not by unit-testing it.
- `App::tick(dt)` is an intentional no-op in this spec. **Do not write a test for it** — there is no behavior to assert. A test like "calling tick does nothing" is noise; do not add it. Its real tests arrive with the future particle spec when it gains behavior.
- The existing test suite in `src/app.rs` (input mapping, movement, key/door logic) must continue to pass unchanged. Do not modify those tests. If the loop restructure is done correctly, none of them are affected (they exercise `update`/`map_key`/pure state, never the loop).
- **Verification for this spec is the smoke test (final task), not `cargo test`.** A green `cargo test` only proves the untouched logic still works; it says nothing about the loop wiring. The smoke test is the actual evidence this feature works.

If you (coder or reviewer) feel blocked because there's "nothing to unit-test," that is the expected state for this spec — proceed to implementation and rely on the smoke test. Do not stall the TDD cycle hunting for a unit to test.

---

## Success Criteria
- [ ] `app()` uses `event::poll(FRAME_TIME)` + conditional `event::read()`; the blocking `event::read()` is gone.
- [ ] `App::tick(&mut self, dt: Duration)` exists, is called exactly once per loop iteration, and is a no-op.
- [ ] `dt` passed to `tick` is the real elapsed time between iterations, computed in the loop via `Instant`.
- [ ] Non-`Press` key events fall through to `tick`/redraw instead of being skipped via `continue`.
- [ ] No new fields added to `App`; `src/render.rs` unchanged.
- [ ] Existing `src/app.rs` test suite passes unchanged (`cargo test`), and `cargo clippy` is clean.
- [ ] Smoke test: the app launches, the player moves with WASD/arrows, `i` toggles About, and `q`/`Esc` quits promptly with no perceptible input lag.

---

## Tasks
Ordered by dependency.

- [ ] **Add the `tick` seam.** Add `use std::time::{Duration, Instant};` and the `FRAME_TIME` constant. Add `pub fn tick(&mut self, _dt: Duration) {}` to the `impl App` block with the doc comment from Technical Approach. No tests (see Testing approach). This compiles cleanly on its own before the loop is touched.
- [ ] **Convert the loop in `app()`.** Replace blocking `event::read()` with the poll-based shape from Technical Approach: draw → compute `dt` → `if event::poll(FRAME_TIME)?` guard around `event::read()` and the existing key match → `app.tick(dt)` at the bottom. Restructure the `KeyEventKind::Press` filter from `continue` to a positive condition so non-Press events still reach `tick`/redraw. Preserve the exact `Quit`/`Game(action)` handling. Depends on the previous task.
- [ ] **Verify existing suite + lint.** Run `cargo test` (existing tests must pass unchanged) and `cargo clippy` (clean). No new tests are added; this task confirms nothing regressed.
- [ ] **Smoke test (required evidence).** Build and run the binary. Confirm: window renders the castle, player moves with WASD and arrows, `i` opens/closes About, `q` and `Esc` quit promptly. Confirm input feels responsive (no lag from the poll timeout). This — not the test suite — is the evidence the conversion works end-to-end.

---

## Considerations
- **The `continue` trap is the one real bug risk.** It's easy to keep the existing `if key.kind != KeyEventKind::Press { continue; }` and silently break the per-frame `tick` cadence. The restructure to a positive condition is the load-bearing detail of the loop conversion — reviewers should check this specifically.
- **Idle CPU is acceptable and intentional.** The loop wakes ~60×/sec even when nothing is happening. Ratatui's buffer diffing means an unchanged frame writes almost nothing, so this is cheap. Do not add idle-throttling to "fix" it — that's explicitly out of scope and a later optimization.
- **`dt` will be roughly `FRAME_TIME` plus handling time**, and larger if a redraw stalls. That's fine; the no-op `tick` ignores it. Capping `dt` is deliberately deferred to whenever real animation needs frame-rate independence.
- **Why a no-op seam now instead of deferring it to the particle spec:** isolating the loop conversion keeps that future spec focused purely on particle logic plugging into an existing hook, rather than mixing infrastructure and feature work in one change.
