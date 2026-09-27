# Colour Themes design

The developer-facing design for Colour Themes in the Desktop and TUI Clients. This page records the decisions reached on the [Colour Themes for the Desktop and TUI](https://github.com/IanTeda/Personal-Ledger/issues/292) Wayfinder map; each decision's detail lives on its ticket. The terms Colour Theme, Colour Role, Colour Variant and Colour Appearance are defined in `CONTEXT.md`.

## Status

In design. The Colour Role set is decided ([#298](https://github.com/IanTeda/Personal-Ledger/issues/298), [ADR-0022](adr/0022-seven-stored-colour-roles-with-calculated-shades.md)), the Preferences and `[theme]` precedence ([#300](https://github.com/IanTeda/Personal-Ledger/issues/300), [ADR-0023](adr/0023-colour-theme-preferences-and-theme-role-overrides.md)) the TUI's terminal-colour approach ([#299](https://github.com/IanTeda/Personal-Ledger/issues/299), [ADR-0024](adr/0024-tui-draws-colour-themes-in-rgb-with-opt-in-terminal-colours.md)) the code architecture ([#302](https://github.com/IanTeda/Personal-Ledger/issues/302), [ADR-0025](adr/0025-lib-colour-theme-resolves-shared-colours-from-build-time-ini-files.md)) the built-in Colour Themes ([#301](https://github.com/IanTeda/Personal-Ledger/issues/301)), what is not part of a Colour Theme ([#304](https://github.com/IanTeda/Personal-Ledger/issues/304)) and the Settings controls ([#303](https://github.com/IanTeda/Personal-Ledger/issues/303)). Nothing here is built yet.

## Colour Roles

### One shared set

Both Clients share one flat set of Colour Roles. A role one Client cannot use (the TUI has no shadows or hover) is simply ignored by that Client; there are no per-Client prefixes or separate sets.

### Stored roles

Every Colour Theme stores exactly seven Colour Roles in each of its two Colour Variants. Their names are snake_case and are also the keys of the `[theme]` Configuration section.

| role | purpose |
| --- | --- |
| `foreground` | body text, and the ink every neutral shade is calculated from |
| `background` | the ground every surface sits on |
| `accent` | the one attention colour: primary action, focus ring and focused border, over-budget, warnings, validation errors, matched text, mode badge, checked controls, first chart series |
| `cursor` | the text-entry caret only (the Desktop's block caret and the TUI's drawn `▌`); the focus ring comes from `accent` |
| `muted` | secondary text: labels, column heads, hints |
| `positive` | money in or above zero, connected/healthy status, candlestick up |
| `negative` | money out or below zero, candlestick down — amounts only, never errors |

Warnings and validation errors use `accent`, following the "one accent" rule; `negative` means only an amount. There is no stored `warning`, `error`, `info`, `selection` or chart role.

### Configuration overrides

A `[theme]` section in `lib-config`'s layered Configuration overrides any stored role on that one Client, one key per role. Values are hex colours with or without the leading `#`, parsed by `lib-core`'s `HexColor`:

```ini
[theme]
accent = "#00cc00"
muted = "492943"
```

How `[theme]` layers against the synced Preferences is under [Preferences and Configuration](#preferences-and-configuration).

### Calculated colours

Every other colour is calculated from the seven stored roles by fixed rules in code, with the proportions as named constants. Calculated colours are not `[theme]` keys and cannot be overridden or set by a Colour Theme. Because each is a proportion between two stored roles rather than a fixed value, a Dark Colour Variant gets matching shades for free.

| calculated colour | rule | replaces today's |
| --- | --- | --- |
| Borders, hairlines, dividers, structural rule, hover tint, dialog scrim | `foreground` over `background` at fixed transparencies (e.g. hover 6%, border 30%, structural rule 38%) | `BORDER`, `HAIRLINE`, `HAIRLINE_LIGHT`, `DIVIDER`, `RAIL_DIVIDER`, `STRUCTURAL_RULE`, `HOVER_TINT`, `DIMMER` |
| Drop shadows | `foreground` at fixed transparency | `PALETTE_SHADOW`, `DIALOG_SHADOW` |
| Chrome (panel bands, status line, disabled fields, inset track) | `background` mixed a small step toward `foreground` | `CHROME`, `INSET_TRACK` |
| Faint text tier (placeholders, jump keys, meta) | halfway between `muted` and `background` | `INK_TERTIARY` |
| Selection (current/selected row) | swap: background is `foreground`, text is `background`; muted text on it is a mix of `background` toward `foreground`, starting at 35% and backed off until it reaches the text contrast rule on `foreground` | `INK` as a fill, `INK_ON_DARK*`, and the Desktop's other two selection styles |
| Text shades of `accent`, `positive`, `negative` | the role, darkened or lightened until it reaches 4.5:1 against the surface it sits on (`background`, or `foreground` on a selected row) | `ACCENT_TEXT`, `ACCENT_ON_DARK` |
| Accent tint background (the "base" flag pill) | `accent` at low opacity over `background`, text as an accent text shade | `TAG_ACCENT_BG`, `TAG_ACCENT_TEXT` |
| Info toast | chrome background, `foreground` text, `muted` border, a thin `foreground` bar on the leading edge | — (new) |
| Chart series | series 1 is `accent`; series 2–5 step evenly from `foreground` to the palest `foreground`-over-`background` mix that still reaches 3:1 against `background`; a 6th or later repeats the pattern. Fixed proportions (the earlier 100/70/45/25%) left series 5 at 1.4–2.1:1 in every candidate Colour Theme | the Desktop donut's hand-picked constants, the TUI pie's `Color::Rgb` slices |
| Diverging charts | `positive` / `negative` | — |

Today's overloads are resolved by these roles: negative amounts move from `ACCENT_TEXT`/`Color::Red` to `negative`; the Desktop's three selection styles become the one swap; the TUI's lone `Color::Yellow` match highlight becomes `accent`.

### Contrast

Contrast is measured as a WCAG 2.x ratio. The rules:

- **4.5:1** for normal text against the surface it sits on: `foreground`, `muted`, and the text shades of `accent`, `positive` and `negative` on `background`, and their selected-row equivalents on `foreground`.
- **3:1** for non-text marks: borders, the focus ring, chart series and `positive`/`negative` bars against `background`.
- **7:1** for text in the high-contrast built-in Colour Theme.

Enforcement:

- **Built-in Colour Themes are tested.** A unit test checks every pair that actually appears on screen in both Colour Variants of every built-in Colour Theme.
- **Calculated text shades correct themselves**, so a pale user-chosen `accent` still yields readable accent text.
- **`[theme]` overrides are never rejected.** A failing pair is still used, and the Client logs a `warn` naming the pair and its ratio; rejecting it would look like the setting was ignored.

### TUI fallback to terminal colours

Every role the TUI uses has a fallback to terminal colours, used when `terminal_colours` is on or the terminal has only 16 colours (see [TUI terminal colours](#tui-terminal-colours)). The 16 ANSI colours are not Colour Roles themselves.

| role or calculated colour | fallback |
| --- | --- |
| `foreground` / `background` | the terminal's default fg / bg |
| `accent` | red |
| `cursor` | red (the drawn `▌`) |
| `muted` | `DIM` |
| `positive` | green |
| `negative` | red |
| Selection | `REVERSED` |
| Status line / header bar | `REVERSED` |
| Info toast | default-colour bordered box with a `BOLD` title |
| Chart series | the same series rule, through this table |
| Drop shadows, hover | none |

`accent` and `negative` both fall back to red, as today. The TUI keeps its "never rely on colour alone" rule: negatives also carry `−`, over-budget also overshoots its track, flagged rows also carry `⚑`.

## TUI terminal colours

Decided on [#299](https://github.com/IanTeda/Personal-Ledger/issues/299), [ADR-0024](adr/0024-tui-draws-colour-themes-in-rgb-with-opt-in-terminal-colours.md).

- **RGB by default.** The TUI draws the resolved Colour Theme's RGB values, like the Desktop. With a null `colour_theme` both Clients draw Modernist.
- **Opt-in terminal colours.** `terminal_colours = true` in the TUI's `[Personal-Ledger]` Configuration makes it ignore the Colour Theme (and `[theme]`) and draw with the [fallback table](#tui-fallback-to-terminal-colours), so the terminal's own theme wins. It is Configuration, not a Preference: it never syncs and the Desktop ignores it. There is no "Terminal" Colour Theme.
- **Colour depth.** Truecolor when `COLORTERM` is `truecolor`/`24bit`; otherwise the nearest of the 256 colours; on a 16-colour terminal the fallback table is used automatically, with a `warn`.
- **System in a terminal.** At start the TUI queries the terminal background (OSC 11) and reads `COLORFGBG`; a light background picks the light Colour Variant, and no answer picks dark.
- **Live changes.** Terminals that support colour-scheme notifications (DEC mode 2031) switch Colour Variant live; elsewhere the setting is read once at start. No polling.

## Preferences and Configuration

Decided on [#300](https://github.com/IanTeda/Personal-Ledger/issues/300), [ADR-0023](adr/0023-colour-theme-preferences-and-theme-role-overrides.md).

### Preferences

Two nullable Ledger-scoped Preferences replace today's `preferences.colour_theme` `HexColor` accent column (edit the table's create file; the schema is still in concept):

| column | values | null means |
| --- | --- | --- |
| `colour_theme` | a built-in Colour Theme id, such as `modernist` | the default Colour Theme (Modernist) |
| `colour_appearance` | `light`, `dark`, `system` | System |

Only an explicit choice is stored and synced. There is no SQL `CHECK` on the id: a Client that does not know a synced id (one added in a later release) draws the default Colour Theme and logs a `warn`, and never writes the default back.

### `[theme]` role overrides

`[theme]` holds only the seven Colour Role keys; it cannot name a Colour Theme or a Colour Appearance. A bare key applies to both Colour Variants; a `[theme.light]` or `[theme.dark]` subsection overrides one, and the more specific key wins:

```ini
[theme]
accent = "#cc0000"

[theme.dark]
accent = "#ff6666"
```

As environment variables: `PERSONAL_LEDGER_THEME__ACCENT`, `PERSONAL_LEDGER_THEME__DARK__ACCENT`. A value that is not a colour, or an unknown key, is ignored with a `warn` naming the key and value; the Client still starts.

### Resolution

1. Colour Theme from the `colour_theme` Preference (or the default).
2. Colour Variant from the `colour_appearance` Preference; System follows the OS light/dark setting (the TUI asks the terminal, see [TUI terminal colours](#tui-terminal-colours)).
3. `[theme.<variant>]` keys, then bare `[theme]` keys for any role still unset, laid over that Colour Variant's roles.
4. Calculated colours from the result.

Preference changes and OS light/dark changes apply live. `[theme]` is read once at start, so changing it needs a restart.

### Settings

Decided on [#303](https://github.com/IanTeda/Personal-Ledger/issues/303), from an HTML prototype kept on the [`prototype/303-colour-theme-settings`](https://github.com/IanTeda/Personal-Ledger/tree/prototype/303-colour-theme-settings/prototypes) branch.

The Colour Theme and Colour Appearance pickers are always enabled: `[theme]` overrides Colour Roles, never the Colour Theme or Colour Appearance themselves, so neither control is ever locked.

**Both Clients.**

- A change applies live on selection; there is no Apply button.
- System names the Colour Variant it resolved to: "System (currently Dark)". If the OS or terminal reports nothing, System uses Dark and says so: "System (not detected, using Dark)".
- While any role is overridden, a note under the picker names the overridden roles and the Configuration file they came from, and says a restart is needed to change them (`[theme]` is read once at start).
- Contrast failures that `resolve` returns (only possible through `[theme]` overrides, since every built-in Colour Theme passes) show as warning lines under that note, for example "accent on background is 2.4:1, needs 3:1".
- Colour Theme names are Messages in the shared `lib-locale` Catalogue, brand names (Catppuccin, Gruvbox, Nord) included and left untranslated.

**Desktop (Display section).** A full-width Colour Theme group below the existing 300px column holds both controls:

- Colour Appearance is a Light / Dark / System segmented control, the same as the section's other segmented fields, above the grid.
- Colour Theme is a wrapping grid of ~160px preview cards. Each card draws a miniature ledger (header bar, three rows with one selected, a positive and a negative amount, an accent chip) in that Colour Theme, with its name underneath.
- Cards draw the Colour Variant in effect, run through `resolve` with this Client's `[theme]` overrides, so they show what would actually appear.
- The chosen card has a 2px border in the current Colour Theme's `accent` and a bold name (radius stays 0).
- Keyboard: Tab onto the grid, arrows or `h`/`j`/`k`/`l` move focus, Enter selects. Moving focus or hovering never previews. The command palette has one command per Colour Theme and per Colour Appearance.

**TUI (Settings view).** Colour Theme and Colour Appearance are rows in a Display group. Enter opens a list popup (the §4b in-place editor pattern) where each Colour Theme row carries seven `█` swatch cells; `j`/`k` previews live, Enter keeps, Esc reverts. With `terminal_colours = true` both rows stay editable, since the Preferences still sync to the user's other Clients, and a note names the `terminal_colours` Configuration key as the reason this Client draws the terminal's colours.

## Built-in Colour Themes

Decided on [#301](https://github.com/IanTeda/Personal-Ledger/issues/301), from a swatch and contrast prototype kept on the [`prototype/301-built-in-colour-themes`](https://github.com/IanTeda/Personal-Ledger/tree/prototype/301-built-in-colour-themes/prototypes) branch.

Five built-in Colour Themes ship, each with both Colour Variants. Modernist is the default. Every value below passes the [contrast rules](#contrast) in both Colour Variants with the calculated-colour rules above; the ports are adjusted only where their published colours fail.

| id | Colour Theme | source and licence | adjustments |
| --- | --- | --- | --- |
| `modernist` | Modernist | ours | light: `positive` darkened from `#2ecc71` (fails 3:1 on the ground), new `negative`; dark Colour Variant new, on the handoff's `#161413` ground |
| `high_contrast` | High Contrast | ours | meets 7:1 for text |
| `catppuccin` | Catppuccin (Latte / Mocha) | [catppuccin](https://github.com/catppuccin/catppuccin), MIT | Latte: `muted` is Subtext 1 not Subtext 0, `positive` darkened, `cursor` is Red not Rosewater |
| `gruvbox` | Gruvbox | [morhetz/gruvbox](https://github.com/morhetz/gruvbox), MIT/X11 | light: `muted` is `fg3` not `fg4` |
| `nord` | Nord | [nordtheme/nord](https://github.com/nordtheme/nord), MIT | Nord has no light Colour Variant; ours inverts Polar Night on Snow Storm. Dark `muted` lightened |

Rejected: Tokyo Night (Day fails muted text, 3.6:1), Solarized (light `foreground` itself fails, 4.1:1) and Everforest (light `muted`, `accent` and `positive` fail) — each would need changes deep enough that it stops being that Colour Theme.

| id | variant | foreground | background | accent | cursor | muted | positive | negative |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `modernist` | light | `#201e1d` | `#f3f2f2` | `#ec3013` | `#ec3013` | `#605d5d` | `#1e8e4e` | `#b3261e` |
| `modernist` | dark | `#f3f2f2` | `#161413` | `#ff4a2b` | `#ff4a2b` | `#a19d9c` | `#3ddc84` | `#ff7a66` |
| `high_contrast` | light | `#000000` | `#ffffff` | `#b30000` | `#b30000` | `#3a3a3a` | `#005c1f` | `#9a0000` |
| `high_contrast` | dark | `#ffffff` | `#000000` | `#ffd400` | `#ffd400` | `#d0d0d0` | `#5cff8a` | `#ff9a8a` |
| `catppuccin` | light | `#4c4f69` | `#eff1f5` | `#8839ef` | `#d20f39` | `#5c5f77` | `#388c26` | `#d20f39` |
| `catppuccin` | dark | `#cdd6f4` | `#1e1e2e` | `#cba6f7` | `#f5e0dc` | `#a6adc8` | `#a6e3a1` | `#f38ba8` |
| `gruvbox` | light | `#3c3836` | `#fbf1c7` | `#af3a03` | `#3c3836` | `#665c54` | `#79740e` | `#9d0006` |
| `gruvbox` | dark | `#ebdbb2` | `#282828` | `#fe8019` | `#ebdbb2` | `#a89984` | `#b8bb26` | `#fb4934` |
| `nord` | light | `#2e3440` | `#eceff4` | `#5e81ac` | `#2e3440` | `#4c566a` | `#4f7a3a` | `#bf616a` |
| `nord` | dark | `#d8dee9` | `#2e3440` | `#88c0d0` | `#d8dee9` | `#9aa5b8` | `#a3be8c` | `#bf616a` |

These values are the content of the `themes/<id>.ini` files. Display names are the Messages `colour-theme-<id>` (en-US as in the Colour Theme column; a port keeps its proper name). A port's licence notice ships with the crate alongside its INI file.

## Architecture

Decided on [#302](https://github.com/IanTeda/Personal-Ledger/issues/302), [ADR-0025](adr/0025-lib-colour-theme-resolves-shared-colours-from-build-time-ini-files.md).

### `lib-colour-theme`

A new pure library crate, `crates/libs/lib-colour-theme` (package `lib_colour_theme`), shared by both bins. It owns the Colour Theme, Colour Variant and Colour Role types, the built-in Colour Themes, resolution, the calculated-colour rules and the contrast maths. It does no I/O, does no logging and does not depend on `lib-config`.

### Built-in Colour Theme files

Each built-in Colour Theme is one INI file, `lib-colour-theme/themes/<id>.ini`, whose filename is the Colour Theme id. A `[light]` and a `[dark]` section each set all seven Colour Roles, using the same keys and hex values as `[theme]`:

```ini
; modernist.ini
[light]
foreground = "#201e1d"
background = "#f3f2f2"
accent = "#ec3013"
; ...

[dark]
; ...
```

The crate's `build.rs` parses the files and generates Rust `const`s, following the `lib-locale-build` pattern. A missing role, a missing Colour Variant or a bad hex value fails the build. The parser is a plain function, so a later runtime loader for user-defined Colour Themes can reuse it. The display name is not in the file: it is the Message `colour-theme-<id>` in `lib-locale`.

### Resolution

`resolve(inputs) -> (ResolvedColours, Vec<ContrastFailure>)` is a pure function. Its inputs are the `colour_theme` and `colour_appearance` Preferences, the `[theme]` override map and the system light/dark setting. `ResolvedColours` holds the seven resolved roles plus every [calculated colour](#calculated-colours) as neutral RGBA (borders, hover, chrome, faint text, the selection pair, text shades, accent tint, shadows, chart series). Clients only convert that RGBA into their own colour types, and each ignores fields it cannot draw.

`lib-config` parses `[theme]`/`[theme.light]`/`[theme.dark]` into a plain role-to-`HexColor` map and logs the `warn` for unparseable values and unknown keys itself. The resolver returns contrast failures as data, and the bin logs each one at `warn`.

The resolver does not detect changes. Each Client watches its own inputs (a Preference change, the gpui window appearance on the Desktop, OSC 11/mode 2031 in the TUI) and calls `resolve` again.

### Desktop

A `gpui` Global `Colours(ResolvedColours)`, read through an extension trait as `cx.colours().border`, replaces the `theme::color` consts. `theme.rs` keeps `type_scale` and `spacing`. When the Colour Theme changes, the Desktop replaces the Global and calls `cx.refresh_windows()`. The same step's `apply_to_component_theme()` writes the resolved colours into `gpui_component::Theme`, so charts and tables follow the Colour Theme and its Colour Variant.

### TUI

A `Colours` struct of ratatui `Style`s is built from `ResolvedColours`, the terminal's colour depth and the `terminal_colours` switch. It maps truecolor RGB to the nearest of the 256 colours, and uses the [fallback table](#tui-fallback-to-terminal-colours) when terminal colours are on or only 16 colours are available. This ANSI logic lives in the TUI bin, not the shared crate. `Colours` lives on `Shell` and is passed into each view's `render`, not held in a global. Views ask for named styles (`c.muted()`, `c.selection()`) instead of `Color::`.

### Preferences before persistence

Until the Clients' Settings read `preferences` from the database, each Client keeps `colour_theme` and `colour_appearance` in memory, starting null (Modernist, System), and its Settings pickers change them live. Wiring them to the database is a slice of the implementation sweep.

### Tests and guards

`lib-colour-theme`'s unit tests cover INI parsing, resolution precedence, the calculated-colour rules and the contrast matrix for every built-in Colour Theme. `clippy.toml` bans `gpui::rgb`/`gpui::rgba` through `disallowed-methods`, with an `#[expect]` in the Desktop's one mapping module. The TUI's `Color` type is not banned; a review note keeps new `Color::` literals out of views.

## Not part of a Colour Theme

A Colour Theme is colour only: its seven Colour Roles, and nothing else in a Colour Theme or in `[theme]` ([#304](https://github.com/IanTeda/Personal-Ledger/issues/304)). Typography, spacing and radius stay fixed by the Modernist design system, and none of them is a Preference or Configuration:

- **Font family**: Archivo, bundled with the Desktop. The layout's widths, figure alignment and kicker tracking are tuned to its metrics. In the TUI the terminal's own font applies.
- **Type scale and weights**: `theme::type_scale` stays fixed.
- **Spacing**: `theme::spacing` stays fixed, with no compact or comfortable density modes, which would double the layouts to test.
- **Radius**: 0 everywhere.
- **TUI borders and glyphs**: the plain line set stays fixed, with no choice of border style.

Two related questions sit outside this map. A single Desktop UI scale factor that multiplies the type scale and spacing together (never separate font sizes) is its own effort ([#313](https://github.com/IanTeda/Personal-Ledger/issues/313)). An ASCII fallback for terminals that can't draw box-drawing or Unicode glyphs is a question of what the terminal can do, not a styling Preference. It is deferred, and would follow the same pattern as the 16-colour auto-fallback: detect first, and add a switch only if detection proves unreliable.

## Open items

- Everything on the tickets listed under [Status](#status).
- Rewriting the Modernist handoffs' "introduce no values outside this set" rule to speak in Colour Roles.
- A TUI ASCII glyph fallback (see [Not part of a Colour Theme](#not-part-of-a-colour-theme)).
