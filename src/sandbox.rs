use crate::app::FRAME_TIME;
use crate::effects::{EffectKind, FireworksParams, spawn};
use crate::particle_render::draw_particles;
use crate::particles::ParticleSystem;
use crate::rng::Rng;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Paragraph};
use std::time::{Duration, Instant};

/// Fixed seed for the sandbox PRNG — reproducible run-to-run.
const SANDBOX_SEED: u64 = 0xdeadbeef_cafe1234;

/// How often to auto-spawn a fireworks burst at screen center.
const SPAWN_INTERVAL: Duration = Duration::from_millis(800);

/// All available effect kinds. When STR-002 exposes an ALL const this can be
/// replaced; for now we define the canonical list locally.
const EFFECT_KINDS: &[EffectKind] = &[EffectKind::Fireworks];

/// What a keypress resolves to in the sandbox.
#[derive(Debug, PartialEq, Eq)]
pub enum SandboxCommand {
    Quit,
    CycleEffect,
    Ignore,
}

/// Pure mapping from a key to a sandbox command. No IO — testable seam.
pub fn map_sandbox_key(code: KeyCode) -> SandboxCommand {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => SandboxCommand::Quit,
        KeyCode::Tab => SandboxCommand::CycleEffect,
        _ => SandboxCommand::Ignore,
    }
}

/// Compute the center of a `Rect` in f32 cell-space, including the area offset.
///
/// Particle coordinates are relative to the terminal buffer origin, so we must
/// add the area's top-left to the half-extents.
pub fn area_center(area: Rect) -> (f32, f32) {
    let cx = area.x as f32 + area.width as f32 / 2.0;
    let cy = area.y as f32 + area.height as f32 / 2.0;
    (cx, cy)
}

/// Advance the effect-kind index by one, wrapping at the end of the list.
///
/// With `len == 1` this always returns `0` — no-op on the value but the
/// dispatch path (title line + spawn call) still exercises the current kind.
pub fn next_kind(idx: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (idx + 1) % len
}

/// How many spawns should fire this frame and what remainder carries over.
///
/// Given the accumulated `dt` and the `interval`, returns `(count, remainder)`
/// where `count` is the number of times `interval` fits in `accumulator + dt`
/// and `remainder` is what's left over.
pub fn cadence_step(accumulator: Duration, dt: Duration, interval: Duration) -> (u32, Duration) {
    let total = accumulator + dt;
    if interval.is_zero() {
        return (0, total);
    }
    let count = total.as_nanos() / interval.as_nanos();
    let remainder = total - interval * count as u32;
    (count as u32, remainder)
}

/// Standalone sandbox loop.
///
/// Owns its own `ParticleSystem` and a seeded `Rng`. Auto-spawns fireworks at
/// the screen center on a recurring cadence. Tab cycles the selected effect
/// kind. Esc/q exits cleanly.
///
/// Same signature as `app::app` so `ratatui::run` accepts it with a one-line
/// swap in `main.rs`.
pub fn sandbox(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut system = ParticleSystem::new();
    let mut rng = Rng::new(SANDBOX_SEED);
    let params = FireworksParams::default();

    let mut kind_idx: usize = 0;
    let mut spawn_acc = Duration::ZERO;
    let mut last_area: Option<Rect> = None;
    let mut last = Instant::now();

    loop {
        // --- draw ---
        // Capture the body Rect via a cell outside the closure so we can use it
        // for spawning after the draw call returns.
        let mut drawn_body: Option<Rect> = None;
        terminal.draw(|frame| {
            let full = frame.area();

            // Split off a 1-row title at the top, mirroring render::ui's layout.
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(full);

            let title_area = chunks[0];
            let body = chunks[1];

            let kind_name = match EFFECT_KINDS[kind_idx] {
                EffectKind::Fireworks => "Fireworks",
            };
            let hint = format!(" Sandbox | Effect: {kind_name} | Tab: cycle | q/Esc: quit ");
            let title = Paragraph::new(Span::styled(hint, Style::default().fg(Color::Yellow)))
                .block(Block::default());
            frame.render_widget(title, title_area);

            // Clear the body with a dark background so particles read clearly.
            let bg = Block::default().style(Style::default().bg(Color::Black));
            frame.render_widget(bg, body);

            // Draw live particles. Origin (0,0) is the top-left of the body area;
            // particle positions are body-relative because center is computed from body.
            draw_particles(&system, frame.buffer_mut(), body, (body.x, body.y));

            drawn_body = Some(body);
        })?;

        // Track the live body area so center is always current after a resize.
        if let Some(body) = drawn_body {
            last_area = Some(body);
        }

        // --- dt ---
        let now = Instant::now();
        let dt = now - last;
        last = now;

        // --- input ---
        if event::poll(FRAME_TIME)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match map_sandbox_key(key.code) {
                SandboxCommand::Quit => return Ok(()),
                SandboxCommand::CycleEffect => {
                    kind_idx = next_kind(kind_idx, EFFECT_KINDS.len());
                }
                SandboxCommand::Ignore => {}
            }
        }

        // --- auto-spawn at center ---
        if let Some(area) = last_area {
            let (count, remainder) = cadence_step(spawn_acc, dt, SPAWN_INTERVAL);
            spawn_acc = remainder;
            let center = area_center(area);
            // center is absolute buffer coords; draw_particles offsets by area origin,
            // so we supply positions relative to the body's top-left.
            let relative_center = (center.0 - area.x as f32, center.1 - area.y as f32);
            for _ in 0..count {
                spawn(
                    EFFECT_KINDS[kind_idx],
                    relative_center,
                    &params,
                    &mut rng,
                    &mut system,
                );
            }
        }

        // --- tick ---
        system.tick(dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    // --- area_center ---

    #[test]
    fn area_center_of_simple_rect() {
        let area = Rect::new(0, 0, 80, 24);
        let (cx, cy) = area_center(area);
        assert!((cx - 40.0).abs() < 1e-4, "cx: {cx}");
        assert!((cy - 12.0).abs() < 1e-4, "cy: {cy}");
    }

    #[test]
    fn area_center_includes_area_offset() {
        // Area starts at (10, 5) with size 20x10 → center at (10+10, 5+5) = (20, 10)
        let area = Rect::new(10, 5, 20, 10);
        let (cx, cy) = area_center(area);
        assert!((cx - 20.0).abs() < 1e-4, "cx: {cx}");
        assert!((cy - 10.0).abs() < 1e-4, "cy: {cy}");
    }

    #[test]
    fn area_center_odd_dimensions() {
        // 81x25 → center at 40.5, 12.5
        let area = Rect::new(0, 0, 81, 25);
        let (cx, cy) = area_center(area);
        assert!((cx - 40.5).abs() < 1e-4, "cx: {cx}");
        assert!((cy - 12.5).abs() < 1e-4, "cy: {cy}");
    }

    // --- next_kind ---

    #[test]
    fn next_kind_wraps_at_end() {
        // With len=3: 0→1→2→0
        assert_eq!(next_kind(0, 3), 1);
        assert_eq!(next_kind(1, 3), 2);
        assert_eq!(next_kind(2, 3), 0);
    }

    #[test]
    fn next_kind_len_one_stays_zero() {
        // With only one kind the index always stays at 0.
        assert_eq!(next_kind(0, 1), 0);
    }

    #[test]
    fn next_kind_zero_len_returns_zero() {
        // Guard against empty list.
        assert_eq!(next_kind(0, 0), 0);
    }

    // --- map_sandbox_key ---

    #[test]
    fn esc_maps_to_quit() {
        assert_eq!(map_sandbox_key(KeyCode::Esc), SandboxCommand::Quit);
    }

    #[test]
    fn q_maps_to_quit() {
        assert_eq!(map_sandbox_key(KeyCode::Char('q')), SandboxCommand::Quit);
    }

    #[test]
    fn tab_maps_to_cycle_effect() {
        assert_eq!(map_sandbox_key(KeyCode::Tab), SandboxCommand::CycleEffect);
    }

    #[test]
    fn unknown_key_maps_to_ignore() {
        assert_eq!(map_sandbox_key(KeyCode::Char('z')), SandboxCommand::Ignore);
        assert_eq!(map_sandbox_key(KeyCode::Enter), SandboxCommand::Ignore);
    }

    // --- cadence_step ---

    #[test]
    fn cadence_no_spawn_below_interval() {
        let interval = Duration::from_millis(800);
        let (count, remainder) = cadence_step(Duration::ZERO, Duration::from_millis(16), interval);
        assert_eq!(count, 0);
        assert_eq!(remainder, Duration::from_millis(16));
    }

    #[test]
    fn cadence_one_spawn_when_interval_crossed() {
        let interval = Duration::from_millis(800);
        // accumulator at 790ms + dt 16ms = 806ms → 1 spawn, 6ms remainder
        let (count, remainder) = cadence_step(
            Duration::from_millis(790),
            Duration::from_millis(16),
            interval,
        );
        assert_eq!(count, 1);
        assert_eq!(remainder, Duration::from_millis(6));
    }

    #[test]
    fn cadence_two_spawns_when_two_intervals_crossed() {
        let interval = Duration::from_millis(100);
        // accumulator 0 + dt 250ms = 250ms → 2 full intervals, 50ms remainder
        let (count, remainder) = cadence_step(Duration::ZERO, Duration::from_millis(250), interval);
        assert_eq!(count, 2);
        assert_eq!(remainder, Duration::from_millis(50));
    }

    #[test]
    fn cadence_carries_remainder_forward() {
        let interval = Duration::from_millis(800);
        // Step 1: 0 + 600ms → 0 spawns, 600ms left
        let (c1, r1) = cadence_step(Duration::ZERO, Duration::from_millis(600), interval);
        assert_eq!(c1, 0);
        // Step 2: 600ms + 400ms = 1000ms → 1 spawn, 200ms left
        let (c2, r2) = cadence_step(r1, Duration::from_millis(400), interval);
        assert_eq!(c2, 1);
        assert_eq!(r2, Duration::from_millis(200));
    }
}
