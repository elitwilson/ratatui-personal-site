use crate::input::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use std::time::Duration;

const INDIGO_BG: Color = Color::Rgb(30, 27, 75);
const BRIGHT_GREEN: Color = Color::Rgb(61, 224, 53);
const DIM_GREEN: Color = Color::Rgb(42, 168, 74);
const SECONDARY_BLUE: Color = Color::Rgb(127, 143, 217);
const PANEL_TEXT: Color = Color::Rgb(191, 233, 189);

/// POST boot log, typed out one character at a time. The last line is the
/// final prompt and renders in bright green.
const BOOT_LOG: [&str; 5] = [
    "> CHECKING MEMORY...",
    "> LOADING MODULES...",
    "> INITIALIZING DISPLAY...",
    "> MOUNTING DRIVES...",
    "> READY.",
];

/// Milliseconds per revealed character in the boot-log typewriter.
const BOOT_CHAR_MS: u128 = 22;
/// Half-period of the cursor blink, in milliseconds (≈1s on/off cycle).
const CURSOR_BLINK_MS: u128 = 500;

#[derive(Debug, PartialEq, Eq)]
pub enum AboutCommand {
    Back,
    Ignore,
}

pub fn map_about_key(code: KeyCode) -> AboutCommand {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => AboutCommand::Back,
        _ => AboutCommand::Ignore,
    }
}

/// Renders a bright-green bordered panel with an inverted header bar.
/// Returns the body rect (inner area below the header row) for content.
fn render_panel(frame: &mut Frame, area: Rect, title: &str) -> Rect {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BRIGHT_GREEN).bg(INDIGO_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let header_row = Rect { height: 1, ..inner };
    frame.render_widget(
        Paragraph::new(title).style(Style::default().fg(INDIGO_BG).bg(BRIGHT_GREEN)),
        header_row,
    );

    Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    }
}

/// The PROFILE.TXT bio prose, authored as plain paragraphs separated by blank
/// lines in `content/bio.txt`. Editing the bio means editing that file — no
/// `Line::styled` boilerplate — and is baked in at build time via `include_str!`.
const BIO: &str = include_str!("../content/bio.txt");

/// Split the bio into paragraphs (on blank lines) and render each as a single
/// styled line with an empty spacer between them; `Paragraph`'s `Wrap` handles
/// the visual line breaks. Source-file line wrapping within a paragraph is
/// collapsed to spaces so the author can hard-wrap for editing comfort.
fn bio_lines(style: Style) -> Vec<Line<'static>> {
    let paragraphs: Vec<String> = BIO
        .split("\n\n")
        .map(|para| para.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|para| !para.is_empty())
        .collect();

    let mut lines = Vec::with_capacity(paragraphs.len() * 2);
    for (i, para) in paragraphs.into_iter().enumerate() {
        if i > 0 {
            lines.push(Line::styled("", style));
        }
        lines.push(Line::styled(para, style));
    }
    lines
}

/// Senior-engineer capabilities, rendered as "loaded modules" in SKILLS.SYS.
/// These lead with judgment over syntax — the part of the job an agent doesn't
/// do for you.
const CAPABILITIES: [&str; 5] = [
    "System Architecture",
    "Agentic AI Workflows",
    "API & Pipeline Design",
    "Problem Decomposition",
    "Research → Delivery",
];

/// A BIOS-style capability line: `LABEL ..... OK`, with the dotted leader sized
/// to right-align `OK` against the panel's inner width.
fn capability_line(name: &str, width: u16) -> Line<'static> {
    let status = "OK";
    let used = name.chars().count() + status.chars().count() + 2; // spaces flanking the dots
    let dots = (width as usize).saturating_sub(used).max(1);
    Line::from(vec![
        Span::styled(
            name.to_string(),
            Style::default().fg(PANEL_TEXT).bg(INDIGO_BG),
        ),
        Span::styled(
            format!(" {} ", ".".repeat(dots)),
            Style::default().fg(DIM_GREEN).bg(INDIGO_BG),
        ),
        Span::styled(status, Style::default().fg(BRIGHT_GREEN).bg(INDIGO_BG)),
    ])
}

/// Whether the blink cursor is in its "on" (visible) half of the cycle.
fn cursor_visible(elapsed: Duration) -> bool {
    (elapsed.as_millis() / CURSOR_BLINK_MS) % 2 == 0
}

/// A blinking block cursor. Renders a space when "off" so the line width — and
/// thus the text to its left — never shifts as it blinks.
fn cursor_span(on: bool) -> Span<'static> {
    let glyph = if on { "█" } else { " " };
    Span::styled(glyph, Style::default().fg(BRIGHT_GREEN).bg(INDIGO_BG))
}

/// Build the boot-log lines revealed up to `elapsed`: characters appear one at a
/// time, line after line, with a blinking cursor trailing the active line.
/// Lines not yet reached are omitted (leaving their rows blank).
fn boot_lines(elapsed: Duration) -> Vec<Line<'static>> {
    let dim = Style::default().fg(DIM_GREEN).bg(INDIGO_BG);
    let bright = Style::default().fg(BRIGHT_GREEN).bg(INDIGO_BG);
    let cursor_on = cursor_visible(elapsed);
    let mut budget = (elapsed.as_millis() / BOOT_CHAR_MS) as usize;

    let mut lines = Vec::with_capacity(BOOT_LOG.len());
    for (i, text) in BOOT_LOG.iter().enumerate() {
        let last = i == BOOT_LOG.len() - 1;
        let style = if last { bright } else { dim };
        let len = text.chars().count();

        if budget >= len {
            budget -= len;
            // The cursor rests on the final line once the whole log is typed;
            // intermediate completed lines carry no cursor.
            if last {
                lines.push(Line::from(vec![
                    Span::styled(*text, style),
                    cursor_span(cursor_on),
                ]));
            } else {
                lines.push(Line::styled(*text, style));
            }
        } else {
            // The line currently being typed: partial text + trailing cursor.
            let shown: String = text.chars().take(budget).collect();
            lines.push(Line::from(vec![
                Span::styled(shown, style),
                cursor_span(cursor_on),
            ]));
            break;
        }
    }
    lines
}

/// The About screen's only state is `elapsed`: time since entry, which drives
/// the boot-log typewriter and the blinking cursor. `tick` advances it; entering
/// the screen builds a fresh `About`, so the animation replays each visit.
#[derive(Default)]
pub struct About {
    elapsed: Duration,
}

impl About {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, dt: Duration) {
        self.elapsed += dt;
    }

    pub fn handle_key(&mut self, code: KeyCode) -> Option<crate::Nav> {
        match map_about_key(code) {
            AboutCommand::Back => Some(crate::Nav::To(crate::Screen::Menu)),
            AboutCommand::Ignore => None,
        }
    }

    pub fn render(&mut self, frame: &mut Frame) -> ratatui::layout::Rect {
        let area = frame.area();

        // Paint indigo background over the full area first
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(INDIGO_BG)),
            area,
        );

        // Vertical layout. The web terminal fills the browser viewport, so its
        // height is dynamic rather than a fixed 24 rows: most sections take a
        // fixed height, PROFILE.TXT is tall enough for the wrapped bio, and a
        // flexible spacer absorbs any leftover height so the footer pins to the
        // bottom.
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // BIOS header bar
                Constraint::Length(1),  // gap
                Constraint::Length(5),  // POST boot log
                Constraint::Length(1),  // gap
                Constraint::Length(2),  // ELI WILSON title + subtitle
                Constraint::Length(1),  // gap
                Constraint::Length(14), // PROFILE.TXT panel
                Constraint::Length(1),  // gap
                Constraint::Length(6),  // two-column SKILLS.SYS + CAREER.LOG
                Constraint::Min(0),     // flexible spacer
                Constraint::Length(1),  // footer hint row
            ])
            .split(area);

        // --- BIOS header bar ---
        let header_style = Style::default().fg(SECONDARY_BLUE).bg(INDIGO_BG);
        let header_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);
        frame.render_widget(
            Paragraph::new("WILSON-BIOS (C) 1983  v2.6.0").style(header_style),
            header_cols[0],
        );
        frame.render_widget(
            Paragraph::new("SYS: CHARLO 2600 / RATATUI WASM")
                .style(header_style)
                .alignment(Alignment::Right),
            header_cols[1],
        );

        // --- POST boot log (typed out character by character) ---
        frame.render_widget(
            Paragraph::new(boot_lines(self.elapsed)).style(Style::default().bg(INDIGO_BG)),
            rows[2],
        );

        // --- ELI WILSON title block ---
        let title_lines = vec![
            Line::styled(
                "ELI WILSON",
                Style::default()
                    .fg(BRIGHT_GREEN)
                    .bg(INDIGO_BG)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::styled(
                "// SOFTWARE DEVELOPER  ·  PLAYER 1",
                Style::default().fg(SECONDARY_BLUE).bg(INDIGO_BG),
            ),
        ];
        frame.render_widget(Paragraph::new(title_lines), rows[4]);

        // --- PROFILE.TXT panel ---
        let profile_body = render_panel(frame, rows[6], "PROFILE.TXT");
        let bio_style = Style::default().fg(PANEL_TEXT).bg(INDIGO_BG);
        frame.render_widget(
            Paragraph::new(bio_lines(bio_style))
                .wrap(Wrap { trim: true })
                .style(Style::default().bg(INDIGO_BG)),
            profile_body,
        );

        // --- Two-column SKILLS.SYS + CAREER.LOG ---
        let two_col = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[8]);

        let skills_body = render_panel(frame, two_col[0], "SKILLS.SYS");
        let panel_style = Style::default().fg(PANEL_TEXT).bg(INDIGO_BG);
        let skill_lines: Vec<Line> = CAPABILITIES
            .iter()
            .map(|name| capability_line(name, skills_body.width))
            .collect();
        frame.render_widget(Paragraph::new(skill_lines), skills_body);

        let career_body = render_panel(frame, two_col[1], "CAREER.LOG");
        let career_lines = vec![
            Line::styled(
                "2022–NOW  · App Programmer Intermediate · Michigan Medicine",
                panel_style,
            ),
            Line::styled(
                "2007–2022 · Psychometrist               · Michigan Medicine",
                panel_style,
            ),
        ];
        frame.render_widget(Paragraph::new(career_lines), career_body);

        // --- Footer hint row ---
        frame.render_widget(
            Paragraph::new("↑/↓  w/s  scroll  ·  Esc  back to menu  ·  q  quit")
                .style(Style::default().fg(SECONDARY_BLUE).bg(INDIGO_BG)),
            rows[10],
        );

        // Paint gap and spacer rows with indigo bg
        for &gap in &[rows[1], rows[3], rows[5], rows[7], rows[9]] {
            frame.render_widget(Block::default().style(Style::default().bg(INDIGO_BG)), gap);
        }

        area
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_maps_to_back() {
        assert_eq!(map_about_key(KeyCode::Esc), AboutCommand::Back);
    }

    #[test]
    fn q_maps_to_back() {
        assert_eq!(map_about_key(KeyCode::Char('q')), AboutCommand::Back);
    }

    #[test]
    fn unknown_key_maps_to_ignore() {
        assert_eq!(map_about_key(KeyCode::Enter), AboutCommand::Ignore);
        assert_eq!(map_about_key(KeyCode::Char('z')), AboutCommand::Ignore);
    }

    /// Concatenate a line's span contents into a single string.
    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn boot_log_reveals_one_character_at_a_time() {
        // 22ms/char: after one line (20 chars ≈ 440ms) plus 5 more chars of the
        // second line, line one is fully typed and line two is mid-word.
        let elapsed = Duration::from_millis(BOOT_CHAR_MS as u64 * 25);
        let lines = boot_lines(elapsed);
        assert_eq!(lines.len(), 2, "only the typed-so-far lines should appear");
        assert_eq!(
            line_text(&lines[0]).trim_end_matches(['█', ' ']),
            BOOT_LOG[0]
        );
        assert!(line_text(&lines[1]).starts_with("> LOA"));
    }

    #[test]
    fn boot_log_completes_with_all_lines() {
        let lines = boot_lines(Duration::from_secs(10));
        assert_eq!(lines.len(), BOOT_LOG.len());
        assert!(line_text(&lines[BOOT_LOG.len() - 1]).contains("> READY."));
    }

    #[test]
    fn bio_paragraphs_are_separated_by_blank_spacer_lines() {
        let lines = bio_lines(Style::default());
        // Four paragraphs in content/bio.txt → 4 prose lines + 3 spacers.
        assert_eq!(lines.len(), 7);
        assert!(line_text(&lines[1]).is_empty(), "spacer between paragraphs");
        // Hard wrapping in the source file is collapsed to single spaces, so a
        // phrase split across two file lines renders as one continuous string.
        assert!(line_text(&lines[0]).contains("choose-your-own-adventure games in BASIC"));
        assert!(!line_text(&lines[0]).contains('\n'));
    }

    #[test]
    fn cursor_blinks_on_and_off() {
        assert!(cursor_visible(Duration::ZERO));
        assert!(!cursor_visible(Duration::from_millis(
            CURSOR_BLINK_MS as u64
        )));
        assert!(cursor_visible(Duration::from_millis(
            CURSOR_BLINK_MS as u64 * 2
        )));
    }
}
