use ratatui::buffer::Buffer;
use ratatui::style::Color;

pub const HEIGHT: usize = 6;

const GLYPH_WIDTH: usize = 7;

fn glyph(c: char) -> [&'static str; HEIGHT] {
    match c {
        'A' => [
            " █████ ",
            "██   ██",
            "███████",
            "██   ██",
            "██   ██",
            "       ",
        ],
        'B' => [
            "██████ ",
            "██   ██",
            "██████ ",
            "██   ██",
            "██████ ",
            "       ",
        ],
        'C' => [
            " █████ ",
            "██     ",
            "██     ",
            "██     ",
            " █████ ",
            "       ",
        ],
        'D' => [
            "██████ ",
            "██   ██",
            "██   ██",
            "██   ██",
            "██████ ",
            "       ",
        ],
        'E' => [
            "███████",
            "██     ",
            "█████  ",
            "██     ",
            "███████",
            "       ",
        ],
        'F' => [
            "███████",
            "██     ",
            "█████  ",
            "██     ",
            "██     ",
            "       ",
        ],
        'G' => [
            " █████ ",
            "██     ",
            "██  ███",
            "██   ██",
            " ██████",
            "       ",
        ],
        'H' => [
            "██   ██",
            "██   ██",
            "███████",
            "██   ██",
            "██   ██",
            "       ",
        ],
        'I' => [
            "███████",
            "  ███  ",
            "  ███  ",
            "  ███  ",
            "███████",
            "       ",
        ],
        'J' => [
            "███████",
            "   ███ ",
            "   ███ ",
            "██ ███ ",
            " █████ ",
            "       ",
        ],
        'K' => [
            "██   ██",
            "██  ██ ",
            "█████  ",
            "██  ██ ",
            "██   ██",
            "       ",
        ],
        'L' => [
            "██     ",
            "██     ",
            "██     ",
            "██     ",
            "███████",
            "       ",
        ],
        'M' => [
            "██   ██",
            "███ ███",
            "███████",
            "██ █ ██",
            "██   ██",
            "       ",
        ],
        'N' => [
            "██   ██",
            "███  ██",
            "██ █ ██",
            "██  ███",
            "██   ██",
            "       ",
        ],
        'O' => [
            " █████ ",
            "██   ██",
            "██   ██",
            "██   ██",
            " █████ ",
            "       ",
        ],
        'P' => [
            "██████ ",
            "██   ██",
            "██████ ",
            "██     ",
            "██     ",
            "       ",
        ],
        'Q' => [
            " █████ ",
            "██   ██",
            "██   ██",
            "██  ███",
            " ██████",
            "       ",
        ],
        'R' => [
            "██████ ",
            "██   ██",
            "██████ ",
            "██  ██ ",
            "██   ██",
            "       ",
        ],
        'S' => [
            " █████ ",
            "██     ",
            " █████ ",
            "     ██",
            " █████ ",
            "       ",
        ],
        'T' => [
            "███████",
            "  ███  ",
            "  ███  ",
            "  ███  ",
            "  ███  ",
            "       ",
        ],
        'U' => [
            "██   ██",
            "██   ██",
            "██   ██",
            "██   ██",
            " █████ ",
            "       ",
        ],
        'V' => [
            "██   ██",
            "██   ██",
            "██   ██",
            " ██ ██ ",
            "  ███  ",
            "       ",
        ],
        'W' => [
            "██   ██",
            "██   ██",
            "██ █ ██",
            "███ ███",
            "██   ██",
            "       ",
        ],
        'X' => [
            "██   ██",
            " ██ ██ ",
            "  ███  ",
            " ██ ██ ",
            "██   ██",
            "       ",
        ],
        'Y' => [
            "██   ██",
            " ██ ██ ",
            "  ███  ",
            "  ███  ",
            "  ███  ",
            "       ",
        ],
        'Z' => [
            "███████",
            "    ██ ",
            "  ███  ",
            " ██    ",
            "███████",
            "       ",
        ],
        _ => [
            "       ",
            "       ",
            "       ",
            "       ",
            "       ",
            "       ",
        ],
    }
}

pub fn compose(text: &str, gap: usize) -> Vec<String> {
    let mut rows: Vec<String> = vec![String::new(); HEIGHT];
    let sep = " ".repeat(gap);
    for (i, c) in text.chars().enumerate() {
        let g = glyph(c);
        for (row_idx, row_str) in g.iter().enumerate() {
            if i > 0 {
                rows[row_idx].push_str(&sep);
            }
            rows[row_idx].push_str(row_str);
        }
    }
    // Pad all rows to equal width (guards against any future variable-width glyphs)
    let max_w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    for row in &mut rows {
        let w = row.chars().count();
        if w < max_w {
            for _ in 0..(max_w - w) {
                row.push(' ');
            }
        }
    }
    rows
}

pub fn draw_banner(
    buf: &mut Buffer,
    top_left: (u16, u16),
    lines: &[String],
    fg: Color,
    shadow: Color,
) {
    let (orig_x, orig_y) = top_left;

    // Pass 1: shadow layer — offset (+1, +1)
    for (row_idx, line) in lines.iter().enumerate() {
        let sy = orig_y as i32 + row_idx as i32 + 1;
        if sy < 0 {
            continue;
        }
        let sy = sy as u16;
        for (col_idx, ch) in line.chars().enumerate() {
            if ch != ' ' {
                let sx = orig_x as i32 + col_idx as i32 + 1;
                if sx < 0 {
                    continue;
                }
                let sx = sx as u16;
                if let Some(cell) = buf.cell_mut((sx, sy)) {
                    cell.set_symbol("█");
                    cell.set_fg(shadow);
                }
            }
        }
    }

    // Pass 2: main layer — at top_left (overdrawing any shadow overlap)
    for (row_idx, line) in lines.iter().enumerate() {
        let my = orig_y as i32 + row_idx as i32;
        if my < 0 {
            continue;
        }
        let my = my as u16;
        for (col_idx, ch) in line.chars().enumerate() {
            if ch != ' ' {
                let mx = orig_x as i32 + col_idx as i32;
                if mx < 0 {
                    continue;
                }
                let mx = mx as u16;
                if let Some(cell) = buf.cell_mut((mx, my)) {
                    cell.set_symbol("█");
                    cell.set_fg(fg);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    // --- compose: line count and equal width ---

    #[test]
    fn compose_returns_exactly_height_lines() {
        let lines = compose("CHARLO", 1);
        assert_eq!(lines.len(), HEIGHT);
    }

    #[test]
    fn compose_all_lines_equal_width() {
        let lines = compose("CHARLO", 1);
        let widths: Vec<usize> = lines.iter().map(|l| l.chars().count()).collect();
        let first = widths[0];
        for w in &widths {
            assert_eq!(*w, first, "lines differ in width: {:?}", widths);
        }
    }

    #[test]
    fn compose_single_char_returns_height_lines() {
        let lines = compose("C", 1);
        assert_eq!(lines.len(), HEIGHT);
    }

    // --- compose: unknown-char fallback is blank (no panic) ---

    #[test]
    fn compose_unknown_char_does_not_panic() {
        // '!' has no defined glyph — should degrade gracefully as blank width
        let lines = compose("!", 1);
        assert_eq!(lines.len(), HEIGHT);
    }

    #[test]
    fn compose_unknown_char_width_matches_known_char_width() {
        // Unknown characters should be blank but have the same fixed width as known chars
        let known = compose("C", 1);
        let unknown = compose("!", 1);
        let known_w = known[0].chars().count();
        let unknown_w = unknown[0].chars().count();
        assert_eq!(known_w, unknown_w, "blank fallback should have same width as a real glyph");
    }

    #[test]
    fn compose_unknown_char_rows_contain_only_spaces() {
        // Unknown char fallback should produce all-space rows
        let lines = compose("!", 1);
        for (i, line) in lines.iter().enumerate() {
            assert!(
                line.chars().all(|c| c == ' '),
                "row {i} of unknown-char fallback should be all spaces: {line:?}"
            );
        }
    }

    // --- draw_banner: shadow and main cell placement + colors ---

    #[test]
    fn draw_banner_paints_main_cells_in_fg_color() {
        // Use a single-char compose to get predictable glyph data
        let lines = compose("C", 1);
        // Buffer big enough to hold the banner + shadow offset
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        let fg = Color::Rgb(0, 255, 0);
        let shadow = Color::Rgb(0, 80, 0);
        draw_banner(&mut buf, (0, 0), &lines, fg, shadow);

        // Find at least one cell that is a filled block glyph and has the fg color
        let mut found_fg = false;
        for row in 0..(HEIGHT as u16) {
            for col in 0..20u16 {
                if let Some(cell) = buf.cell((col, row)) {
                    if cell.symbol() == "█" && cell.fg == fg {
                        found_fg = true;
                    }
                }
            }
        }
        assert!(found_fg, "expected at least one cell with fg color");
    }

    #[test]
    fn draw_banner_paints_shadow_cells_at_offset_plus_one() {
        // With top_left=(0,0), shadow cells should appear at (+1,+1) offsets
        let lines = compose("C", 1);
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        let fg = Color::Rgb(0, 255, 0);
        let shadow = Color::Rgb(0, 80, 0);
        draw_banner(&mut buf, (0, 0), &lines, fg, shadow);

        // Check that at least one shadow-colored filled block exists at row>=1, col>=1
        let mut found_shadow = false;
        for row in 1..(HEIGHT as u16 + 1) {
            for col in 1..20u16 {
                if let Some(cell) = buf.cell((col, row)) {
                    if cell.symbol() == "█" && cell.fg == shadow {
                        found_shadow = true;
                    }
                }
            }
        }
        assert!(found_shadow, "expected at least one shadow cell at (+1,+1) offset");
    }

    #[test]
    fn draw_banner_main_overdrawes_shadow_at_overlap() {
        // Where a main glyph cell and a shadow cell land on the same buffer cell,
        // the main layer must win (fg color, not shadow color).
        // We test this by checking every position where the main glyph has a filled
        // char — those cells must always carry fg, never shadow.
        let lines = compose("C", 1);
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        let fg = Color::Rgb(0, 255, 0);
        let shadow = Color::Rgb(0, 80, 0);
        draw_banner(&mut buf, (0, 0), &lines, fg, shadow);

        // Check only positions where the compose output has a filled char (non-space).
        // Those are the main-layer cells and must carry fg.
        for (row_idx, line) in lines.iter().enumerate() {
            for (col_idx, ch) in line.chars().enumerate() {
                if ch != ' ' {
                    let col = col_idx as u16;
                    let row = row_idx as u16;
                    if let Some(cell) = buf.cell((col, row)) {
                        assert_eq!(
                            cell.fg, fg,
                            "main glyph cell at ({col},{row}) should have fg color, not shadow"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn draw_banner_bounds_checked_no_panic_on_small_buffer() {
        // A buffer smaller than the banner should not panic — cells are clipped
        let lines = compose("CHARLO", 1);
        let area = Rect::new(0, 0, 5, 5); // intentionally tiny
        let mut buf = Buffer::empty(area);
        let fg = Color::Rgb(0, 255, 0);
        let shadow = Color::Rgb(0, 80, 0);
        // Should not panic even though banner is ~40 cols wide
        draw_banner(&mut buf, (0, 0), &lines, fg, shadow);
    }
}
