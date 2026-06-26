use crate::input::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

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
    ScrollUp,
    ScrollDown,
    FocusNextLink,
    OpenFocusedLink,
    Ignore,
}

pub fn map_about_key(code: KeyCode) -> AboutCommand {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => AboutCommand::Back,
        KeyCode::Up | KeyCode::Char('w') => AboutCommand::ScrollUp,
        KeyCode::Down | KeyCode::Char('s') => AboutCommand::ScrollDown,
        KeyCode::Tab => AboutCommand::FocusNextLink,
        KeyCode::Enter => AboutCommand::OpenFocusedLink,
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

/// The PROFILE.TXT bio prose, authored in `content/bio.txt`. Paragraphs are
/// separated by blank lines; inline links use Markdown syntax `[label](url)`.
/// Editing the bio — including adding, moving, or removing links — means editing
/// that file, baked in at build time via `include_str!`.
const BIO: &str = include_str!("../content/bio.txt");

/// A link parsed from the bio. Both fields are `'static` slices of the embedded
/// content, so the URL flows straight into `Nav::OpenUrl` without allocation.
#[derive(Debug, PartialEq, Eq)]
struct BioLink {
    label: &'static str,
    url: &'static str,
}

/// One piece of a rendered bio paragraph: a run of prose, or a link carrying its
/// global index (so focus styling can single it out).
enum BioSegment {
    Text(String),
    Link { index: usize, label: &'static str },
}

/// The bio as paragraphs of segments, plus the flat, document-order list of
/// links — the order Tab cycles through.
struct ParsedBio {
    paragraphs: Vec<Vec<BioSegment>>,
    links: Vec<BioLink>,
}

/// Collapse every run of whitespace to a single space, preserving a single
/// leading/trailing space if present. Lets the author hard-wrap paragraphs in
/// the source file while keeping the spacing around inline links intact.
fn collapse_ws(s: &str) -> String {
    let mut out = String::new();
    let mut prev_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            out.push(c);
            prev_ws = false;
        }
    }
    out
}

/// Read a `[label](url)` beginning at `open` (the index of `[`). Returns
/// `(before, label, url, after)` on success, or `None` if it isn't a well-formed
/// link. Operates on `&'static str` so the extracted URL stays `'static`.
fn parse_link_at(
    s: &'static str,
    open: usize,
) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    let close = open + 1 + s[open + 1..].find(']')?;
    let after_label = s[close + 1..].strip_prefix('(')?;
    let url_len = after_label.find(')')?;
    Some((
        &s[..open],
        &s[open + 1..close],
        &after_label[..url_len],
        &after_label[url_len + 1..],
    ))
}

/// Parse bio content into paragraphs of segments and a flat link list. A
/// well-formed `[label](url)` becomes a link; an unmatched `[` is left as
/// literal text.
fn parse_bio(content: &'static str) -> ParsedBio {
    let mut links = Vec::new();
    let mut paragraphs = Vec::new();

    for para in content.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        let mut segs = Vec::new();
        let mut rest = para;
        while let Some(open) = rest.find('[') {
            match parse_link_at(rest, open) {
                Some((before, label, url, after)) => {
                    if !before.is_empty() {
                        segs.push(BioSegment::Text(collapse_ws(before)));
                    }
                    segs.push(BioSegment::Link {
                        index: links.len(),
                        label,
                    });
                    links.push(BioLink { label, url });
                    rest = after;
                }
                None => {
                    // Not a link: keep the text through the '[' as literal.
                    segs.push(BioSegment::Text(collapse_ws(&rest[..=open])));
                    rest = &rest[open + 1..];
                }
            }
        }
        if !rest.is_empty() {
            segs.push(BioSegment::Text(collapse_ws(rest)));
        }
        paragraphs.push(segs);
    }

    ParsedBio { paragraphs, links }
}

/// The bio's links in document order — the sequence Tab cycles through.
fn bio_links() -> Vec<BioLink> {
    parse_bio(BIO).links
}

/// Black underlined text on green — dim when idle, bright when active (the
/// keyboard-focused link or the one under the mouse cursor).
fn link_style(active: bool) -> Style {
    let bg = if active { BRIGHT_GREEN } else { DIM_GREEN };
    Style::default()
        .fg(Color::Black)
        .bg(bg)
        .add_modifier(Modifier::UNDERLINED)
}

/// A link reads as "active" (bright) when it's the keyboard-focused link or the
/// one the mouse is hovering.
fn link_active(link: Option<usize>, focused: Option<usize>, hovered: Option<usize>) -> bool {
    link.is_some() && (link == focused || link == hovered)
}

/// A single word and which link (if any) it belongs to. Wrapping operates on
/// words; a multi-word link label contributes several link-tagged words.
struct Word {
    text: String,
    link: Option<usize>,
}

/// On-screen extent of (part of) a link within the wrapped bio, in content-row
/// coordinates (row 0 = first wrapped line, before scrolling). A link that
/// straddles a wrap boundary yields one rect per row it occupies.
struct LinkRect {
    index: usize,
    row: u16,
    col: u16,
    width: u16,
}

/// The bio wrapped to a fixed width: the visual lines to render, plus the link
/// hit-rects that align with them. We do the wrapping ourselves (rather than
/// `Paragraph`'s `Wrap`) precisely so we know where each link landed — which is
/// what makes inline links both clickable and highlightable.
struct WrappedBio {
    lines: Vec<Line<'static>>,
    rects: Vec<LinkRect>,
}

/// Flatten a paragraph's segments into wrap-ready words, tagging each with its
/// link index. Whitespace is normalized by the split.
fn paragraph_words(segs: &[BioSegment]) -> Vec<Word> {
    let mut words = Vec::new();
    for seg in segs {
        match seg {
            BioSegment::Text(text) => words.extend(text.split_whitespace().map(|w| Word {
                text: w.to_string(),
                link: None,
            })),
            BioSegment::Link { index, label } => {
                words.extend(label.split_whitespace().map(|w| Word {
                    text: w.to_string(),
                    link: Some(*index),
                }))
            }
        }
    }
    words
}

/// Record a link cell, merging it into the previous rect when it's contiguous on
/// the same row (so a multi-word link reads as one clickable chip).
fn push_link_cell(rects: &mut Vec<LinkRect>, index: usize, row: u16, col: u16, width: u16) {
    if let Some(last) = rects.last_mut() {
        if last.index == index && last.row == row && last.col + last.width == col {
            last.width += width;
            return;
        }
    }
    rects.push(LinkRect {
        index,
        row,
        col,
        width,
    });
}

/// Greedy word-wrap the bio to `width`, producing visual lines and link rects.
/// Paragraphs are separated by a blank line. The link at `focused` is drawn
/// bright; spaces *inside* a link carry the link style so the chip is unbroken.
fn wrap_bio_content(
    content: &'static str,
    focused: Option<usize>,
    hovered: Option<usize>,
    prose: Style,
    width: u16,
) -> WrappedBio {
    let width = width.max(1);
    let parsed = parse_bio(content);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut rects: Vec<LinkRect> = Vec::new();

    for (p, segs) in parsed.paragraphs.iter().enumerate() {
        if p > 0 {
            lines.push(Line::styled("", prose));
        }
        let mut spans: Vec<Span> = Vec::new();
        let mut col: u16 = 0;
        let mut prev_link: Option<usize> = None;

        for word in paragraph_words(segs) {
            let wlen = word.text.chars().count() as u16;
            if col > 0 && col + 1 + wlen > width {
                lines.push(Line::from(std::mem::take(&mut spans)));
                col = 0;
                prev_link = None;
            }
            // Inter-word space (omitted at line start). It belongs to the link
            // only when it sits between two words of the *same* link.
            if col > 0 {
                let in_link = word.link.is_some() && word.link == prev_link;
                let style = if in_link {
                    link_style(link_active(word.link, focused, hovered))
                } else {
                    prose
                };
                spans.push(Span::styled(" ", style));
                if in_link {
                    push_link_cell(&mut rects, word.link.unwrap(), lines.len() as u16, col, 1);
                }
                col += 1;
            }
            let style = match word.link {
                Some(i) => link_style(link_active(Some(i), focused, hovered)),
                None => prose,
            };
            spans.push(Span::styled(word.text, style));
            if let Some(i) = word.link {
                push_link_cell(&mut rects, i, lines.len() as u16, col, wlen);
            }
            col += wlen;
            prev_link = word.link;
        }
        lines.push(Line::from(spans));
    }

    WrappedBio { lines, rects }
}

/// Advance link focus to the next link, wrapping around; from `None` (nothing
/// focused) it moves to the first. With no links, focus stays `None`.
fn next_focus(current: Option<usize>, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(match current {
        None => 0,
        Some(i) => (i + 1) % count,
    })
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
    /// Vertical scroll offset (in rows) into the PROFILE.TXT bio. Incremented by
    /// scroll keys; clamped to the content's overflow at render time, once the
    /// panel's visible height is known.
    scroll: u16,
    /// Index of the currently focused inline bio link (Tab cycles, Enter opens).
    /// `None` until the first Tab.
    link_focus: Option<usize>,
    /// Set when focus changes via Tab, so the next render scrolls the focused
    /// link into view (cleared once handled).
    ensure_focus_visible: bool,
    /// Screen rects of the links visible in the last render, paired with their
    /// link index — used to hit-test mouse clicks and hover. Rebuilt every frame.
    link_hitboxes: Vec<(usize, ratatui::layout::Rect)>,
    /// Last known mouse cell, for the hover highlight. `None` until the mouse
    /// moves over the terminal.
    mouse: Option<(u16, u16)>,
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
            AboutCommand::ScrollUp => {
                self.scroll = self.scroll.saturating_sub(1);
                None
            }
            AboutCommand::ScrollDown => {
                self.scroll = self.scroll.saturating_add(1);
                None
            }
            AboutCommand::FocusNextLink => {
                self.link_focus = next_focus(self.link_focus, bio_links().len());
                self.ensure_focus_visible = true;
                None
            }
            AboutCommand::OpenFocusedLink => self
                .link_focus
                .and_then(|i| bio_links().into_iter().nth(i))
                .map(|link| crate::Nav::OpenUrl(link.url)),
            AboutCommand::Ignore => None,
        }
    }

    /// Handle a left-click at terminal cell `(col, row)`. If it lands on a
    /// visible bio link, focus it and open its URL.
    pub fn handle_click(&mut self, pos: (u16, u16)) -> Option<crate::Nav> {
        let (col, row) = pos;
        let hit = self.link_hitboxes.iter().find(|(_, rect)| {
            col >= rect.x
                && col < rect.x + rect.width
                && row >= rect.y
                && row < rect.y + rect.height
        })?;
        let index = hit.0;
        let url = bio_links().into_iter().nth(index)?.url;
        self.link_focus = Some(index);
        Some(crate::Nav::OpenUrl(url))
    }

    /// Record the latest cursor cell, driving the bio link hover highlight.
    pub fn set_mouse(&mut self, pos: (u16, u16)) {
        self.mouse = Some(pos);
    }

    /// The link index under the mouse, if any, using the previous frame's
    /// hitboxes. A one-frame lag on the highlight is imperceptible.
    fn hovered_link(&self) -> Option<usize> {
        let (col, row) = self.mouse?;
        self.link_hitboxes
            .iter()
            .find(|(_, rect)| {
                col >= rect.x
                    && col < rect.x + rect.width
                    && row >= rect.y
                    && row < rect.y + rect.height
            })
            .map(|(index, _)| *index)
    }

    pub fn render(&mut self, frame: &mut Frame) -> ratatui::layout::Rect {
        let area = frame.area();
        let bio_style = Style::default().fg(PANEL_TEXT).bg(INDIGO_BG);

        // Wrap the bio ourselves at the panel's inner width so the panel can grow
        // to fit its content *and* we know where each link landed (for clicks and
        // highlighting). Inner width is the full width minus the panel's
        // left/right borders; the panel adds the header row plus top/bottom
        // borders (3 rows) on top of the content. The panel spans the full width,
        // so its body width equals this measurement width.
        let bio_inner_width = area.width.saturating_sub(2);
        let hovered = self.hovered_link();
        let wrapped = wrap_bio_content(BIO, self.link_focus, hovered, bio_style, bio_inner_width);
        let bio_content_h = wrapped.lines.len() as u16;
        let profile_h = bio_content_h.saturating_add(3);

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
                Constraint::Max(profile_h), // PROFILE.TXT panel (grows with bio)
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
        let visible_h = profile_body.height;

        // A fresh Tab focus scrolls the focused link into view before we clamp.
        if self.ensure_focus_visible {
            if let Some(focus) = self.link_focus {
                if let Some(row) = wrapped
                    .rects
                    .iter()
                    .filter(|r| r.index == focus)
                    .map(|r| r.row)
                    .min()
                {
                    if row < self.scroll {
                        self.scroll = row;
                    } else if visible_h > 0 && row >= self.scroll + visible_h {
                        self.scroll = row - visible_h + 1;
                    }
                }
            }
            self.ensure_focus_visible = false;
        }

        // Clamp scroll to the actual overflow so it can't run past the end (or
        // scroll at all when the bio fits).
        let max_scroll = bio_content_h.saturating_sub(visible_h);
        self.scroll = self.scroll.min(max_scroll);

        // Translate the on-screen link rects (those within the scrolled viewport)
        // into absolute terminal cells for click hit-testing next frame.
        self.link_hitboxes.clear();
        for r in &wrapped.rects {
            if r.row < self.scroll || r.row >= self.scroll + visible_h {
                continue;
            }
            let x = profile_body.x + r.col;
            let max_w = (profile_body.x + profile_body.width).saturating_sub(x);
            let width = r.width.min(max_w);
            if width == 0 {
                continue;
            }
            self.link_hitboxes.push((
                r.index,
                ratatui::layout::Rect {
                    x,
                    y: profile_body.y + (r.row - self.scroll),
                    width,
                    height: 1,
                },
            ));
        }

        // Lines are pre-wrapped to the body width, so render without `Wrap`.
        frame.render_widget(
            Paragraph::new(wrapped.lines)
                .scroll((self.scroll, 0))
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
            Paragraph::new("↑/↓  w/s  scroll  ·  Tab/↵  links  ·  Esc  back  ·  q  quit")
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
        assert_eq!(map_about_key(KeyCode::Char('z')), AboutCommand::Ignore);
        assert_eq!(map_about_key(KeyCode::Left), AboutCommand::Ignore);
    }

    #[test]
    fn link_keys_map_to_link_commands() {
        assert_eq!(map_about_key(KeyCode::Tab), AboutCommand::FocusNextLink);
        assert_eq!(map_about_key(KeyCode::Enter), AboutCommand::OpenFocusedLink);
    }

    #[test]
    fn next_focus_cycles_and_wraps() {
        assert_eq!(next_focus(None, 0), None, "no links → stays unfocused");
        assert_eq!(next_focus(None, 3), Some(0), "first Tab focuses the first link");
        assert_eq!(next_focus(Some(0), 3), Some(1));
        assert_eq!(next_focus(Some(2), 3), Some(0), "wraps past the last link");
    }

    #[test]
    fn parse_extracts_inline_link_with_surrounding_prose() {
        let parsed = parse_bio("Built [SPI](https://spi.test) for science.");
        assert_eq!(
            parsed.links,
            vec![BioLink {
                label: "SPI",
                url: "https://spi.test"
            }]
        );
        let segs = &parsed.paragraphs[0];
        assert_eq!(segs.len(), 3);
        assert!(matches!(&segs[0], BioSegment::Text(t) if t == "Built "));
        assert!(matches!(&segs[1], BioSegment::Link { index: 0, label: "SPI" }));
        assert!(matches!(&segs[2], BioSegment::Text(t) if t == " for science."));
    }

    #[test]
    fn parse_indexes_multiple_links_in_document_order() {
        let parsed = parse_bio("[A](a) and [B](b)");
        assert_eq!(parsed.links.len(), 2);
        assert_eq!(parsed.links[0].url, "a");
        assert_eq!(parsed.links[1].url, "b");
    }

    #[test]
    fn parse_leaves_unmatched_bracket_as_literal() {
        let parsed = parse_bio("see [this] thing");
        assert!(parsed.links.is_empty());
        let text: String = parsed.paragraphs[0]
            .iter()
            .map(|seg| match seg {
                BioSegment::Text(t) => t.clone(),
                BioSegment::Link { label, .. } => label.to_string(),
            })
            .collect();
        assert_eq!(text, "see [this] thing");
    }

    #[test]
    fn enter_opens_focused_link_only_when_a_link_is_focused() {
        let mut about = About::new();
        // Nothing focused yet → Enter does nothing.
        assert!(about.handle_key(KeyCode::Enter).is_none());
        // With links present, Tab then Enter yields an OpenUrl nav; with none,
        // Tab leaves focus None and Enter still does nothing. Assert against
        // whichever the real bio currently has.
        about.handle_key(KeyCode::Tab);
        match bio_links().first() {
            Some(first) => assert_eq!(
                about.handle_key(KeyCode::Enter),
                Some(crate::Nav::OpenUrl(first.url))
            ),
            None => assert!(about.handle_key(KeyCode::Enter).is_none()),
        }
    }

    #[test]
    fn scroll_keys_map_to_scroll_commands() {
        assert_eq!(map_about_key(KeyCode::Up), AboutCommand::ScrollUp);
        assert_eq!(map_about_key(KeyCode::Char('w')), AboutCommand::ScrollUp);
        assert_eq!(map_about_key(KeyCode::Down), AboutCommand::ScrollDown);
        assert_eq!(map_about_key(KeyCode::Char('s')), AboutCommand::ScrollDown);
    }

    #[test]
    fn scroll_up_saturates_at_top() {
        let mut about = About::new();
        // Already at the top; scrolling up stays put rather than underflowing.
        assert!(about.handle_key(KeyCode::Up).is_none());
        assert_eq!(about.scroll, 0);
        // Scrolling down advances the offset.
        about.handle_key(KeyCode::Down);
        assert_eq!(about.scroll, 1);
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
    fn link_active_when_focused_or_hovered() {
        // Bright when this link is the focused one or the hovered one.
        assert!(link_active(Some(1), Some(1), None), "focused");
        assert!(link_active(Some(1), None, Some(1)), "hovered");
        assert!(link_active(Some(1), Some(0), Some(1)), "hovered while another focused");
        assert!(!link_active(Some(1), Some(0), Some(2)), "neither");
        assert!(!link_active(None, Some(0), None), "prose is never a link");
    }

    #[test]
    fn wrap_separates_paragraphs_with_a_blank_line() {
        let wrapped =
            wrap_bio_content("First para.\n\nSecond para.", None, None, Style::default(), 80);
        assert_eq!(wrapped.lines.len(), 3, "para, spacer, para");
        assert_eq!(line_text(&wrapped.lines[0]), "First para.");
        assert_eq!(line_text(&wrapped.lines[1]), "");
        assert_eq!(line_text(&wrapped.lines[2]), "Second para.");
    }

    #[test]
    fn wrap_breaks_a_long_paragraph_across_lines() {
        // Width 9 can't hold "one two three" on one row.
        let wrapped = wrap_bio_content("one two three four", None, None, Style::default(), 9);
        assert!(wrapped.lines.len() > 1);
        for line in &wrapped.lines {
            assert!(line_text(line).chars().count() <= 9);
        }
    }

    #[test]
    fn wrap_records_a_link_rect_at_its_rendered_position() {
        let wrapped =
            wrap_bio_content("Go to [GitHub](https://gh) now", None, None, Style::default(), 80);
        let rect = wrapped.rects.iter().find(|r| r.index == 0).expect("link rect");
        // "Go to " = 6 cells, then the 6-cell "GitHub" chip on row 0.
        assert_eq!((rect.row, rect.col, rect.width), (0, 6, 6));
    }

    #[test]
    fn wrap_keeps_a_multiword_link_as_one_contiguous_chip() {
        let wrapped = wrap_bio_content(
            "at [University of Michigan](u) today",
            None,
            None,
            Style::default(),
            80,
        );
        let rects: Vec<_> = wrapped.rects.iter().filter(|r| r.index == 0).collect();
        assert_eq!(rects.len(), 1, "one merged rect, not three word rects");
        // "at " = 3 cells; "University of Michigan" = 22 cells incl. inner spaces.
        assert_eq!((rects[0].col, rects[0].width), (3, 22));
    }

    #[test]
    fn collapse_ws_flattens_hard_wraps_but_keeps_boundary_spaces() {
        // Newlines and runs of spaces become single spaces; a leading/trailing
        // space (e.g. either side of an inline link) is preserved.
        assert_eq!(collapse_ws("a\n  b   c"), "a b c");
        assert_eq!(collapse_ws(" joined "), " joined ");
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
