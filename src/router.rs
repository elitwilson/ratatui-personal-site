//! Target-agnostic screen state machine.
//!
//! The native terminal loop and the web (ratzilla) callbacks both drive a single
//! `Router`: feed it keys (`handle_key`), advance animation (`tick`), draw the
//! active screen (`render`). All screen-specific state and transitions live here
//! and in the screen structs — the runners only differ in how they source events
//! and time, and in how they interpret `Nav::Quit`.

use crate::about::About;
use crate::app::GameScreen;
use crate::input::KeyCode;
use crate::menu::Menu;
use crate::sandbox::SandboxScreen;
use crate::{Nav, Screen};
use ratatui::Frame;
use ratatui::layout::Rect;
use std::time::Duration;

enum ScreenState {
    Menu(Menu),
    Game(GameScreen),
    About(About),
    Sandbox(SandboxScreen),
}

pub struct Router {
    screen: ScreenState,
}

impl Router {
    pub fn new() -> Self {
        Self {
            screen: ScreenState::Menu(Menu::new()),
        }
    }

    /// Dispatch a keypress to the active screen. Returns `Some(Nav)` if the press
    /// requests a navigation; the runner decides what `Nav::Quit` means.
    pub fn handle_key(&mut self, code: KeyCode) -> Option<Nav> {
        match &mut self.screen {
            ScreenState::Menu(s) => s.handle_key(code),
            ScreenState::Game(s) => s.handle_key(code),
            ScreenState::About(s) => s.handle_key(code),
            ScreenState::Sandbox(s) => s.handle_key(code),
        }
    }

    /// Advance time-based animation on screens that have any.
    pub fn tick(&mut self, dt: Duration) {
        match &mut self.screen {
            ScreenState::Game(s) => s.tick(dt),
            ScreenState::Sandbox(s) => s.tick(dt),
            ScreenState::About(s) => s.tick(dt),
            ScreenState::Menu(_) => {}
        }
    }

    /// Draw the active screen; returns its body `Rect`.
    pub fn render(&mut self, frame: &mut Frame) -> Rect {
        match &mut self.screen {
            ScreenState::Menu(s) => s.render(frame),
            ScreenState::Game(s) => s.render(frame),
            ScreenState::About(s) => s.render(frame),
            ScreenState::Sandbox(s) => s.render(frame),
        }
    }

    /// Feed the latest cursor cell to the active screen. The sandbox uses it as a
    /// cursor attractor; About uses it for the bio link hover highlight.
    pub fn set_mouse(&mut self, pos: (u16, u16)) {
        match &mut self.screen {
            ScreenState::Sandbox(s) => s.set_mouse(pos),
            ScreenState::About(s) => s.set_mouse(pos),
            _ => {}
        }
    }

    /// Dispatch a left-click at cell `pos` to the active screen. Returns
    /// `Some(Nav)` if the click requests a navigation (e.g. an About bio link).
    pub fn handle_click(&mut self, pos: (u16, u16)) -> Option<Nav> {
        match &mut self.screen {
            ScreenState::About(s) => s.handle_click(pos),
            _ => None,
        }
    }

    /// Swap in a fresh state for the target screen. Each entry starts clean
    /// (matching the old per-screen loops, which built fresh state on entry).
    pub fn goto(&mut self, screen: Screen) {
        self.screen = match screen {
            Screen::Menu => ScreenState::Menu(Menu::new()),
            Screen::Game => ScreenState::Game(GameScreen::new()),
            Screen::About => ScreenState::About(About::new()),
            Screen::Sandbox => ScreenState::Sandbox(SandboxScreen::new()),
        };
    }
}
