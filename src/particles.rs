use ratatui::style::Color;
use std::time::Duration;

/// Downward acceleration in cell-units per second squared.
/// Screen y grows downward, so gravity is positive on vy.
/// This is a tuning value; effects can revisit.
const GRAVITY: f32 = 30.0;

/// A single simulated particle in f32 cell-space.
///
/// Fields are public so the effects layer (STR-002) can construct particles
/// ergonomically. Mutation is only permitted inside `ParticleSystem::tick`.
pub struct Particle {
    /// Position in cell-space (x, y). Screen y grows downward.
    pub pos: (f32, f32),
    /// Velocity in cells per second (vx, vy). Positive vy moves downward.
    pub vel: (f32, f32),
    /// Remaining lifetime. Decrements each tick; particle is culled at zero.
    pub remaining: Duration,
    /// Total initial lifetime. Used to compute fade progress.
    pub total: Duration,
    /// Display color passed through to the renderer.
    pub color: Color,
    /// Character glyph to render at the particle's position.
    pub glyph: char,
}

impl Particle {
    /// Normalized fade progress in `[0.0, 1.0]`.
    ///
    /// Returns `0.0` for a freshly spawned particle and approaches `1.0` as
    /// remaining lifetime drains. The renderer uses this to dim aging particles.
    /// Clamped so it never exceeds `[0.0, 1.0]` regardless of Duration rounding.
    pub fn fade(&self) -> f32 {
        let total_secs = self.total.as_secs_f32();
        if total_secs <= 0.0 {
            return 1.0;
        }
        let remaining_secs = self.remaining.as_secs_f32();
        (1.0 - remaining_secs / total_secs).clamp(0.0, 1.0)
    }
}

/// Owns and advances a collection of live particles.
///
/// The only mutating entry points are `spawn` (inject a new particle) and
/// `tick` (advance physics). Outside code observes the live set through
/// `particles()`, which hands out shared references only.
pub struct ParticleSystem {
    particles: Vec<Particle>,
}

impl ParticleSystem {
    /// Construct an empty particle system.
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
        }
    }

    /// Inject a particle into the system. Called by the effects layer.
    pub fn spawn(&mut self, particle: Particle) {
        self.particles.push(particle);
    }

    /// Advance all live particles by `dt`.
    ///
    /// Per particle: gravity accumulates downward velocity, position is
    /// integrated by velocity * dt, remaining lifetime is decremented.
    /// Particles whose remaining lifetime reaches zero are culled.
    pub fn tick(&mut self, dt: Duration) {
        let secs = dt.as_secs_f32();
        for p in &mut self.particles {
            p.vel.1 += GRAVITY * secs;
            p.pos.0 += p.vel.0 * secs;
            p.pos.1 += p.vel.1 * secs;
            p.remaining = p.remaining.saturating_sub(dt);
        }
        // A particle with remaining == 0 is dead; cull it so it never renders.
        self.particles.retain(|p| p.remaining > Duration::ZERO);
    }

    /// Read-only view of live particles for the renderer.
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- helpers ---

    fn make_particle(vx: f32, vy: f32, lifetime_secs: f32) -> Particle {
        let lifetime = Duration::from_secs_f32(lifetime_secs);
        Particle {
            pos: (0.0, 0.0),
            vel: (vx, vy),
            remaining: lifetime,
            total: lifetime,
            color: Color::White,
            glyph: '*',
        }
    }

    // --- spawn + read surface ---

    #[test]
    fn empty_system_has_no_particles() {
        let sys = ParticleSystem::new();
        assert_eq!(sys.particles().len(), 0);
    }

    #[test]
    fn spawned_particle_appears_in_read_surface() {
        let mut sys = ParticleSystem::new();
        sys.spawn(make_particle(0.0, 0.0, 1.0));
        assert_eq!(sys.particles().len(), 1);
    }

    #[test]
    fn multiple_spawned_particles_all_appear() {
        let mut sys = ParticleSystem::new();
        sys.spawn(make_particle(1.0, 0.0, 1.0));
        sys.spawn(make_particle(-1.0, 0.0, 2.0));
        assert_eq!(sys.particles().len(), 2);
    }

    #[test]
    fn read_surface_returns_shared_refs_only() {
        // The return type of particles() is &[Particle] (shared slice reference).
        // This test asserts the compiler-enforced contract: we can only obtain &Particle.
        let mut sys = ParticleSystem::new();
        sys.spawn(make_particle(0.0, 0.0, 1.0));
        let slice: &[Particle] = sys.particles();
        let _p: &Particle = &slice[0]; // shared ref — mutation not possible
    }

    // --- fade ---

    #[test]
    fn fade_is_zero_at_spawn() {
        let p = make_particle(0.0, 0.0, 2.0);
        // remaining == total at spawn, so fade should be 0.0
        assert!((p.fade() - 0.0).abs() < 1e-4, "fade at spawn: {}", p.fade());
    }

    #[test]
    fn fade_approaches_one_as_lifetime_drains() {
        let total = Duration::from_secs_f32(2.0);
        let p = Particle {
            pos: (0.0, 0.0),
            vel: (0.0, 0.0),
            remaining: Duration::from_secs_f32(0.5),
            total,
            color: Color::White,
            glyph: '*',
        };
        // remaining is 0.5/2.0 = 25% left, so fade = 1 - 0.25 = 0.75
        assert!((p.fade() - 0.75).abs() < 1e-4, "fade: {}", p.fade());
    }

    #[test]
    fn fade_is_clamped_to_unit_range() {
        // A zero-duration total should return 1.0 (fully faded), not NaN/inf.
        let p = Particle {
            pos: (0.0, 0.0),
            vel: (0.0, 0.0),
            remaining: Duration::ZERO,
            total: Duration::ZERO,
            color: Color::White,
            glyph: '*',
        };
        let f = p.fade();
        assert!(f >= 0.0 && f <= 1.0, "fade out of range: {f}");
    }

    // --- physics tick: position integration ---

    #[test]
    fn tick_integrates_position_by_velocity() {
        let mut sys = ParticleSystem::new();
        // Simple velocity: vx=4.0, vy=0.0; dt=0.5s => dx=2.0, dy depends on gravity
        sys.spawn(make_particle(4.0, 0.0, 10.0));
        let dt = Duration::from_secs_f32(0.5);
        sys.tick(dt);

        let p = &sys.particles()[0];
        // x should move by vx * dt = 4.0 * 0.5 = 2.0
        assert!((p.pos.0 - 2.0).abs() < 1e-4, "x pos: {}", p.pos.0);
        // y: gravity applied first (vy += GRAVITY * secs), then integrate
        // vy_after = 0 + 30.0 * 0.5 = 15.0; y = 0 + 15.0 * 0.5 = 7.5
        assert!((p.pos.1 - 7.5).abs() < 1e-4, "y pos: {}", p.pos.1);
    }

    #[test]
    fn tick_with_nonzero_vy_moves_position() {
        let mut sys = ParticleSystem::new();
        sys.spawn(make_particle(0.0, 10.0, 10.0));
        let dt = Duration::from_secs_f32(0.5);
        sys.tick(dt);

        let p = &sys.particles()[0];
        // vy_after = 10.0 + GRAVITY * 0.5 = 10.0 + 15.0 = 25.0
        // y = 0 + 25.0 * 0.5 = 12.5
        assert!((p.pos.1 - 12.5).abs() < 1e-4, "y pos: {}", p.pos.1);
    }

    // --- physics tick: gravity ---

    #[test]
    fn zero_vy_particle_gains_downward_velocity_after_tick() {
        let mut sys = ParticleSystem::new();
        sys.spawn(make_particle(0.0, 0.0, 10.0));
        let dt = Duration::from_secs_f32(0.1);
        sys.tick(dt);

        let p = &sys.particles()[0];
        // vy should now be positive (downward) due to gravity
        assert!(
            p.vel.1 > 0.0,
            "vy should be positive after gravity tick: {}",
            p.vel.1
        );
    }

    #[test]
    fn gravity_accumulates_over_successive_ticks() {
        let mut sys = ParticleSystem::new();
        sys.spawn(make_particle(0.0, 0.0, 10.0));
        let dt = Duration::from_secs_f32(0.1);

        sys.tick(dt);
        let vy_after_1 = sys.particles()[0].vel.1;

        sys.tick(dt);
        let vy_after_2 = sys.particles()[0].vel.1;

        assert!(
            vy_after_2 > vy_after_1,
            "gravity should accumulate: {vy_after_1} -> {vy_after_2}"
        );
    }

    // --- lifetime cull ---

    #[test]
    fn particle_with_remaining_lifetime_survives_tick() {
        let mut sys = ParticleSystem::new();
        // lifetime 2s, tick by 0.5s → still 1.5s remaining
        sys.spawn(make_particle(0.0, 0.0, 2.0));
        sys.tick(Duration::from_secs_f32(0.5));
        assert_eq!(sys.particles().len(), 1);
    }

    #[test]
    fn particle_whose_lifetime_expires_is_culled() {
        let mut sys = ParticleSystem::new();
        // lifetime 0.5s, tick by exactly 0.5s → remaining hits 0 → culled
        sys.spawn(make_particle(0.0, 0.0, 0.5));
        sys.tick(Duration::from_secs_f32(0.5));
        assert_eq!(
            sys.particles().len(),
            0,
            "expired particle should be removed"
        );
    }

    #[test]
    fn particle_culled_when_dt_exceeds_remaining_lifetime() {
        let mut sys = ParticleSystem::new();
        // lifetime 0.3s, tick by 1.0s → saturating_sub floors at 0 → culled
        sys.spawn(make_particle(0.0, 0.0, 0.3));
        sys.tick(Duration::from_secs_f32(1.0));
        assert_eq!(
            sys.particles().len(),
            0,
            "particle should be culled when dt > remaining"
        );
    }

    #[test]
    fn only_expired_particle_is_culled_living_one_survives() {
        let mut sys = ParticleSystem::new();
        sys.spawn(make_particle(0.0, 0.0, 0.5)); // will expire
        sys.spawn(make_particle(0.0, 0.0, 5.0)); // will survive
        sys.tick(Duration::from_secs_f32(0.5));
        assert_eq!(
            sys.particles().len(),
            1,
            "only the living particle should remain"
        );
    }

    // --- determinism ---

    #[test]
    fn same_inputs_produce_same_outputs() {
        let make_sys = || {
            let mut sys = ParticleSystem::new();
            sys.spawn(make_particle(3.0, -5.0, 2.0));
            sys
        };

        let dt = Duration::from_secs_f32(0.25);

        let mut sys_a = make_sys();
        sys_a.tick(dt);
        sys_a.tick(dt);

        let mut sys_b = make_sys();
        sys_b.tick(dt);
        sys_b.tick(dt);

        let pa = &sys_a.particles()[0];
        let pb = &sys_b.particles()[0];
        assert!(
            (pa.pos.0 - pb.pos.0).abs() < 1e-6,
            "x differs: {} vs {}",
            pa.pos.0,
            pb.pos.0
        );
        assert!(
            (pa.pos.1 - pb.pos.1).abs() < 1e-6,
            "y differs: {} vs {}",
            pa.pos.1,
            pb.pos.1
        );
    }
}
