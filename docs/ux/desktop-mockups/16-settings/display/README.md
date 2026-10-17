# Handoff: Settings › Display

> **Sub deck of package 16 · Settings** — 6 frames: DESKTOP-129 (Medium), DESKTOP-130, DESKTOP-132 and DESKTOP-133 (Large: top, scrolled, and scrolled in Dark), DESKTOP-131 (X-Large), then DESKTOP-064 (16f, the v5 reference). Open `Display.dc.html` through `mise run mockups`. The Settings index deck, the navigation per Window Size and the anatomy every page shares are in `../README.md`; the Chrome at each Window Size is in `../../01-shell/README.md#window-sizes`.

## Overview

Display is the client-only Configuration page: how dates, numbers, rows and status glyphs look on this device, plus the Sidebar options. Nothing here syncs. The values are read from `personal-ledger.conf` at start-up and written back from this page. The code lives in `crates/bins/bin-desktop/src/settings/display/`, which names this folder.

Display has no Dialogs: every field saves on change.

## Frames

Rows run Medium, Large and X-Large, the order the Chrome is designed in ([ADR-0042](../../../../adr/0042-window-size-from-window-width-with-a-medium-floor.md)). The fields, PREVIEW and Sidebar group are 16f's; the Toasts field and the Colour Theme group below them are new to this sub deck. Everything is capped at 840px; only the Chrome, the heading, the space beside the PREVIEW and how far the page scrolls change.

| Card | Window Size | Frame | Chrome | Heading | Content |
| --- | --- | --- | --- | --- | --- |
| **DESKTOP-129** | Medium | 960 × 1040 | icon rail (52px), no docked index | breadcrumb **Settings › Display ▾** with the page picker caret | The PREVIEW takes the 500px beside the 300px field column; the Sidebar group sits below the fields, then the Toasts field and the Colour Theme group, four cards to a row. The page runs about 120px past the View, so it scrolls. |
| **DESKTOP-130** | Large | 1280 × 800 | expanded rail (206px), docked 232px Settings index with Display selected | **Display** alone | The PREVIEW flexes to the View's edge. The page is taller than the View: a vertical scroll bar shows and the Toasts field and Colour Theme group start below the fold. |
| **DESKTOP-132** | Large | 1280 × 800 | as DESKTOP-130 | **Display** alone, scrolled with the page | DESKTOP-130 scrolled to the end, so the Toasts field and the Colour Theme group show in full. |
| **DESKTOP-133** | Large | 1280 × 800 | as DESKTOP-130, in Modernist's Dark Colour Variant | **Display** alone, scrolled with the page | DESKTOP-132 with Appearance set to Dark and a `[theme.dark] positive = "#2a6340"` override: every card draws its Dark roles with that `positive`, and the override note and contrast line sit under the grid. |
| **DESKTOP-131** | X-Large | 1920 × 1080 | the same Chrome as Large | **Display** alone | The PREVIEW stops with the page at 840px and the rest of the View stays empty. Even at 1080px the page runs about 80px past the View, so a short scroll bar shows. Settings docks no detail pane. |
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
- **Toasts** (`margin-top:24px`, 300px, below the Sidebar group; `toasts_field` in `render.rs`, [ADR-0027](../../../../adr/0027-toasts-off-is-a-client-scoped-preference-and-errors-always-toast.md)): an On / Off segmented control with 88px options, On in the sample, over the 12px #605d5d note "Errors still show". Off sends Toasts to the status line instead; Errors still toast.
- **Colour Theme group** (full width, `margin-top:28px`, below the Toasts field), per `docs/colour-themes-design.md` "Settings":
  - **Appearance:** a Light / Dark / System segmented control, the same as the other segmented fields. System is chosen in the sample, with the 12px #605d5d note "System (currently Light)" under it; if the OS reports nothing it reads "System (not detected, using Dark)".
  - **Colour theme:** a wrapping grid (`gap:12px`) of 160px cards, one per built-in Colour Theme in order: Modernist, High Contrast, Catppuccin, Gruvbox, Nord. Each card is a miniature ledger 84px tall in that Colour Theme's Colour Variant in effect (Light in most frames, Dark in DESKTOP-133), run through `resolve` with this Client's `[theme]` overrides: an 18px chrome band with a muted bar and an accent chip, then three 14px rows, the middle one selected (the `foreground`/`background` swap), carrying a positive, a negative and a muted amount. The name sits 6px under it in 12px.
  - **Chosen card:** a 2px border in the current Colour Theme's `accent` and a bold name; the rest have a 1px border. Keyboard focus draws a 1px `foreground` ring 2px outside the card, so focus and choice never hide each other.
  - **Override and contrast notes** (DESKTOP-133): while `[theme]` overrides any role in the Colour Variant drawn, a 12px `muted` note 6px under the grid reads "positive overridden by [theme] in your configuration file. Restart to change them."; each contrast failure adds a 12px line 4px below in the negative text shade, "positive on background is 2.6:1, needs 3:1". The override is still used.
- **Footnote:** these values are read from `personal-ledger.conf` at start-up and written back here; ledger-scoped Preferences are under **General**. DESKTOP-064 still says "Ledger & units".

## Keyboard

| Key | Action |
| --- | --- |
| `j` / `k` | Next / previous field. |
| `tab` | Next field. |
| `tab` onto the grid, then arrows or `h`/`j`/`k`/`l` | Move focus between Colour Theme cards; `enter` chooses. Focus and hover never preview. |
| `[` / `]` | Medium only: previous / next page, General and Units. |
| `p` | Medium only: open the page picker. |
| `esc` | Medium only: back to the Settings index. |

## Known gaps

Toasts, the Dark Colour Variant and the `[theme]` notes are now drawn. Two differences from the code in `settings/display/render.rs` remain, both inherited from 16f:

- **Locale isn't drawn.** The code opens the field column with a read-only Locale field (`locale_field`: the effective Locale, where it came from, and a hint that it changes through the `locale` setting or `--locale` and a restart, per [ADR-0021](../../../../adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md)). No frame shows it.
- **Decimal & thousands separator isn't in the code.** The frames draw it from 16f, but the Desktop has no such field: number formatting follows the Locale (ADR-0021).

## Files

- `Display.dc.html` — the Display frames in Medium, Large (top, scrolled, and scrolled in Dark) and X-Large rows, then the v5 reference. It loads `../../support.js` and `../../styles.css`; links to the Chrome cards open `../../01-shell/Shell.dc.html`.
- `README.md` — this document.
