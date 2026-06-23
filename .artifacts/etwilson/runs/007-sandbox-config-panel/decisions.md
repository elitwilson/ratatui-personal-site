# Decisions — 007-sandbox-config-panel

## Test convention
Spec explicitly overrides the global rust-testing.md rule (sibling test files).
All tests for sandbox_config.rs go inline in `#[cfg(test)] mod tests { ... }` per the spec's "Follow the existing inline test convention" note.

## selection field placement
`selected: usize` will live directly on `SandboxConfig` (not separately tracked in sandbox.rs) — simpler and the spec lists it as an option.

## FireworksParams.palette
`palette` is `&'static [Color]` and is not mutable via the panel (only 7 knobs are in scope; palette is out of scope per spec). SandboxConfig will hold `params: FireworksParams` and palette stays at its default.

## gravity field
Deliberately excluded from panel knobs per spec. `FireworksParams.gravity` is inert and the real gravity lives in `particles.rs`.

## ConfigField step for spread
Spec suggests ±π/12. Using `std::f32::consts::PI / 12.0` as step, clamped to `[0.0, TAU]`.

## spawn_interval in SandboxConfig
`spawn_interval` stored as `Duration` on `SandboxConfig`, replacing the `SPAWN_INTERVAL` const. Min clamp: 50ms per spec.
