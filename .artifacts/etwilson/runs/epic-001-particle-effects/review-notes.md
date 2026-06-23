# Review Notes — epic-001-particle-effects

## Module scaffold + Spawn/read surface + Physics tick + Lifetime cull

## Verdict: APPROVED

**Task:** All tasks (Module scaffold + Spawn/read surface + Physics tick + Lifetime cull)
**Spec:** .artifacts/etwilson/specs/003-particle-system-core.md

**Scope issues:** none

**Coverage gaps:** none

All 16 expected test cases are covered across 16 named tests. Fade direction (0.0 = fresh, 1.0 = fully faded) matches the spec's documented direction. The `retain` predicate `remaining > Duration::ZERO` correctly culls at exactly zero. `saturating_sub` is used for lifetime decrement. Public fields are acceptable per the spec; no `&mut` accessor is exposed, preserving the single-mutator invariant.

---

## PRNG + Effects scaffolding + Fireworks emit logic

## Verdict: APPROVED

**Task:** PRNG (src/rng.rs + tests) + Effects scaffolding + Fireworks emit logic + tests
**Spec:** .artifacts/etwilson/specs/004-effects-layer-fireworks.md

**Scope issues:** none

**Coverage gaps:** none

All required test cases are covered. PRNG: same-seed determinism, different-seed divergence, `next_f32` in `[0.0, 1.0)`, and `range_f32` bounds (including negative range). Effects: burst count equals `params.count`, custom count respected, velocity variation observed, and full burst seed reproducibility. The extra `zero_seed_does_not_panic_or_return_all_zeros` test is a valid bonus edge case not in the spec — not flagged. The `emitted_particles_start_at_origin` test covers an implicit spec requirement (inject at origin) — welcome. Tests are in the correct RED state (todo! panics). Proceed to implementation.

---

## Projection math + Fade-to-color + draw_particles pipeline + Lint pass

## Verdict: APPROVED

**Task:** All tasks — projection math, fade-to-color, draw_particles pipeline, lint pass
**Spec:** .artifacts/etwilson/specs/005-particle-rendering.md

**Scope issues:** none

**Coverage gaps:** none

The semantic inversion in `Particle::fade()` (spec 003 ships 0.0=fresh, 1.0=aged; spec 005 assumed the reverse) is a legitimate upstream adaptation per the spec's own note ("if STR-001's actual surface differs, adapt the call sites"). The `fade_color` convention (0.0=fresh→base color, 1.0=aged→black) is consistent with the adapted implementation and tests.

Projection: all required rounding cases covered including positive round-down, half-way-from-zero round-up, zero, and negative coordinates surviving as `i32`. Fade-to-color: base color at fresh (0.0), black at fully aged (1.0), half-dim verified per channel, non-`Rgb` passthrough confirmed. draw_particles: glyph painted at projected cell, faded color applied, negative coordinate skips, beyond-area skips (right and bottom), same-cell last-write confirmed, origin offset verified. The `fade_color_aged_is_strictly_dimmer_than_fresh` test has an empty body in the submitted text — the requirement is fully covered by `fade_color_at_half_dims_each_channel` and `fade_color_at_fully_aged_approaches_black`, so no gap. All 21 tests pass.

---

## Module scaffold + pure helpers + Spawn cadence accumulator + Loop integration + Entry swap

## Verdict: APPROVED

**Task:** All 4 tasks (Module scaffold + pure helpers, Spawn cadence accumulator, Loop integration, Entry swap + parked game)
**Spec:** .artifacts/etwilson/specs/006-sandbox-screen.md

**Scope issues:** none

`src/app.rs` and `src/effects.rs` are not in the spec's declared scope files, but both edits are explicitly anticipated by the spec: making `FRAME_TIME` `pub(crate)` and adding `#![allow(dead_code)]` to `app.rs` are called out by name; adding `Clone, Copy, PartialEq, Eq` derives to `EffectKind` is a natural integration-time adaptation (required to use `EffectKind` in a const slice and dispatch by value). All edits are minimal and enabling. Not flagged.

**Coverage gaps:** none

Pure helpers: `area_center` verified with zero-offset, non-zero offset, and odd dimensions. `next_kind` verified for advance, wrap at end, and len-1 no-op. `map_sandbox_key` verified for Esc, q, Tab, and unknown keys. Cadence accumulator: below-interval (zero spawns), single-interval crossing (one spawn + correct remainder), double-interval crossing (two spawns), and two-frame carry-forward all covered. Loop integration and entry swap have no unit tests per spec (IO-bound) — confirmed by build/clippy/test all green. All 98 tests pass.
