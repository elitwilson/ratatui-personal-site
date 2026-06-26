//! Web (WASM) runner.
//!
//! Drives the same [`Router`](crate::router::Router) as the native loop, but
//! through ratzilla's callback model: input arrives via `on_key_event` /
//! `on_mouse_event`, and `draw_web` invokes the render closure every animation
//! frame (driven by `requestAnimationFrame`). The frame hook supplies no delta
//! time, so we derive it from `performance.now()`.
//!
//! There's no process to exit on the web, so `Nav::Quit` is remapped to the menu.

use crate::router::Router;
use crate::{Nav, Screen};
use ratzilla::ratatui::Terminal;
use ratzilla::{DomBackend, WebRenderer};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub fn run() -> std::io::Result<()> {
    // Route Rust panics to the browser console with a readable stack trace.
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // `DomBackend` sizes the terminal grid from the *measured* width of a
    // character cell. If we build it before the Fira Code web font has loaded,
    // that measurement uses the fallback `monospace` font; once Fira Code swaps
    // in, the now-wrong grid overflows the viewport (the cold-load horizontal
    // scrollbar). Awaiting `document.fonts.ready` first guarantees the real font
    // is in place before the first measurement. On a warm cache it resolves
    // immediately, so refreshes are unaffected.
    wasm_bindgen_futures::spawn_local(async {
        wait_for_fonts().await;
        start();
    });

    Ok(())
}

/// Resolve once the document's fonts have finished loading. Any missing window /
/// document (e.g. a non-browser host) is treated as "nothing to wait for".
async fn wait_for_fonts() {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let Ok(ready) = document.fonts().ready() {
        let _ = wasm_bindgen_futures::JsFuture::from(ready).await;
    }
}

fn start() {
    let backend = DomBackend::new().expect("failed to create DOM backend");
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    // Shared across the key, mouse, and draw callbacks. Single-threaded in the
    // browser, and the callbacks never run reentrantly, so the borrows never
    // overlap.
    let router = Rc::new(RefCell::new(Router::new()));

    terminal
        .on_key_event({
            let router = router.clone();
            move |key_event| {
                let mut router = router.borrow_mut();
                if let Some(nav) = router.handle_key(key_event.code) {
                    match nav {
                        // No process to quit on the web — fall back to the menu.
                        Nav::Quit => router.goto(Screen::Menu),
                        Nav::To(screen) => router.goto(screen),
                        // Open the link in a new tab; stay on the current screen.
                        Nav::OpenUrl(url) => open_in_new_tab(url),
                    }
                }
            }
        })
        .expect("failed to register key handler");

    terminal
        .on_mouse_event({
            let router = router.clone();
            move |mouse_event| {
                use ratzilla::event::{MouseButton, MouseEventKind};
                let pos = (mouse_event.col, mouse_event.row);
                let mut router = router.borrow_mut();
                router.set_mouse(pos);
                if matches!(mouse_event.kind, MouseEventKind::SingleClick(MouseButton::Left)) {
                    if let Some(nav) = router.handle_click(pos) {
                        match nav {
                            Nav::Quit => router.goto(Screen::Menu),
                            Nav::To(screen) => router.goto(screen),
                            Nav::OpenUrl(url) => open_in_new_tab(url),
                        }
                    }
                }
            }
        })
        .expect("failed to register mouse handler");

    let mut last = now_ms();
    // ratzilla attaches its `keydown` listener to the grid element and makes it
    // focusable with `tabindex="0"`, but only appends that element to the DOM
    // during the first render frame. On a fresh load nothing focuses it, so the
    // menu stays unresponsive until the user clicks. We can't focus it before the
    // loop starts (it isn't in the DOM yet), so retry each frame until focus takes
    // — it lands on the first frame where the grid exists, then stops.
    let mut focused = false;
    terminal.draw_web(move |frame| {
        let now = now_ms();
        // Clamp against clock anomalies so a bad sample can't rewind animation.
        let dt = Duration::from_secs_f64((now - last).max(0.0) / 1000.0);
        last = now;

        let mut router = router.borrow_mut();
        router.render(frame);
        router.tick(dt);

        if !focused {
            focused = focus_grid();
        }
    });
}

/// Give keyboard focus to ratzilla's grid element (default id `grid`) so key
/// events flow without an initial click. Returns `true` once the element exists
/// and focus was issued; `false` if the grid isn't in the DOM yet (so the caller
/// can retry on a later frame). A non-`HtmlElement` match is treated as success
/// to avoid spinning forever.
fn focus_grid() -> bool {
    use web_sys::wasm_bindgen::JsCast;
    let Some(element) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("grid"))
    else {
        return false;
    };
    if let Ok(html) = element.dyn_into::<web_sys::HtmlElement>() {
        let _ = html.focus();
    }
    true
}

/// Open a URL in a new browser tab. A blocked popup or missing window is
/// non-fatal — there's nothing useful to do but ignore it.
fn open_in_new_tab(url: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target(url, "_blank");
    }
}

/// Milliseconds from a monotonic high-resolution clock.
fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}
