use crate::input::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

const INDIGO_BG: Color = Color::Rgb(30, 27, 75);
const BRIGHT_GREEN: Color = Color::Rgb(61, 224, 53);
const DIM_GREEN: Color = Color::Rgb(42, 168, 74);
const SECONDARY_BLUE: Color = Color::Rgb(127, 143, 217);
const PANEL_TEXT: Color = Color::Rgb(191, 233, 189);

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

/// The About screen carries no state — it's a static page until a key sends it
/// back to the menu. A unit struct keeps it uniform with the other screens so
/// the router can drive all four the same way.
pub struct About;

impl About {
    pub fn handle_key(&mut self, code: KeyCode) -> Option<crate::Nav> {
        match map_about_key(code) {
            AboutCommand::Back => Some(crate::Nav::To(crate::Screen::Menu)),
            AboutCommand::Ignore => None,
        }
    }

    pub fn render(&self, frame: &mut Frame) -> ratatui::layout::Rect {
        let area = frame.area();

        // Paint indigo background over the full area first
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(INDIGO_BG)),
            area,
        );

        // Vertical layout: 10 rows fitting within 24 terminal rows
        // 1 + 1 + 5 + 1 + 2 + 1 + 5 + 1 + 6 + 1 = 24
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // BIOS header bar
                Constraint::Length(1), // gap
                Constraint::Length(5), // POST boot log
                Constraint::Length(1), // gap
                Constraint::Length(2), // ELI WILSON title + subtitle
                Constraint::Length(1), // gap
                Constraint::Length(5), // PROFILE.TXT panel
                Constraint::Length(1), // gap
                Constraint::Length(6), // two-column SKILLS.SYS + CAREER.LOG
                Constraint::Length(1), // footer hint row
            ])
            .split(area);

        // --- BIOS header bar ---
        let header_style = Style::default().fg(SECONDARY_BLUE).bg(INDIGO_BG);
        let header_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);
        frame.render_widget(
            Paragraph::new("WILSON-BIOS (C) 1981  v2.6.0").style(header_style),
            header_cols[0],
        );
        frame.render_widget(
            Paragraph::new("SYS: ATARI 2600 / RATATUI WASM")
                .style(header_style)
                .alignment(Alignment::Right),
            header_cols[1],
        );

        // --- POST boot log ---
        let dim_style = Style::default().fg(DIM_GREEN).bg(INDIGO_BG);
        let bright_style = Style::default().fg(BRIGHT_GREEN).bg(INDIGO_BG);
        let boot_lines = vec![
            Line::styled("> CHECKING MEMORY...", dim_style),
            Line::styled("> LOADING MODULES...", dim_style),
            Line::styled("> INITIALIZING DISPLAY...", dim_style),
            Line::styled("> MOUNTING DRIVES...", dim_style),
            Line::from(vec![
                Span::styled("> READY.", bright_style),
                Span::styled(" █", bright_style.add_modifier(Modifier::SLOW_BLINK)),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(boot_lines).style(Style::default().bg(INDIGO_BG)),
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
        frame.render_widget(
            Paragraph::new(title_lines),
            rows[4],
        );

        // --- PROFILE.TXT placeholder ---
        frame.render_widget(
            Paragraph::new("[ PROFILE.TXT ]")
                .style(Style::default().fg(PANEL_TEXT).bg(INDIGO_BG)),
            rows[6],
        );

        // --- SKILLS / CAREER placeholder ---
        frame.render_widget(
            Paragraph::new("[ SKILLS / CAREER ]")
                .style(Style::default().fg(SECONDARY_BLUE).bg(INDIGO_BG)),
            rows[8],
        );

        // --- Footer hint row ---
        frame.render_widget(
            Paragraph::new("↑/↓  w/s  scroll  ·  Esc  back to menu  ·  q  quit")
                .style(Style::default().fg(SECONDARY_BLUE).bg(INDIGO_BG)),
            rows[9],
        );

        // Paint gap rows with indigo bg
        for &gap in &[rows[1], rows[3], rows[5], rows[7]] {
            frame.render_widget(
                Block::default().style(Style::default().bg(INDIGO_BG)),
                gap,
            );
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
}
