use crate::app::FRAME_TIME;
use crate::effects::{EffectKind, FireworksParams, spawn};
use crate::particle_render::draw_particles;
use crate::particles::ParticleSystem;
use crate::rng::Rng;
use crate::sandbox_config::{ConfigField, FIELDS, SandboxConfig, step};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
};
use crossterm::execute;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::io::stdout;
use std::time::{Duration, Instant};

/// Fixed seed for the sandbox PRNG — reproducible run-to-run.
const SANDBOX_SEED: u64 = 0xdeadbeef_cafe1234;

/// How often to auto-spawn a fireworks burst. This is the customizable cadence
/// knob — e.g. `from_secs(2)` for a slow drip, `from_millis(500)` for rapid fire.
const SPAWN_INTERVAL: Duration = Duration::from_millis(750);

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

/// Choose the burst origin in body-relative cell-space.
///
/// Returns the mouse cell when known, otherwise the body center — both expressed
/// relative to `area`'s top-left, which is what `spawn` and `draw_particles`
/// expect (particle positions are body-relative; the renderer re-applies the
/// area offset when drawing).
pub fn spawn_origin(area: Rect, mouse: Option<(u16, u16)>) -> (f32, f32) {
    match mouse {
        Some((mx, my)) => (mx as f32 - area.x as f32, my as f32 - area.y as f32),
        None => {
            let (cx, cy) = area_center(area);
            (cx - area.x as f32, cy - area.y as f32)
        }
    }
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

/// Render the config overlay onto `frame`.
///
/// Draws a centered bordered panel listing all adjustable fields with their
/// current values. The selected row is highlighted in yellow+bold. The sandbox
/// keeps rendering behind the panel (caller draws particles first, then calls
/// this to overlay).
pub fn draw_config_panel(frame: &mut Frame, config: &SandboxConfig) {
    let full = frame.area();

    // Centre a panel that is 36 columns wide and has one row per field plus
    // 2 border rows and 1 header padding row = FIELDS.len() + 3 rows tall.
    let panel_w = 36u16;
    let panel_h = (FIELDS.len() as u16) + 3;
    let x = full.width.saturating_sub(panel_w) / 2;
    let y = full.height.saturating_sub(panel_h) / 2;
    let panel = Rect::new(x, y, panel_w.min(full.width), panel_h.min(full.height));

    // Clear the cells behind the panel so particle glyphs don't show through.
    frame.render_widget(Clear, panel);

    let block = Block::default()
        .title(" Config ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White).bg(Color::Black));
    frame.render_widget(block, panel);

    // Inner area: inside the border (1-cell inset on all sides).
    let inner = Rect::new(panel.x + 1, panel.y + 1, panel.width.saturating_sub(2), panel.height.saturating_sub(2));

    for (i, field) in FIELDS.iter().enumerate() {
        let label = field_label(*field);
        let value = field_value(config, *field);
        let text = format!("{label}: {value}");

        let style = if i == config.selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
        if row.y < panel.y + panel.height - 1 {
            frame.render_widget(Paragraph::new(Line::from(Span::styled(text, style))), row);
        }
    }
}

/// Human-readable label for a config field.
fn field_label(field: ConfigField) -> &'static str {
    match field {
        ConfigField::Count => "count",
        ConfigField::Spread => "spread (rad)",
        ConfigField::SpeedMin => "speed_min",
        ConfigField::SpeedMax => "speed_max",
        ConfigField::LifetimeMin => "lifetime_min",
        ConfigField::LifetimeMax => "lifetime_max",
        ConfigField::SpawnInterval => "spawn_interval",
    }
}

/// Formatted current value of a config field.
fn field_value(config: &SandboxConfig, field: ConfigField) -> String {
    match field {
        ConfigField::Count => format!("{}", config.params.count),
        ConfigField::Spread => format!("{:.2}", config.params.spread),
        ConfigField::SpeedMin => format!("{:.1}", config.params.speed.0),
        ConfigField::SpeedMax => format!("{:.1}", config.params.speed.1),
        ConfigField::LifetimeMin => format!("{:.1}", config.params.lifetime.0),
        ConfigField::LifetimeMax => format!("{:.1}", config.params.lifetime.1),
        ConfigField::SpawnInterval => format!("{}ms", config.spawn_interval.as_millis()),
    }
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
    let mut last_mouse: Option<(u16, u16)> = None;
    let mut last = Instant::now();

    // The sandbox follows the mouse, so opt into mouse events for this run.
    execute!(stdout(), EnableMouseCapture)?;

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
            let hint = format!(
                " Sandbox | Effect: {kind_name} | Follows mouse | Tab: cycle | q/Esc: quit "
            );
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
        // Block up to one frame for the first event (this paces the loop), then
        // drain the rest so rapid mouse motion doesn't lag or back up the queue.
        if event::poll(FRAME_TIME)? {
            loop {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        match map_sandbox_key(key.code) {
                            SandboxCommand::Quit => {
                                execute!(stdout(), DisableMouseCapture)?;
                                return Ok(());
                            }
                            SandboxCommand::CycleEffect => {
                                kind_idx = next_kind(kind_idx, EFFECT_KINDS.len());
                            }
                            SandboxCommand::Ignore => {}
                        }
                    }
                    // Track the latest cursor cell — bursts spawn here.
                    Event::Mouse(me) => last_mouse = Some((me.column, me.row)),
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }

        // --- auto-spawn at the mouse (or center until the mouse first moves) ---
        if let Some(area) = last_area {
            let (count, remainder) = cadence_step(spawn_acc, dt, SPAWN_INTERVAL);
            spawn_acc = remainder;
            let origin = spawn_origin(area, last_mouse);
            for _ in 0..count {
                spawn(
                    EFFECT_KINDS[kind_idx],
                    origin,
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

    // --- spawn_origin ---

    #[test]
    fn spawn_origin_uses_mouse_relative_to_area() {
        // Body starts at (0, 1); cursor at absolute (30, 11) is (30, 10) body-relative.
        let area = Rect::new(0, 1, 80, 23);
        let (x, y) = spawn_origin(area, Some((30, 11)));
        assert!((x - 30.0).abs() < 1e-4, "x: {x}");
        assert!((y - 10.0).abs() < 1e-4, "y: {y}");
    }

    #[test]
    fn spawn_origin_falls_back_to_center_without_mouse() {
        // No mouse yet → body center, expressed relative to the body origin.
        // area center abs = (40, 13); relative = (40, 12).
        let area = Rect::new(0, 1, 80, 24);
        let (x, y) = spawn_origin(area, None);
        assert!((x - 40.0).abs() < 1e-4, "x: {x}");
        assert!((y - 12.0).abs() < 1e-4, "y: {y}");
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
