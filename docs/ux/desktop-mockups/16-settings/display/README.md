# Handoff: Settings › Display

> **Sub deck of package 16 · Settings** — 4 frames: DESKTOP-129 (Medium), DESKTOP-130 (Large), DESKTOP-131 (X-Large), then DESKTOP-064 (16f, the v5 reference). Open `Display.dc.html` through `mise run mockups`. The Settings index deck, the navigation per Window Size and the anatomy every page shares are in `../README.md`; the Chrome at each Window Size is in `../../01-shell/README.md#window-sizes`.

## Overview

Display is the client-only Configuration page: how dates, numbers, rows and status glyphs look on this device, plus the Sidebar options. Nothing here syncs. The values are read from `personal-ledger.conf` at start-up and written back from this page. The code lives in `crates/bins/bin-desktop/src/settings/display/`, which names this folder.

Display has no Dialogs: every field saves on change.

## Frames

Rows run Medium, Large and X-Large, the order the Chrome is designed in ([ADR-0042](../../../../adr/0042-window-size-from-window-width-with-a-medium-floor.md)). The fields, PREVIEW and Sidebar group are 16f's, capped at 840px; only the Chrome, the heading and the space beside the PREVIEW change.

| Card | Window Size | Frame | Chrome | Heading | Content |
| --- | --- | --- | --- | --- | --- |
| **DESKTOP-129** | Medium | 960 × 1040 | icon rail (52px), no docked index | breadcrumb **Settings › Display ▾** with the page picker caret | The PREVIEW takes the 500px beside the 300px field column; the Sidebar group sits below the fields. |
| **DESKTOP-130** | Large | 1280 × 800 | expanded rail (206px), docked 232px Settings index with Display selected | **Display** alone | The PREVIEW flexes to the View's edge. |
| **DESKTOP-131** | X-Large | 1920 × 1080 | the same Chrome as Large | **Display** alone | The PREVIEW stops with the page at 840px and the rest of the View stays empty. Settings docks no detail pane. |
| DESKTOP-064 | — (v5) | 1280 × 800 | the v5 Chrome: expanded rail, 214px index, header breadcrumb | **Settings – Display** | Kept as the reference for the field values below. Where it differs from DESKTOP-130, DESKTOP-130 is correct. |

- **Status line at Medium:** `j/k section · [/] pages · p picker · esc settings`, with the ledger path shortened to `…/teda.pldb`. At Large and X-Large the form-page hints from `../README.md` apply.
- **Stacking:** at 960 the two columns sit side by side. Below about 700px of page width, as at the 720px floor with the icon rail, the PREVIEW wraps under the field column instead of crushing the payee.

## Content

- **Heading note:** `configuration · local · never synced`.
- **Segmented controls:** Date format (12 sep 2026 / 12/09/2026 / ISO), Decimal & thousands separator (1,234.56 / 1.234,56 / 1 234.56) and Row density (compact / regular / roomy).
- **Radios:** Status glyphs (unicode — ○ ◐ ● ⚑ / ascii fallback — o / x !).
- **PREVIEW (DATE · PAYEE · AMOUNT):** ● 12 sep 2026 Woolworths Metro −86.40 · ◐ 10 sep 2026 Telstra −99.00 · ⚑ 09 sep 2026 Dept of Education +4,210.00. It redraws as the fields change.
- **Sidebar group** (`margin-top:18px`, 300px max), two checkboxes (14×14, square, `accent-color:#201e1d`; bold 12.5px label over an 11.5px #605d5d hint):
  - **Hide sidebar** (off): "Start with the primary rail hidden. **b** or the header toggle still shows it for the session." It applies at Large and X-Large only; Medium always starts with the icon rail.
  - **Show keyboard navigation hints** (on): "Show each destination's **g**-jump, e.g. **g d**, at the right of its row. The jumps work either way."
- **Footnote:** these values are read from `personal-ledger.conf` at start-up and written back here; ledger-scoped Preferences are under **General**. DESKTOP-064 still says "Ledger & units".

## Keyboard

| Key | Action |
| --- | --- |
| `j` / `k` | Next / previous field. |
| `tab` | Next field. |
| `[` / `]` | Medium only: previous / next page, General and Units. |
| `p` | Medium only: open the page picker. |
| `esc` | Medium only: back to the Settings index. |

## Known gaps

- **Colour Theme isn't drawn.** The Desktop's Display page also has a full-width Colour Theme group (a Light / Dark / System segmented control over a grid of preview cards, specified in `docs/colour-themes-design.md` "Settings" and [ADR-0025](../../../../adr/0025-lib-colour-theme-resolves-shared-colours-from-build-time-ini-files.md)); none of these frames show it. Follow that spec until a frame is drawn.

## Files

- `Display.dc.html` — the Display frames in Medium, Large and X-Large rows, then the v5 reference. It loads `../../support.js` and `../../styles.css`; links to the Chrome cards open `../../01-shell/Shell.dc.html`.
- `README.md` — this document.
