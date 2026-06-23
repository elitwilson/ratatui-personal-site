use crate::effects::FireworksParams;
use std::f32::consts::{PI, TAU};
use std::time::Duration;

/// Minimum spawn interval — prevents cadence reaching zero (which silently
/// stops spawning because `cadence_step` guards zero).
const SPAWN_INTERVAL_MIN: Duration = Duration::from_millis(50);

/// Per-field step sizes for the config panel.
const COUNT_STEP: usize = 5;
const SPREAD_STEP: f32 = PI / 12.0;
const SPEED_STEP: f32 = 1.0;
const LIFETIME_STEP: f32 = 0.1;
const SPAWN_INTERVAL_STEP: Duration = Duration::from_millis(100);

/// An adjustable knob exposed by the config panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigField {
    Count,
    Spread,
    SpeedMin,
    SpeedMax,
    LifetimeMin,
    LifetimeMax,
    SpawnInterval,
}

/// Ordered list of fields as they appear in the panel (top to bottom).
pub const FIELDS: &[ConfigField] = &[
    ConfigField::Count,
    ConfigField::Spread,
    ConfigField::SpeedMin,
    ConfigField::SpeedMax,
    ConfigField::LifetimeMin,
    ConfigField::LifetimeMax,
    ConfigField::SpawnInterval,
];

/// Runtime-mutable sandbox configuration.
///
/// Replaces the compile-time `FireworksParams::default()` and `SPAWN_INTERVAL`
/// const in `sandbox.rs` so all knobs can be adjusted live from the config panel.
pub struct SandboxConfig {
    /// Fireworks burst parameters — mutated live by the panel.
    pub params: FireworksParams,
    /// How often to auto-spawn a burst. Clamped ≥ 50 ms.
    pub spawn_interval: Duration,
    /// Index into `FIELDS` for the currently selected panel row.
    pub selected: usize,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            params: FireworksParams::default(),
            spawn_interval: Duration::from_millis(750),
            selected: 0,
        }
    }
}

impl SandboxConfig {
    /// Move the selection up one row, wrapping from the top to the bottom.
    pub fn select_prev(&mut self) {
        if self.selected == 0 {
            self.selected = FIELDS.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    /// Move the selection down one row, wrapping from the bottom to the top.
    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1) % FIELDS.len();
    }

    /// Return the currently selected `ConfigField`.
    pub fn selected_field(&self) -> ConfigField {
        FIELDS[self.selected]
    }
}

/// Apply one step of `dir` (`+1` or `-1`) to `field` in `config`, then clamp
/// and re-establish any ordering invariants.
pub fn step(config: &mut SandboxConfig, field: ConfigField, dir: i32) {
    match field {
        ConfigField::Count => {
            if dir > 0 {
                config.params.count = config.params.count.saturating_add(COUNT_STEP);
            } else {
                config.params.count = config.params.count.saturating_sub(COUNT_STEP).max(1);
            }
        }
        ConfigField::Spread => {
            config.params.spread += SPREAD_STEP * dir as f32;
            config.params.spread = config.params.spread.clamp(0.0, TAU);
        }
        ConfigField::SpeedMin => {
            config.params.speed.0 += SPEED_STEP * dir as f32;
            config.params.speed.0 = config.params.speed.0.max(0.0);
            // Maintain speed_min <= speed_max
            if config.params.speed.0 > config.params.speed.1 {
                config.params.speed.1 = config.params.speed.0;
            }
        }
        ConfigField::SpeedMax => {
            config.params.speed.1 += SPEED_STEP * dir as f32;
            config.params.speed.1 = config.params.speed.1.max(0.0);
            // Maintain speed_min <= speed_max
            if config.params.speed.1 < config.params.speed.0 {
                config.params.speed.0 = config.params.speed.1;
            }
        }
        ConfigField::LifetimeMin => {
            config.params.lifetime.0 += LIFETIME_STEP * dir as f32;
            config.params.lifetime.0 = config.params.lifetime.0.max(0.1);
            // Maintain lifetime_min <= lifetime_max
            if config.params.lifetime.0 > config.params.lifetime.1 {
                config.params.lifetime.1 = config.params.lifetime.0;
            }
        }
        ConfigField::LifetimeMax => {
            config.params.lifetime.1 += LIFETIME_STEP * dir as f32;
            config.params.lifetime.1 = config.params.lifetime.1.max(0.1);
            // Maintain lifetime_min <= lifetime_max
            if config.params.lifetime.1 < config.params.lifetime.0 {
                config.params.lifetime.0 = config.params.lifetime.1;
            }
        }
        ConfigField::SpawnInterval => {
            if dir > 0 {
                config.spawn_interval += SPAWN_INTERVAL_STEP;
            } else {
                config.spawn_interval = config
                    .spawn_interval
                    .saturating_sub(SPAWN_INTERVAL_STEP)
                    .max(SPAWN_INTERVAL_MIN);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn default_config() -> SandboxConfig {
        SandboxConfig::default()
    }

    // --- selection wrapping ---

    #[test]
    fn select_next_advances_by_one() {
        let mut cfg = default_config();
        cfg.selected = 0;
        cfg.select_next();
        assert_eq!(cfg.selected, 1);
    }

    #[test]
    fn select_next_wraps_at_end() {
        let mut cfg = default_config();
        cfg.selected = FIELDS.len() - 1;
        cfg.select_next();
        assert_eq!(cfg.selected, 0);
    }

    #[test]
    fn select_prev_moves_back_one() {
        let mut cfg = default_config();
        cfg.selected = 3;
        cfg.select_prev();
        assert_eq!(cfg.selected, 2);
    }

    #[test]
    fn select_prev_wraps_at_start() {
        let mut cfg = default_config();
        cfg.selected = 0;
        cfg.select_prev();
        assert_eq!(cfg.selected, FIELDS.len() - 1);
    }

    // --- count step/clamp ---

    #[test]
    fn step_count_increments_by_step_size() {
        let mut cfg = default_config();
        let before = cfg.params.count;
        step(&mut cfg, ConfigField::Count, 1);
        assert_eq!(cfg.params.count, before + COUNT_STEP);
    }

    #[test]
    fn step_count_decrements_by_step_size() {
        let mut cfg = default_config();
        let before = cfg.params.count;
        step(&mut cfg, ConfigField::Count, -1);
        assert_eq!(cfg.params.count, before - COUNT_STEP);
    }

    #[test]
    fn step_count_cannot_go_below_one() {
        let mut cfg = default_config();
        cfg.params.count = 1;
        step(&mut cfg, ConfigField::Count, -1);
        assert_eq!(cfg.params.count, 1, "count must not go below 1");
    }

    #[test]
    fn step_count_from_small_value_clamps_at_one() {
        let mut cfg = default_config();
        cfg.params.count = 3; // less than COUNT_STEP
        step(&mut cfg, ConfigField::Count, -1);
        assert_eq!(cfg.params.count, 1);
    }

    // --- spread step/clamp ---

    #[test]
    fn step_spread_increments() {
        let mut cfg = default_config();
        cfg.params.spread = PI; // start below TAU so increment is not clamped
        let before = cfg.params.spread;
        step(&mut cfg, ConfigField::Spread, 1);
        assert!((cfg.params.spread - (before + SPREAD_STEP)).abs() < 1e-5);
    }

    #[test]
    fn step_spread_decrements() {
        let mut cfg = default_config();
        let before = cfg.params.spread;
        step(&mut cfg, ConfigField::Spread, -1);
        assert!((cfg.params.spread - (before - SPREAD_STEP)).abs() < 1e-5);
    }

    #[test]
    fn step_spread_clamps_at_zero() {
        let mut cfg = default_config();
        cfg.params.spread = 0.0;
        step(&mut cfg, ConfigField::Spread, -1);
        assert_eq!(cfg.params.spread, 0.0);
    }

    #[test]
    fn step_spread_clamps_at_tau() {
        let mut cfg = default_config();
        cfg.params.spread = TAU;
        step(&mut cfg, ConfigField::Spread, 1);
        assert!((cfg.params.spread - TAU).abs() < 1e-5);
    }

    // --- speed step/clamp/invariant ---

    #[test]
    fn step_speed_min_increments() {
        let mut cfg = default_config();
        let before = cfg.params.speed.0;
        step(&mut cfg, ConfigField::SpeedMin, 1);
        assert!((cfg.params.speed.0 - (before + SPEED_STEP)).abs() < 1e-5);
    }

    #[test]
    fn step_speed_min_cannot_go_negative() {
        let mut cfg = default_config();
        cfg.params.speed.0 = 0.0;
        step(&mut cfg, ConfigField::SpeedMin, -1);
        assert_eq!(cfg.params.speed.0, 0.0);
    }

    #[test]
    fn step_speed_min_above_max_pulls_max_up() {
        let mut cfg = default_config();
        cfg.params.speed = (9.5, 10.0); // stepping +1 → min=10.5 > max=10.0
        step(&mut cfg, ConfigField::SpeedMin, 1);
        assert!(
            cfg.params.speed.0 <= cfg.params.speed.1,
            "speed_min must not exceed speed_max: {:?}",
            cfg.params.speed
        );
        assert_eq!(cfg.params.speed.1, cfg.params.speed.0);
    }

    #[test]
    fn step_speed_max_decrements() {
        let mut cfg = default_config();
        let before = cfg.params.speed.1;
        step(&mut cfg, ConfigField::SpeedMax, -1);
        assert!((cfg.params.speed.1 - (before - SPEED_STEP)).abs() < 1e-5);
    }

    #[test]
    fn step_speed_max_below_min_pulls_min_down() {
        let mut cfg = default_config();
        cfg.params.speed = (9.0, 9.5); // stepping -1 → max=8.5 < min=9.0
        step(&mut cfg, ConfigField::SpeedMax, -1);
        assert!(
            cfg.params.speed.0 <= cfg.params.speed.1,
            "speed_min must not exceed speed_max: {:?}",
            cfg.params.speed
        );
        assert_eq!(cfg.params.speed.0, cfg.params.speed.1);
    }

    // --- lifetime step/clamp/invariant ---

    #[test]
    fn step_lifetime_min_increments() {
        let mut cfg = default_config();
        let before = cfg.params.lifetime.0;
        step(&mut cfg, ConfigField::LifetimeMin, 1);
        assert!((cfg.params.lifetime.0 - (before + LIFETIME_STEP)).abs() < 1e-5);
    }

    #[test]
    fn step_lifetime_min_cannot_go_below_0_1() {
        let mut cfg = default_config();
        cfg.params.lifetime.0 = 0.1;
        step(&mut cfg, ConfigField::LifetimeMin, -1);
        assert!((cfg.params.lifetime.0 - 0.1).abs() < 1e-5);
    }

    #[test]
    fn step_lifetime_min_above_max_pulls_max_up() {
        let mut cfg = default_config();
        cfg.params.lifetime = (0.95, 1.0);
        step(&mut cfg, ConfigField::LifetimeMin, 1);
        assert!(
            cfg.params.lifetime.0 <= cfg.params.lifetime.1,
            "lifetime_min must not exceed lifetime_max: {:?}",
            cfg.params.lifetime
        );
        assert_eq!(cfg.params.lifetime.1, cfg.params.lifetime.0);
    }

    #[test]
    fn step_lifetime_max_decrements() {
        let mut cfg = default_config();
        let before = cfg.params.lifetime.1;
        step(&mut cfg, ConfigField::LifetimeMax, -1);
        assert!((cfg.params.lifetime.1 - (before - LIFETIME_STEP)).abs() < 1e-5);
    }

    #[test]
    fn step_lifetime_max_below_min_pulls_min_down() {
        let mut cfg = default_config();
        cfg.params.lifetime = (0.9, 1.0); // stepping -1 → max=0.9, min=0.9 - that's fine
        // make it clearly cross: (0.95, 1.0) → -1 → max=0.9 < min=0.95
        cfg.params.lifetime = (0.95, 1.0);
        step(&mut cfg, ConfigField::LifetimeMax, -1);
        assert!(
            cfg.params.lifetime.0 <= cfg.params.lifetime.1,
            "lifetime_min must not exceed lifetime_max: {:?}",
            cfg.params.lifetime
        );
        assert_eq!(cfg.params.lifetime.0, cfg.params.lifetime.1);
    }

    #[test]
    fn step_lifetime_max_cannot_go_below_0_1() {
        let mut cfg = default_config();
        cfg.params.lifetime = (0.1, 0.1);
        step(&mut cfg, ConfigField::LifetimeMax, -1);
        assert!((cfg.params.lifetime.1 - 0.1).abs() < 1e-5);
        assert!((cfg.params.lifetime.0 - 0.1).abs() < 1e-5);
    }

    // --- spawn_interval step/clamp ---

    #[test]
    fn step_spawn_interval_increments() {
        let mut cfg = default_config();
        let before = cfg.spawn_interval;
        step(&mut cfg, ConfigField::SpawnInterval, 1);
        assert_eq!(cfg.spawn_interval, before + SPAWN_INTERVAL_STEP);
    }

    #[test]
    fn step_spawn_interval_decrements() {
        let mut cfg = default_config();
        let before = cfg.spawn_interval;
        step(&mut cfg, ConfigField::SpawnInterval, -1);
        assert_eq!(cfg.spawn_interval, before - SPAWN_INTERVAL_STEP);
    }

    #[test]
    fn step_spawn_interval_cannot_go_below_minimum() {
        let mut cfg = default_config();
        cfg.spawn_interval = SPAWN_INTERVAL_MIN;
        step(&mut cfg, ConfigField::SpawnInterval, -1);
        assert_eq!(cfg.spawn_interval, SPAWN_INTERVAL_MIN);
    }

    #[test]
    fn step_spawn_interval_near_minimum_clamps() {
        let mut cfg = default_config();
        cfg.spawn_interval = Duration::from_millis(75); // less than one step above min
        step(&mut cfg, ConfigField::SpawnInterval, -1);
        assert_eq!(cfg.spawn_interval, SPAWN_INTERVAL_MIN);
    }
}
