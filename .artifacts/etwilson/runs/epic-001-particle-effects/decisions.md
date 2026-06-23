# Decisions — epic-001-particle-effects (003-particle-system-core + 004-effects-layer-fireworks)

## 2026-06-22: Consolidated all 4 spec tasks into a single review round

**Context:** The spec defines 4 sequential tasks (scaffold, spawn+read, tick, cull), but all 4 live in a single file (`src/particles.rs`) with tight interdependencies — you can't write a meaningful failing test for spawn without the struct definition, and you can't write a meaningful failing test for tick without spawn.

**Decision:** Sent a single review request covering all tasks together rather than 4 sequential RED→review→GREEN cycles.

**Why:** Sending 4 separate requests would require scaffolding stub implementations between rounds, adding noise without adding safety. The reviewer can still verify all spec acceptance criteria against the tests.

**Risk:** Minor — if the reviewer flags task 1's tests as wrong, the later tasks would need revision. Given the tight coupling, this is the more practical approach.

## 2026-06-22: Kept dead_code warnings unfixed

**Context:** `cargo clippy` emits `dead_code` warnings for `GRAVITY`, `Particle`, `ParticleSystem`, etc. because `src/main.rs` declares `mod particles;` but no production code calls into it yet.

**Decision:** Left warnings in place rather than adding `#[allow(dead_code)]` suppressions.

**Why:** STR-004 (sandbox screen) will wire `ParticleSystem` into `App::tick`, at which point the warnings resolve naturally. Suppressing them now would leave stale allows after STR-004 lands.

## 2026-06-22 (004): Consolidated 3 spec tasks into one review round

**Context:** The spec defines 3 tasks (PRNG, effects scaffold, fireworks emit), but the effects tests depend on a compiling `Rng` stub — you cannot write meaningful effects tests without at least a stub `rng.rs` in place. The PRNG and effects layers are naturally ordered but have no mutual coupling beyond the import.

**Decision:** Sent a single review request covering all three tasks together.

**Why:** Splitting across three rounds would require shipping a stub `Rng` and a stub `effects.rs` just to make the project compile between rounds. Consolidated review was lower noise without lowering coverage.

## 2026-06-22 (004): Angular spread design — centered on x-axis

**Context:** The spec says `spread: f32` controls the arc of the burst and that default `2π` gives a full circle. It does not specify where the "center" of the spread is.

**Decision:** Angles are drawn uniformly from `[-spread/2, +spread/2)`. At full `TAU` spread this covers the full circle. At narrower values the burst fans around the positive-x axis; gravity then pulls the arc downward.

**Why:** The spec only requires "varied outward motion" and a radial-ish burst. Centering on x-axis is the simplest, symmetric choice. The sandbox (STR-004) can rotate the spawn call if directional control is needed — that's a call-site concern.

## 2026-06-22: Pre-existing test failures noted

**Failing tests on base branch (not caused by this spec):**
- `app::tests::app_initial_state` — expects player at (10,8), gets (32,34)
- `map::tests::castle_map_parses_to_expected_state` — same mismatch

These appear to be from a castle.map update (commit f6309f6 "Add epic 001 and stories/specs. Update castle.map.") that changed player spawn position but didn't update the test assertions. Outside scope of this spec.
