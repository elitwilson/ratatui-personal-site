---
number: 004
story: STR-002
status: complete
base_branch: main
depends_on: ["STR-001"]
scope_files:
  - src/rng.rs
  - src/rng/tests.rs
  - src/effects.rs
  - src/effects/tests.rs
  - src/main.rs
---

# Feature: Effects layer + fireworks (+ PRNG)

## Summary
This feature adds the layer that turns a high-level "spawn this effect here" request into a configured burst of particles injected into the `ParticleSystem` from STR-001. It introduces an `EffectKind` enum (one variant, `Fireworks`), a `spawn(kind, origin, params, rng, system)` dispatch seam, a per-effect parameter struct (`FireworksParams`) with hardcoded defaults, and the fireworks emit logic that fans N particles outward with randomized velocities, colors, and lifetimes. The randomness comes from a small hand-rolled, seedable PRNG (`Rng`) — no `rand` crate. The effects layer depends on the particle system, never the reverse: it only calls the system's injection entry, it does not own or simulate particles.

---

## Requirements
- `spawn(EffectKind::Fireworks, origin, ...)` injects a burst of multiple particles into a `ParticleSystem` at/around `origin`.
- Emitted particles have varied velocities — randomization is actually applied, not a fixed pattern. Particles fan outward (radial-ish burst) so that after gravity they fall in an arc.
- The PRNG is deterministic per seed: the same seed produces the same `u32`/`f32` sequence, so a seeded fireworks spawn produces a byte-reproducible burst.
- Fireworks parameters are a struct (`FireworksParams`) with a `Default` impl; changing a param (e.g. particle count) changes the emitted burst accordingly.
- Dispatch goes through `EffectKind`: adding a future variant is adding an enum arm plus an emit function, with no change to the shape of the `spawn` call site.
- The PRNG exposes `next_u32`, `next_f32` (in `[0, 1)`), and a float range helper, and is seedable via an explicit constructor.

---

## Scope

### In Scope
- `Rng` struct (xorshift) with seedable constructor and `next_u32` / `next_f32` / `range_f32` helpers, in `src/rng.rs`.
- `EffectKind` enum (`Fireworks` only) and a free `spawn` dispatch function in `src/effects.rs`.
- `FireworksParams` struct with a `Default` impl carrying hardcoded defaults (particle count, color palette, angular spread, speed range, gravity hint, lifetime range).
- The fireworks emit function: given `origin`, `params`, an `&mut Rng`, and an `&mut ParticleSystem`, emit N randomized particles into the system.
- Threading a caller-owned `&mut Rng` into `spawn` (no global PRNG state).
- Registering the new modules in `src/main.rs`.
- Tests for the PRNG (determinism, range bounds) and the effects layer (burst count, velocity variation, param-driven count, seed reproducibility).

### Out of Scope
- The `ParticleSystem` itself and its physics (STR-001).
- Rendering / projection to the cell grid (STR-003).
- The sandbox loop, repeat-spawn cadence, and cycle-effect UI (STR-004).
- Additional effect types beyond fireworks (enum stays length 1).
- Config-file / runtime tuning of params (in-code only).
- Wiring fireworks into the actual game win condition.

---

## Technical Approach

- **Entry point / interface:** `pub fn spawn(kind: EffectKind, origin: (f32, f32), params: &FireworksParams, rng: &mut Rng, system: &mut ParticleSystem)`. The call site passes an `EffectKind`; `spawn` matches on it and delegates to the per-effect emit function. (Single param type for now since there is one effect; the per-effect param struct is the documented extension point — when a second effect arrives, params become per-variant, e.g. an enum or per-arm argument. The seam that matters today is that each effect owns its own param struct and dispatch routes by `EffectKind`.)

- **Assumed STR-001 interface (design against this; align at implementation):** STR-001 is being specced in parallel and does not exist yet. This spec assumes:
  - A `ParticleSystem` type in `src/particles.rs` with a public injection method. Assumed signature:
    `fn spawn_particle(&mut self, pos: (f32, f32), vel: (f32, f32), lifetime: Duration, color: Color, glyph: char)`
    — or a single `Particle` struct constructed by the caller and passed to `system.add(particle)`. The fireworks emit function will call whichever injection entry STR-001 exposes, building each particle from `origin`, a randomized velocity, a randomized lifetime, and a chosen color + glyph.
  - Particle position/velocity are `f32` cell-space `(x, y)` / `(vx, vy)`; lifetime is a `Duration` (or `f32` seconds); appearance is `ratatui::style::Color` + `char`.
  - **Implementation note for the coder:** read the merged `src/particles.rs` before writing the emit function and adapt the call to the actual injection signature. If STR-001 names the entry `emit` or takes a constructed `Particle`, adjust — the burst logic (loop, randomization) is unchanged; only the final injection call shifts.

- **Key modules / components:**
  - `src/rng.rs` — `Rng` (xorshift32 or splitmix-seeded xorshift). Owns a `u64`/`u32` state. `Rng::new(seed: u64) -> Self`, `next_u32`, `next_f32`, `range_f32(min, max)`. Tiny and self-contained. Tests in `src/rng/tests.rs`.
  - `src/effects.rs` — `EffectKind`, `FireworksParams`, `spawn`, and `emit_fireworks` (private). Tests in `src/effects/tests.rs`.

- **Data model:**
  - `Rng { state: u32 }` (or `u64`). xorshift step; `next_f32` derives a `[0,1)` float from the top mantissa bits of `next_u32`.
  - `FireworksParams { count: usize, palette: &'static [Color], spread: f32 /* radians, full-circle default */, speed: (f32, f32) /* min,max cell/s */, gravity: f32 /* hint; actual gravity lives in the system */, lifetime: (f32, f32) /* min,max seconds */ }`. `Default` sets a sensible burst (e.g. count ~40, a small fixed bright palette, full 2π spread, moderate speed and lifetime ranges).
  - Per particle: pick a random angle within `spread` and a random speed in `speed` → `(vx, vy)` via `(cos*speed, sin*speed)`; pick a random color from `palette`; pick a random lifetime in `lifetime`; glyph a fixed spark char (e.g. `'*'`). Inject at `origin`.

- **Key design decisions:**
  - **Seedable PRNG, no globals.** The `Rng` is constructed with an explicit seed and threaded by `&mut` into `spawn`. The sandbox (STR-004) will own one instance. This keeps determinism testable and avoids hidden global state.
  - **`gravity` lives in the system, not here.** STR-001 owns gravity. `FireworksParams.gravity` is a hint/placeholder for the "per-effect params are the extension point" decision; whether it overrides system gravity is deferred — for this story it may be unused or stored for future use. Note this in Considerations rather than wiring a gravity override into STR-001's API.
  - **Palette is a small fixed `&'static [Color]`** of bright colors reusing `ratatui::style::Color::Rgb`, mirroring `theme.rs`.

---

## Success Criteria
- [ ] `Rng::new(seed)` followed by repeated `next_u32` produces an identical sequence for two instances with the same seed, and a different sequence for a different seed (test).
- [ ] `next_f32` returns values in `[0.0, 1.0)` and `range_f32(min, max)` stays within `[min, max)` across many draws (test).
- [ ] `spawn(EffectKind::Fireworks, origin, &params, &mut rng, &mut system)` increases the system's live-particle count by `params.count` (test against STR-001's read surface).
- [ ] Emitted particles do not all share the same velocity — at least two distinct velocity vectors in a burst (test).
- [ ] Two spawns from `Rng` seeded identically (with identical params/origin) produce identical bursts; the particle velocities/lifetimes match (test).
- [ ] Changing `params.count` changes the number of particles injected (test).
- [ ] `cargo test`, `cargo fmt --check`, and `cargo clippy` all pass.

---

## Tasks
Ordered by dependency.

- [ ] **PRNG (`src/rng.rs` + tests):** Implement `Rng` (xorshift) with `new(seed)`, `next_u32`, `next_f32` (`[0,1)`), `range_f32(min, max)`. Register `mod rng;` in `src/main.rs`. Write tests in `src/rng/tests.rs`: same-seed determinism, different-seed divergence, `next_f32` bounds, `range_f32` bounds. Must be fully tested before the effects layer uses it.

- [ ] **Effects scaffolding (`src/effects.rs`):** Define `EffectKind` (with `Fireworks`), `FireworksParams` + `Default` impl, and the `spawn` dispatch signature delegating to a private `emit_fireworks`. Register `mod effects;` in `src/main.rs`. Get it compiling against the assumed STR-001 `ParticleSystem` interface (adapt to the real injection signature from the merged `src/particles.rs`).

- [ ] **Fireworks emit logic + tests:** Implement `emit_fireworks`: loop `params.count` times, draw random angle (within `spread`), speed (within `speed`), lifetime (within `lifetime`), and color (from `palette`); compute velocity and inject each particle into the system at `origin`. Write tests in `src/effects/tests.rs`: burst count equals `params.count`, velocity variation, seed reproducibility, param-count change. Use the system's read surface to assert.

---

## Considerations
- **STR-001 is unbuilt.** The injection call (`spawn_particle` vs. `add(Particle{...})`) and the exact lifetime type (`Duration` vs. `f32`) come from STR-001. Read the merged `src/particles.rs` first and adapt; do not invent a second particle model here.
- **`gravity` param is intentionally a near-no-op this story.** Gravity is the system's responsibility (STR-001). Keep the field for the params-extension decision but don't push a gravity-override into STR-001's API as part of this story — call that out if it ever needs wiring.
- **`next_f32` distribution:** derive the float from the high bits of `next_u32` (e.g. `(x >> 8) as f32 / (1u32 << 24) as f32`) to get a clean `[0,1)` — avoid modulo bias.
- **Angular distribution:** default `spread` of full `2π` gives a circular burst; with gravity the upward half arcs back down. The observable requirement is varied outward motion, so the exact angle distribution is the coder's call.
- **No `rand`.** Dependencies stay `color-eyre`, `crossterm`, `ratatui` only.
- **Module/test convention:** impl in `src/<mod>.rs` with `#[cfg(test)] mod tests;` and a sibling `src/<mod>/tests.rs`, matching `map.rs` / the project rule.
- **Palette colors** reuse `ratatui::style::Color::Rgb`, consistent with `src/theme.rs`.
