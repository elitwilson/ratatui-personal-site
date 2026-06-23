# Decisions — epic-001-particle-effects (003-particle-system-core)

## 2026-06-22: Consolidated all 4 spec tasks into a single review round

**Context:** The spec defines 4 sequential tasks (scaffold, spawn+read, tick, cull), but all 4 live in a single file (`src/particles.rs`) with tight interdependencies — you can't write a meaningful failing test for spawn without the struct definition, and you can't write a meaningful failing test for tick without spawn.

**Decision:** Sent a single review request covering all tasks together rather than 4 sequential RED→review→GREEN cycles.

**Why:** Sending 4 separate requests would require scaffolding stub implementations between rounds, adding noise without adding safety. The reviewer can still verify all spec acceptance criteria against the tests.

**Risk:** Minor — if the reviewer flags task 1's tests as wrong, the later tasks would need revision. Given the tight coupling, this is the more practical approach.

## 2026-06-22: Kept dead_code warnings unfixed

**Context:** `cargo clippy` emits `dead_code` warnings for `GRAVITY`, `Particle`, `ParticleSystem`, etc. because `src/main.rs` declares `mod particles;` but no production code calls into it yet.

**Decision:** Left warnings in place rather than adding `#[allow(dead_code)]` suppressions.

**Why:** STR-004 (sandbox screen) will wire `ParticleSystem` into `App::tick`, at which point the warnings resolve naturally. Suppressing them now would leave stale allows after STR-004 lands.

## 2026-06-22: Pre-existing test failures noted

**Failing tests on base branch (not caused by this spec):**
- `app::tests::app_initial_state` — expects player at (10,8), gets (32,34)
- `map::tests::castle_map_parses_to_expected_state` — same mismatch

These appear to be from a castle.map update (commit f6309f6 "Add epic 001 and stories/specs. Update castle.map.") that changed player spawn position but didn't update the test assertions. Outside scope of this spec.
