use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::block_font;

const TITLE_FG: Color = Color::Rgb(0, 255, 0);
const TITLE_SHADOW: Color = Color::Rgb(0, 80, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Play,
    About,
    Quit,
}

pub(crate) const ITEMS: &[MenuItem] = &[MenuItem::Play, MenuItem::About, MenuItem::Quit];

impl MenuItem {
    fn label(self) -> &'static str {
        match self {
            MenuItem::Play => "Play",
            MenuItem::About => "About",
            MenuItem::Quit => "Quit",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MenuCommand {
    Up,
    Down,
    Select,
    Quit,
    Ignore,
}

pub fn map_menu_key(code: KeyCode) -> MenuCommand {
    match code {
        KeyCode::Up | KeyCode::Char('w') => MenuCommand::Up,
        KeyCode::Down | KeyCode::Char('s') => MenuCommand::Down,
        KeyCode::Enter => MenuCommand::Select,
        KeyCode::Esc | KeyCode::Char('q') => MenuCommand::Quit,
        _ => MenuCommand::Ignore,
    }
}

pub struct Menu {
    pub selected: usize,
}

impl Menu {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn up(&mut self) {
        if self.selected == 0 {
            self.selected = ITEMS.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    pub fn down(&mut self) {
        self.selected = (self.selected + 1) % ITEMS.len();
    }

    pub fn selected_item(&self) -> MenuItem {
        ITEMS[self.selected]
    }
}

pub fn activate(item: MenuItem) -> crate::Nav {
    match item {
        MenuItem::Play => crate::Nav::To(crate::Screen::Game),
        MenuItem::About => crate::Nav::To(crate::Screen::About),
        MenuItem::Quit => crate::Nav::Quit,
    }
}

fn render_menu(frame: &mut Frame, menu: &Menu) {
    let area = frame.area();

    // Banner: HEIGHT rows + 1 shadow row + 1 padding below
    let banner_h = (block_font::HEIGHT + 2) as u16;
    let item_h = ITEMS.len() as u16;
    let footer_h = 1u16;
    let total_content = banner_h + item_h + 1 + footer_h; // +1 gap between items/footer

    let top_pad = area.height.saturating_sub(total_content) / 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_pad),
            Constraint::Length(banner_h),
            Constraint::Length(item_h),
            Constraint::Length(1),
            Constraint::Length(footer_h),
            Constraint::Min(0),
        ])
        .split(area);

    let banner_area = chunks[1];
    let items_area = chunks[2];
    let footer_area = chunks[4];

    // Title banner — horizontally centered
    let lines = block_font::compose("CHARLO", 1);
    let banner_w = lines[0].chars().count() as u16;
    let banner_x = banner_area.x + banner_area.width.saturating_sub(banner_w) / 2;
    block_font::draw_banner(
        frame.buffer_mut(),
        (banner_x, banner_area.y),
        &lines,
        TITLE_FG,
        TITLE_SHADOW,
    );

    // Item list
    for (i, item) in ITEMS.iter().enumerate() {
        let row = Rect::new(items_area.x, items_area.y + i as u16, items_area.width, 1);
        let style = if i == menu.selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(0, 255, 0))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(0, 200, 0))
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {}  ", item.label()),
                style,
            )))
            .alignment(Alignment::Center)
            .block(Block::default()),
            row,
        );
    }

    // Footer hint
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "↑/↓  w/s  select · Enter · q quit",
            Style::default().fg(Color::DarkGray),
        )))
        .alignment(Alignment::Center)
        .block(Block::default()),
        footer_area,
    );
}

pub fn menu(terminal: &mut DefaultTerminal) -> std::io::Result<crate::Nav> {
    let mut state = Menu::new();
    loop {
        terminal.draw(|frame| render_menu(frame, &state))?;

        if event::poll(std::time::Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match map_menu_key(key.code) {
                MenuCommand::Up => state.up(),
                MenuCommand::Down => state.down(),
                MenuCommand::Select => return Ok(activate(state.selected_item())),
                MenuCommand::Quit => return Ok(crate::Nav::Quit),
                MenuCommand::Ignore => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- map_menu_key ---

    #[test]
    fn arrow_up_maps_to_up() {
        assert_eq!(map_menu_key(KeyCode::Up), MenuCommand::Up);
    }

    #[test]
    fn w_maps_to_up() {
        assert_eq!(map_menu_key(KeyCode::Char('w')), MenuCommand::Up);
    }

    #[test]
    fn arrow_down_maps_to_down() {
        assert_eq!(map_menu_key(KeyCode::Down), MenuCommand::Down);
    }

    #[test]
    fn s_maps_to_down() {
        assert_eq!(map_menu_key(KeyCode::Char('s')), MenuCommand::Down);
    }

    #[test]
    fn enter_maps_to_select() {
        assert_eq!(map_menu_key(KeyCode::Enter), MenuCommand::Select);
    }

    #[test]
    fn esc_maps_to_quit() {
        assert_eq!(map_menu_key(KeyCode::Esc), MenuCommand::Quit);
    }

    #[test]
    fn q_maps_to_quit() {
        assert_eq!(map_menu_key(KeyCode::Char('q')), MenuCommand::Quit);
    }

    #[test]
    fn unknown_key_maps_to_ignore() {
        assert_eq!(map_menu_key(KeyCode::Char('z')), MenuCommand::Ignore);
        assert_eq!(map_menu_key(KeyCode::Tab), MenuCommand::Ignore);
    }

    // --- Menu::up() / down() wraparound ---

    #[test]
    fn down_advances_selection() {
        let mut m = Menu::new();
        assert_eq!(m.selected, 0);
        m.down();
        assert_eq!(m.selected, 1);
        m.down();
        assert_eq!(m.selected, 2);
    }

    #[test]
    fn down_wraps_from_last_to_first() {
        let mut m = Menu::new();
        for _ in 0..(ITEMS.len() - 1) {
            m.down();
        }
        assert_eq!(m.selected, ITEMS.len() - 1);
        m.down();
        assert_eq!(m.selected, 0);
    }

    #[test]
    fn up_wraps_from_first_to_last() {
        let mut m = Menu::new();
        assert_eq!(m.selected, 0);
        m.up();
        assert_eq!(m.selected, ITEMS.len() - 1);
    }

    #[test]
    fn up_decrements_selection() {
        let mut m = Menu::new();
        m.down();
        m.down(); // at index 2
        m.up();
        assert_eq!(m.selected, 1);
    }

    // --- activate: item → Nav ---

    #[test]
    fn activate_play_goes_to_game() {
        assert_eq!(activate(MenuItem::Play), crate::Nav::To(crate::Screen::Game));
    }

    #[test]
    fn activate_about_goes_to_about() {
        assert_eq!(activate(MenuItem::About), crate::Nav::To(crate::Screen::About));
    }

    #[test]
    fn activate_quit_returns_quit() {
        assert_eq!(activate(MenuItem::Quit), crate::Nav::Quit);
    }
}
