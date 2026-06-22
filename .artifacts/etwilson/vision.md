# Vision: Ratatui Personal Site

> A personal website that looks like a terminal and plays like a tiny game — explore a little, and the content reveals itself.

---

## Problem Statement

A plain personal site is forgettable. This project exists to make one that isn't: a portfolio you *play* for a few seconds rather than scroll. It's built in the open as a vehicle for learning Ratatui and Ratzilla (Rust TUI rendered to the browser via WASM), so the work doubles as craft practice and as a real, shareable artifact. The game is the point — but the content underneath it (about, etc.) is the reason the site exists, so reaching it must never *require* finishing the game.

---

## Product Positioning

A browser-delivered personal site, rendered as a terminal UI, where a visitor steers a character through a tiny castle to reach an "about" payoff. It's for anyone who lands on the URL — and, eventually, for the kind of people who look at a personal site (developers, the curious). The differentiator is the medium itself: it looks like a terminal and behaves like a video game, which is far more memorable than a conventional page. Because some visitors won't want to play, the same content is also reachable directly, without the game. The site is currently exploratory and not actively promoted; "published" here means deployed and reachable, not marketed.

---

## Users & Personas

The audience is intentionally undecided — the project is exploratory and the author is not yet committed to actively promoting it. Two loosely-held personas inform design:

- **The Player** — lands on the site, is willing to mess around for a few seconds. Wants the game to be obvious and instantly playable (WASD/arrows), not a puzzle to figure out. Assumed *not* especially terminal-savvy: it should look like a terminal but behave like a game.
- **The Skimmer** — wants the content, not the game. Should be able to reach the about content directly without playing through.

---

## Core Features

- Terminal-styled UI rendered in the browser (Ratzilla/WASM).
- A small explorable tile map (castle) with walls, floor, and outside void.
- Keyboard-driven character movement (WASD and arrow keys).
- A trivially simple game loop: find the key, reach the door — roughly a 3-second experience.
- An "about" content view, revealed as the payoff of the game loop.
- A direct, no-game path to the same content (you don't *need* to play to read it).
- A swappable color theme so the whole look can change in one place.

---

## Explicit Non-Goals

- **Not** a complex or challenging game — no difficulty, no real puzzles, no time pressure. The game is a 3-second hook, not the product.
- **No** sound in v1.
- **No** multiple levels / multiple maps in v1.
- **No** multiplayer, save state, accounts, or analytics.
- **No** backend or server-side logic — it's a static WASM site.
- **No** active marketing or promotion as part of building it; deployment ≠ a launch campaign.
- Content sections beyond "game" and "about" are **not** specified or committed yet.

---

## Product Constraints

- Delivered as a **WASM website** via Ratzilla, running in a desktop browser.
- **Keyboard-driven**; playing the game must not require a mouse.
- Must **look like a terminal** while behaving like a game.
- Content must remain **accessible without completing (or playing) the game**.
- Static/client-side only — no server dependency.

---

## Tech Stack

- **Rust** (edition 2024).
- **Ratatui** (0.30) for the TUI layer.
- **Ratzilla** for rendering the Ratatui app to the browser via **WASM** (the intended web delivery; not yet implemented).
- **crossterm** for native terminal runs during development.
- **color-eyre** for error reporting.

---

## Milestones & Phasing

### v1 — On the Web, Playable
- Port the existing native Ratatui app to run in the browser via Ratzilla/WASM.
- Deploy to a public URL.
- Ship the full game loop (move → key → door → about reveal).
- Provide a direct path to the about content that doesn't require playing the game.

### v2 — Later (uncommitted)
- Possible sound.
- Possible multiple levels / maps.
- Audience and promotion decisions, if the project graduates from "exploratory."

---

## Success Criteria

- **v1:** The app is deployed to a public URL and loads/runs in a desktop browser via Ratzilla/WASM. A visitor can move the character with the keyboard, complete the key→door loop, and read the About content. The same About content is also reachable **without** completing or playing the game.
- **v2:** Deferred — to be defined if and when the project moves past exploration.
