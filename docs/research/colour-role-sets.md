# Research: Colour Role sets in established theme systems

**Question.** Issue [#293](https://github.com/IanTeda/Personal-Ledger/issues/293), part of the [Colour Themes for the Desktop and TUI](https://github.com/IanTeda/Personal-Ledger/issues/292) map: which Colour Roles do established theme systems define, and which recur often enough to be a sensible baseline for ours? This note is primary-source research only, feeding the later Colour Role decision ticket — it does not decide the baseline. All claims are cited to primary sources: official docs, the systems' own schema/spec files, and vendored/published source (`gpui-component` was read directly from the vendored crate at `~/.cargo/registry/src/.../gpui-component-0.5.1`, pinned by this workspace's `Cargo.toml` at `0.5.1`). Research was carried out on 2026-09-26; where an upstream doc doesn't spell out a mechanism explicitly, that gap is called out rather than guessed at.

Vocabulary note, per `CONTEXT.md`/the map: this repo calls a named colour set a **Colour Theme**, never a "palette" (that word means the command palette here), and **Colour Role** is the term for a named slot such as `accent` or `border`. Where a source's own docs say "palette" or "scheme", that is quoted verbatim as their term, not adopted as ours.

## 1. base16 / base24

**base16** (spec maintained at [tinted-theming/home](https://github.com/tinted-theming/home), successor to chriskempson/base16) defines exactly 16 roles, `base00`–`base0F`, per the [styling guide](https://github.com/tinted-theming/home/blob/main/styling.md):

| Role | Role name |
|---|---|
| base00 | Default background |
| base01 | Lighter background (status bars, line highlighting) |
| base02 | Selection background |
| base03 | Comments, invisibles, line highlighting |
| base04 | Dark foreground (status bars) |
| base05 | Default foreground, caret, delimiters, operators |
| base06 | Light foreground |
| base07 | Lightest foreground |
| base08 | Variables, XML tags, markup link text, diff deletions |
| base09 | Integers, booleans, constants, XML attributes |
| base0A | Classes, markup bold, search-text background |
| base0B | Strings, inherited classes, markup code, diff insertions |
| base0C | Support, regex, escape characters, markup quotes |
| base0D | Functions, methods, attribute IDs, headings |
| base0E | Keywords, storage, selectors, markup italics, diff changes |
| base0F | Deprecated code, embedded language tags |

Light vs dark is not a separate role set — it's a *direction*: base00–base07 "should span from dark to light" for a dark scheme and light to dark for a light scheme (styling guide, via search-indexed excerpt of the same doc). A scheme file carries a `variant: dark` / `variant: light` metadata field (per [`builder.md`](https://github.com/tinted-theming/home/blob/main/builder.md): "Currently only 'light' and 'dark' are used") rather than the scheme shipping two colour sets.

Semantic roles are folded into the same 16 slots by *convention*, not by name: "accent" is really base0D (functions/headings), "positive" is base0B (strings/diff-insertions), "negative" is base08 (variables/diff-deletions), "warning" is base0A or base0F depending on template. There is no dedicated `warning`/`muted`/`border`/`hover`/`focus`/`chart` role — templates for specific apps (terminal, editor, etc.) reinterpret the 16 bases into whatever their own role needs, which is exactly the gap base24 exists to close.

**base24** ([tinted-theming/base24](https://github.com/tinted-theming/base24)) is base16 plus 8 more slots, `base10`–`base17`, aimed specifically at giving terminals a *complete* 16-colour ANSI table instead of reusing base16 slots twice:

| Role | Purpose | ANSI mapping |
|---|---|---|
| base10 | Darker background | none (falls back to base00) |
| base11 | Darkest background | none (falls back to base00) |
| base12 | Bright red | ANSI bright red (9) |
| base13 | Bright yellow | ANSI bright yellow (11) |
| base14 | Bright green | ANSI bright green (10) |
| base15 | Bright cyan | ANSI bright cyan (14) |
| base16 | Bright blue | ANSI bright blue (12) |
| base17 | Bright magenta | ANSI bright magenta (13) |

("Bright colors can have a higher luminosity relative to its non-bright counterpart", per base24's [styling.md](https://github.com/tinted-theming/base24/blob/main/styling.md).) The conventional base16→ANSI terminal mapping (used by base16 terminal templates, e.g. base16-shell) is: black=base00, red=base08, green=base0B, yellow=base0A, blue=base0D, magenta=base0E, cyan=base0C, white=base05, with the "bright" row reusing base03, base08, base0B, base0A, base0D, base0E, base0C, base07 — i.e. base16 alone *fakes* a 16-colour ANSI table by doubling up 8 of its 16 roles, which is precisely the fidelity problem base24's 8 new slots fix.

Contrast handling: no explicit contrast-ratio rule in either spec; correctness is by convention ("should be legible against the ordinary background") and manual scheme authoring, not an enforced or computed check.

## 2. Terminal emulators (Alacritty, Ghostty, Kitty) and Omarchy

**Alacritty** ([config-alacritty.html](https://alacritty.org/config-alacritty.html)) `[colors]`:

- `primary`: `background`, `foreground`, `dim_foreground`, `bright_foreground`
- `cursor` / `vi_mode_cursor`: each `{ text, cursor }`
- `selection`: `{ text, background }`
- `normal` / `bright` / `dim`: each `{ black, red, green, yellow, blue, magenta, cyan, white }` — i.e. three full 8-colour rows (24 ANSI-adjacent slots total, the `dim` row is optional and synthesised if absent)
- `indexed_colors`: array of `{ index, color }` for the 256-colour extended table
- `search.matches` / `search.focused_match`, `hints.start`/`hints.end`, `line_indicator`, `footer_bar`: each `{ foreground, background }`
- Behavioural flags: `transparent_background_colors`, `draw_bold_text_with_bright_colors`

No dedicated light/dark Colour Variant field — a config *is* one Colour Variant; switching is done by pointing at a different config/import.

**Ghostty** ([features/theme](https://ghostty.org/docs/features/theme), [config/reference](https://ghostty.org/docs/config/reference)) theme files are flat key=value: `background`, `foreground`, `cursor-color`, `cursor-text`, `selection-background`, `selection-foreground`, `palette = N=#hex` (N = 0–255, covering the 16 ANSI + 240 extended slots), plus `palette-generate`/`palette-harmonious` to derive an extended palette from the base 16. Ghostty is the one primary source in this survey with an explicit, first-class **contrast** setting: `minimum-contrast` — "the minimum contrast ratio between the foreground and background colors", on the WCAG 2.0 1–21 scale — the terminal will adjust the rendered foreground to meet it rather than trusting the theme author. It also has dedicated `search-foreground`/`search-background` and `search-selected-foreground`/`search-selected-background` roles. No light/dark Colour Variant field on the theme file itself; Ghostty instead supports pointing `theme` at `light:<name>,dark:<name>` and following the OS appearance (`features/theme` doc).

**Kitty** (`kitty.conf`, per its docs and manpage): `foreground`, `background`, `selection_foreground`, `selection_background`, `cursor`, `cursor_text_color`, and `color0`–`color15` for the 16 ANSI slots (colours 16–255 available as `color16`–`color255`). No first-class contrast setting found in the reference docs.

**Omarchy** ([Making your own theme manual](https://omarchy.org/manual/making-your-own-theme/); [basecamp/omarchy `themes/nord/colors.toml`](https://github.com/basecamp/omarchy/blob/dev/themes/nord/colors.toml)) — a theme is a directory whose one mandatory file is `colors.toml`, which the CLAUDE.md ticket text summarises as "foreground, background, cursor, selection fg/bg, 16 ANSI"; the actual (pre-"Quattro") shape confirms that exactly:

```
accent, cursor, foreground, background, selection_foreground, selection_background,
color0 .. color15
```

Omarchy's build step then template-substitutes these into per-app configs (Alacritty, Ghostty, Kitty, foot, btop, Hyprland, Neovim, Helix, VS Code, Waybar, …), so this flat set is the *lowest common denominator* Omarchy found sufficient to drive every one of those consumers. A newer "Quattro" schema (Omarchy 4.0+, seen in `omacom/omarchy` fork docs) widens this to named semantic slots — `mode`, `accent`, `background`/`dark_background`/`darker_background`/`lighter_background`, `foreground`/`dark_foreground`/`light_foreground`/`bright_foreground`, plus `red`/`yellow`/`orange`/`green`/`cyan`/`blue`/`magenta`/`brown` and their `bright_*` counterparts, with legacy `bg`/`fg` aliases kept for back-compat — this is worth flagging as the direction terminal-scheme-driven systems tend to evolve in once "16 ANSI colours" stops being enough for a whole desktop's theming. Light vs dark Colour Variant: a `mode = "light"` field at the top of `colors.toml` (Quattro), or the older convention of dropping an empty `light.mode` marker file next to it.

## 3. Helix and Zed (editor theme scopes)

**Helix** ([themes.html](https://docs.helix-editor.com/themes.html)) themes are TOML mapping dotted `ui.*` scope keys to `{ fg, bg, underline, modifiers }` or a bare colour string, with a `[palette]` section of reusable named colours and an `inherits = "<theme>"` mechanism for extending an existing theme. Selected UI roles, by category:

- Canvas/text: `ui.background`, `ui.background.separator`, `ui.text`, `ui.text.focus`, `ui.text.inactive`, `ui.text.directory`
- Cursor: `ui.cursor` (+ `.normal`/`.insert`/`.select`/`.match` mode variants), `ui.cursor.primary`
- Selection/highlight: `ui.selection`, `ui.selection.primary`, `ui.highlight`, `ui.highlight.frameline`
- Line/gutter: `ui.linenr`, `ui.linenr.selected`, `ui.cursorline.primary/secondary`, `ui.gutter`, `ui.gutter.selected`
- Chrome: `ui.statusline` (+ mode variants), `ui.bufferline`/`ui.bufferline.active`, `ui.window`, `ui.popup`/`ui.popup.info`, `ui.menu`/`ui.menu.selected`/`ui.menu.scroll`, `ui.help`
- Virtual text: `ui.virtual.ruler`, `.whitespace`, `.indent-guide`, `.inlay-hint` (+ parameter/type variants), `.wrap`, `.jump-label`
- Diagnostics: gutter `error`/`warning`/`info`/`hint`, editing-area `diagnostic.error`/`.warning`/`.info`/`.hint`, plus `diagnostic.unnecessary`/`.deprecated`
- Debug: `ui.debug.breakpoint`, `ui.debug.active`

No documented light/dark Colour Variant mechanism beyond "author two separate theme files" — Helix's docs describe the palette/inherits system but not a paired-Colour-Variant concept the way this repo's Colour Theme does. No explicit contrast-ratio handling documented; syntax scopes (tree-sitter capture names like `function`, `keyword`, `string`) are a separate namespace from the `ui.*` roles, resolved independently by longest-match.

**Zed** ([themes doc](https://zed.dev/docs/themes); schema at `https://zed.dev/schema/themes/v0.2.0.json`) is the deepest role set surveyed — reported at "200+ color tokens" for backgrounds, borders and text state alone, before syntax. Selected structure from the schema:

- Theme metadata: `name`, `appearance` (`"light"` or `"dark"` — this is where Zed's Colour Variant lives, one JSON object per Colour Variant inside a theme family)
- Surfaces: `background`, `surface.background`, `elevated_surface.background`, `element.background`/`.active`/`.hover`/`.selected`/`.disabled`, `ghost_element.*` (the same states for a "ghost"/transparent element), `drop_target.background`
- Text: `text`, `text.muted`, `text.placeholder`, `text.disabled`, `text.accent`
- Borders: `border`, `border.variant`, `border.focused`, `border.selected`, `border.disabled`, `border.transparent`
- Status/semantic, each with `.background`/`.border` companions: `error`, `warning`, `info`, `success`, `created`, `modified`, `renamed`, `conflict`, `ignored`, `hidden`, `unreachable`, `predictive`, `hint`
- Terminal fidelity: `terminal.background`/`.foreground`, `terminal.ansi.{black,red,green,yellow,blue,magenta,cyan,white}` plus `bright_*` and `dim_*` rows, `terminal.ansi.background`
- Collaboration: `players[]`, each `{ background, cursor, selection }` — a role family none of the other systems surveyed have, because it's rendering *other users'* cursors/selections simultaneously

No explicit numeric contrast-ratio handling found in the schema or docs surveyed.

## 4. VS Code workbench colours

[Theme Color Reference](https://code.visualstudio.com/api/references/theme-color) documents several hundred keys grouped by contributing UI area (editor, activity bar, tabs, panel, list, button, badge, …), but a small "Base colors" set underpins all of them: `foreground`, `disabledForeground`, `errorForeground`, `descriptionForeground`, `focusBorder`, `widget.border`, `widget.shadow`, `selection.background`, `icon.foreground`, `sash.hoverBorder`. On top of that base, the hundreds of component-specific colours follow a small number of very consistent naming suffixes: `*.background`, `*.foreground`, `*.border`, `*.hoverBackground`, `*.activeBackground`/`*.activeForeground`/`*.activeBorder` — i.e. VS Code gets its huge role count not from hundreds of *distinct concepts* but from cartesian-producting a handful of state suffixes (default/hover/active/disabled/border) across every individual UI surface.

Colour Variant handling is structural, not a colour-role field: a colour theme extension declares each theme's base type via `contributes.themes[].uiTheme` ∈ `{ vs, vs-dark, hc-black, hc-light }` (light / dark / high-contrast-dark / high-contrast-light — [extension manifest reference](https://code.visualstudio.com/api/extension-guides/color-theme) and search-confirmed enumeration), and the theme JSON itself repeats this as a `"type"` field (e.g. `"type": "dark"`). High contrast is thus a first-class *fourth* Colour Variant in VS Code's model, not a derived/algorithmic adjustment — it ships as its own separately-authored colour set (`workbench.colorCustomizations` can further override per-theme, but that's user override, not the base contrast mechanism).

## 5. `gpui-component`'s `ThemeColor` (primary source: vendored crate)

Read directly from the pinned `gpui-component = { version = "0.5.1" }` dependency, `src/theme/theme_color.rs` and `src/theme/default-theme.json`. `ThemeColor` is a flat Rust struct of ~90 `Hsla` fields (not JSON-nested at the Rust level, though the on-disk theme JSON uses dotted keys like `accent.background`/`accent.foreground` that get flattened into it). By category:

- Canvas/text: `background`, `foreground`
- Accent/state families, each following a `{name}` / `{name}_hover` / `{name}_active` / `{name}_foreground` pattern: `accent`/`accent_foreground`, `primary`/`primary_hover`/`primary_active`/`primary_foreground`, `secondary`/`secondary_hover`/`secondary_active`/`secondary_foreground`, `danger`/`danger_hover`/`danger_active`/`danger_foreground`, `success`/`success_hover`/`success_active`/`success_foreground`, `warning`/`warning_hover`/`warning_active`/`warning_foreground`, `info`/`info_hover`/`info_active`/`info_foreground`
- Muted: `muted`, `muted_foreground` (doc comment: "used for disabled text")
- Border/ring/input: `border`, `input`, `ring` ("used for focus ring")
- Selection/caret: `selection`, `caret`
- Overlay/scrollbar/drag: `overlay`, `scrollbar`, `scrollbar_thumb`, `scrollbar_thumb_hover`, `drag_border`, `drop_target`, `window_border`
- Component-specific surfaces: `popover`/`popover_foreground`, `sidebar` family, `tab`/`tab_bar` family, `table` family, `list` family, `title_bar`/`title_bar_border`, `accordion`, `group_box`, `tiles`, `skeleton`, `progress_bar`, `slider_bar`/`slider_thumb`, `switch`/`switch_thumb`, `description_list_label`
- Chart series: `chart_1`..`chart_5` (five, not the six-plus some other systems use) plus `bullish`/`bearish` specifically for candlestick colouring
- Base hues, each with a `_light` companion: `red`/`red_light`, `green`/`green_light`, `blue`/`blue_light`, `yellow`/`yellow_light`, `magenta`/`magenta_light`, `cyan`/`cyan_light` — a 6-hue "raw colour" layer sitting underneath the semantic roles, closer in spirit to base16's raw 16 than to VS Code's all-semantic naming

Colour Variant: `ThemeColor::light()` / `ThemeColor::dark()` return the two halves of `DEFAULT_THEME_COLORS`, keyed by `ThemeMode` (`Light`/`Dark` — `theme/mod.rs`); the on-disk `default-theme.json` schema literally pairs a `"mode": "light"` and `"mode": "dark"` object under one `"themes"` array per named Colour Theme (`"name": "Default Light"` / `"Default Dark"`), which is exactly the "every built-in Colour Theme has paired light and dark Colour Variants" shape the map already settled on for this repo. `default-theme.json` also carries a separate `"highlight"` block per Colour Variant for syntax/editor colours (`editor.foreground`, `editor.line_number`, and a `syntax.*` map keyed by tree-sitter-style capture names) — i.e. `gpui-component` keeps UI Colour Roles and syntax-highlighting roles as two parallel systems, the same split Helix and Zed make. No contrast-ratio handling found in the struct or JSON schema — correctness is by theme authoring only.

## 6. ratatui theme crates

Ratatui's own core (`ratatui::style::{Color, Style}`) has **no Colour Role concept whatsoever** — `Color` is just an enum of the 16 ANSI names, `Rgb(u8,u8,u8)`, `Indexed(u8)`, `Reset`; `Style` is `{ fg, bg, underline_color, add_modifier, sub_modifier }`. Any Colour Role layer is bolted on by an application or a third-party crate, not supplied by ratatui itself — a materially different starting point from every other system surveyed, and directly relevant to how much this repo has to invent for the TUI side. Two illustrative crates:

- **`catppuccin`** ([docs.rs](https://docs.rs/catppuccin/latest/catppuccin/)) exposes the Catppuccin colour system's 26 named hues per Flavor — `rosewater`, `flamingo`, `pink`, `mauve`, `red`, `maroon`, `peach`, `yellow`, `green`, `teal`, `sky`, `sapphire`, `blue`, `lavender`, `text`, `subtext1`, `subtext0`, `overlay2`, `overlay1`, `overlay0`, `surface2`, `surface1`, `surface0`, `base`, `mantle`, `crust` — and converts them to `ratatui::style::Color`. Colour Variant handling: four `Flavor`s, **one light** (`Latte`) and **three dark**, ordered by contrast/darkness (`Frappé` → `Macchiato` → `Mocha`, the "original", darkest) — confirmed against [catppuccin.com/palette](https://catppuccin.com/palette/): "The theme comes in one light and three dark variants." This is a case of *ramped* neutrals (`base`/`mantle`/`crust` = background darkening; `text`/`subtext1`/`subtext0` = foreground dimming; `overlay0-2`/`surface0-2` = the muted/border/hover middle ground) rather than named `muted`/`border`/`hover` roles — the role is expressed positionally in the ramp, not by name, which is closer to base16's approach than to VS Code's or `gpui-component`'s.
- **`ratatui-themes`** ([docs.rs](https://docs.rs/ratatui-themes/latest/ratatui_themes/)) defines a `ThemePalette` that is explicitly semantic and small: `accent` ("primary accent for highlights and active elements"), `secondary`, `bg`, `fg`, `muted` ("dimmed text... disabled elements"), `selection`, plus four status colours `error`/`warning`/`success`/`info`. It ships 15+ named themes (Dracula, Nord, Catppuccin, Gruvbox, Tokyo Night, Solarized, …) each pre-classified `is_light()`/`is_dark()` by background brightness, rather than authored as an explicit paired Colour Variant the way `gpui-component` or VS Code's `uiTheme` do it. No `border`, `hover`, `focus`, or `chart` role, and no contrast-ratio handling — this crate is the smallest role set in the whole survey, essentially base16's semantic subset (bg/fg/muted/accent/selection + 4 status colours) re-expressed for ratatui's `Style`.

## Comparison table

| System | Role-naming style / rough count | Light/Dark Colour Variant expression | Accent | Positive / Negative | Warning | Muted / dim | Border | Hover / focus ring | Chart / series | Contrast handling |
|---|---|---|---|---|---|---|---|---|---|---|
| base16 | positional slots, 16 | `variant:` metadata field on the scheme; base00–07 direction flips | base0D (function/heading, by convention) | base0B / base08 (by convention) | base0A or base0F (by convention) | base03 (comments/dim, by convention) | none | none | none (16 raw slots only) | none |
| base24 | positional slots, 24 | same as base16 | same as base16 | same, + guaranteed bright ANSI red/green | same | same | none | none | none | none |
| Alacritty | grouped rows (`normal`/`bright`/`dim` × 8), + a few named extras | one Colour Variant per config file; no in-file field | none named | ANSI red/green (by convention) | ANSI yellow (by convention) | `dim` row | none | `search`, `hints`, `line_indicator` come closest | none | none |
| Ghostty | flat `background`/`foreground`/`cursor-*`/`selection-*` + 256-slot `palette` | `theme = light:<x>,dark:<y>`; follows OS appearance | none named | ANSI red/green (by convention) | ANSI yellow (by convention) | none named | none | `search-selected-*` | none | **`minimum-contrast`**, WCAG 1–21 scale, auto-adjusts foreground |
| Kitty | flat `foreground`/`background`/`selection_*`/`cursor*` + `color0`-`color15` | one Colour Variant per conf; no in-file field | none named | ANSI red/green (by convention) | ANSI yellow (by convention) | none named | none | none | none | none |
| Omarchy (pre-Quattro) | flat: `accent`, `cursor`, `foreground`, `background`, `selection_foreground/background`, `color0`-`color15` | `light.mode` marker file | `accent` (named) | ANSI red/green (by convention) | ANSI yellow (by convention) | none named | none | none | none | none |
| Omarchy (Quattro) | flat, semantic: `mode`, `accent`, `background`×4 depths, `foreground`×4 depths, named hues + `bright_*` | `mode = "light"` field | `accent` (named) | `red`/`green` (named hues) | `orange`/`yellow` (named) | none named (closest: `dark_foreground`) | none | none | none | none |
| Helix `ui.*` | dotted scopes, ~40+ | no Colour Variant field; author two theme files | none named (closest: `ui.text.focus`) | `diagnostic.error` only (no positive role) | `diagnostic.warning` | `ui.text.inactive` | none named | `ui.menu.selected`, `ui.cursor.*` modes | none | none |
| Zed `theme.json` | dotted families, 200+ | `appearance: "light"\|"dark"` field, per theme object | `text.accent` | `success` / `error` | `warning` | `text.muted` | `border`, `border.variant`, `border.focused`, `border.selected` | `element.hover`, `element.selected`, `border.focused` | none named (syntax highlighting is separate) | none |
| VS Code workbench | dotted, hundreds, `*.background/foreground/border/hoverBackground/activeBackground` suffix pattern | `uiTheme` ∈ `{vs, vs-dark, hc-black, hc-light}` + `"type"` field; hc is a 4th Colour Variant | none single (per-component `*.foreground`) | `gitDecoration.addedResourceForeground` / `...deletedResourceForeground` (git-specific, no generic role) | none generic (editor-specific `editorWarning.foreground`) | `disabledForeground`, `descriptionForeground` | `focusBorder`, `widget.border`, per-component `*.border` | `*.hoverBackground`, `focusBorder` | none | none generic; high-contrast handled as a separate authored Colour Variant, not computed |
| `gpui-component` `ThemeColor` | flat struct, ~90 fields, `{name}/_hover/_active/_foreground` pattern | `ThemeMode::Light`/`Dark`; JSON pairs `"mode": "light"`/`"dark"` under one theme | `accent`/`accent_foreground` | `success` / `danger` | `warning` | `muted`/`muted_foreground` | `border`, `input` | `ring` (focus ring, named), `*_hover` per family | `chart_1..chart_5`, + `bullish`/`bearish` | none |
| ratatui core | none (raw `Color`/`Style` only) | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |
| `catppuccin` crate | positional ramp, 26 hues × 4 Flavors | 4 named `Flavor`s, 1 light + 3 dark, ordered by darkness | `mauve`/`blue` (by convention, not named "accent") | `red`/`green` (named hues) | `yellow`/`peach` (named hues) | `overlay0-2` (ramp position) | `surface0-2` (ramp position) | none named | none | none |
| `ratatui-themes` crate | flat, semantic, ~10 fields | `is_light()`/`is_dark()` derived from bg brightness, not authored | `accent`, `secondary` | `error` / `success` | `warning` | `muted` | none | none | none | none |

## Proposed union/intersection baseline

Counting how many of the 14 systems above name (or convention-map) each concept independently gives a rough recurrence signal, then grouping into families:

**Near-universal (appears in nearly every system, in some form) — clear baseline candidates:**

- `background` / `foreground` — universal; every system has this pair in some form (even ratatui core, as `Style.bg`/`.fg`).
- `accent` — named explicitly in Omarchy, Zed (`text.accent`), `gpui-component`, `ratatui-themes`; convention-mapped (a single "brand" hue) everywhere else. Strong candidate.
- Positive/success and negative/error/danger — named explicitly in Zed, VS Code (git-specific), `gpui-component`, `ratatui-themes`, `ratatui-themes`; convention-mapped (red/green) in base16/24, terminal schemes, Catppuccin. Strong candidate as two roles: `positive`/`negative` (repo vocabulary should pick one pair of terms and use it consistently — CLAUDE.md's own ticket text already says "positive/negative").
- `warning` — named explicitly in Zed, `gpui-component`, `ratatui-themes`, Helix (`diagnostic.warning`); convention-mapped (yellow) elsewhere. Strong candidate.
- `muted`/dim text — named in `gpui-component` (`muted_foreground`), Zed (`text.muted`), `ratatui-themes` (`muted`), Helix (`ui.text.inactive`), Omarchy Quattro (`dark_foreground`), Catppuccin (ramp position). Strong candidate as one role, `muted` (foreground) — a separate muted *background* (`gpui-component`'s plain `muted`) is a smaller but still-recurring second role.
- `border` — named in Zed (with 5 sub-states), `gpui-component`, VS Code (`*.border` everywhere); absent from every terminal-scheme system (terminals don't draw chrome borders) and from base16/24 and Catppuccin (no dedicated slot, though `surface`/`overlay` ramp positions are often reused for this). Strong candidate for the two Clients even though terminal-native systems skip it.
- Selection (fg/bg) and cursor colour — named explicitly in every terminal scheme (Alacritty, Ghostty, Kitty, Omarchy) and in `gpui-component` (`selection`, `caret`); Helix and Zed have selection too. Strong candidate, and the one place the TUI's "terminal theme wins" approach (per CLAUDE.md) already gets this for free from the host terminal.
- 16 ANSI colours — the base16/24 core, all three terminal emulators, Omarchy, Zed's `terminal.ansi.*`, VS Code's `terminal.ansi.*` (not surveyed in depth above but present). This is the *only* role family every terminal-facing system agrees on by name and count (8 normal + 8 bright), which matters directly for the TUI given its "map colours to ANSI roles" strategy already in place.

**Common but not universal — good second-tier candidates:**

- `info` — named in `gpui-component`, `ratatui-themes`, Zed; absent from Helix, VS Code (no generic info role, just `editorInfo.*`), base16/24, terminal schemes.
- Hover / active / pressed state suffixes — Zed (`element.hover/.active`), `gpui-component` (`*_hover`/`*_active` per family), VS Code (`*.hoverBackground`/`*.activeBackground` pattern), Helix (`ui.menu.selected`). This recurs as a *pattern* (state suffix on an existing role) rather than a standalone role — the baseline decision is whether to bake `_hover`/`_active` suffixes onto every interactive role (gpui-component's/VS Code's approach) or keep it minimal.
- Focus ring — named explicitly only in `gpui-component` (`ring`) and VS Code (`focusBorder`) and Zed (`border.focused`); absent from every terminal-scheme and base16/24 system (terminals don't have a focus concept in the same sense). Worth keeping as its own role given both Clients are GUI/TUI apps with real focus traversal.
- Chart/series colours — `gpui-component` (`chart_1..chart_5`, 5), no other system surveyed has a dedicated *named* chart-series role family (Zed/VS Code/Helix have none; base16/24 templates sometimes reuse the 16 raw hues for charts by convention, which is effectively what `ratatui-themes`' consuming apps would have to do too). Since this repo's map explicitly puts chart/series colours in scope, `gpui-component`'s existing `chart_1..chart_N` pattern (already a live dependency) is the natural anchor rather than inventing a new shape.
- Danger vs "negative" wording — worth flagging as a naming decision, not just a role: `gpui-component` says `danger`, most others say `error`/`negative`. CLAUDE.md's ticket text uses "positive/negative", so that pairing is likely the house preference already.

**System-specific, not recurring enough to bake into a baseline:**

- Collaboration/`players[]` (Zed only — this repo has no multi-cursor collaboration feature).
- VCS-specific roles (`created`/`modified`/`renamed`/`conflict`/`ignored` — Zed and VS Code only, driven by their git integration, not a generic theming need).
- Ramped/positional neutrals (`surface0-2`/`overlay0-2`, Catppuccin only as an explicit *n*-step ramp) — useful as an *implementation technique* for deriving `muted`/`border`/hover shades from a small role set, but not itself a role name worth exposing.
- Dozens of VS Code per-component roles (`activityBar.*`, `statusBar.*`, etc.) — these are VS Code's own chrome, not portable Colour Role concepts; the *pattern* (`{component}.{state}`) is more useful to borrow than any individual key.

**Suggested starting Colour Role set** (union of the "near-universal" and "common" tiers above, intersected down to what recurs at least 3 times independently, deduplicated by concept rather than by source's exact name):

`background`, `foreground`, `muted` (background) / `muted-foreground` (text), `accent` / `accent-foreground`, `border`, `positive`, `negative`, `warning`, `info`, `selection` (background) / `selection-foreground`, `cursor`, `focus-ring`, `hover`/`active` as state suffixes on the interactive roles above rather than standalone roles, chart series `chart-1`..`chart-N` (N to be decided against `gpui-component`'s existing 5), and the 16 ANSI slots for terminal/TUI fidelity where the TUI needs to speak the terminal's own language rather than (or alongside) the Desktop's semantic roles.

This is offered as raw survey output for the decision ticket to narrow, rename to house vocabulary and confirm against `gpui-component`'s already-live `ThemeColor` shape (since the Desktop Client is a fixed dependency on it) and the TUI's "terminal theme wins" ANSI-mapping approach (since the TUI can't invent new terminal capabilities) — not as a final recommendation.

## Sources

- base16 styling guide: <https://github.com/tinted-theming/home/blob/main/styling.md>, builder spec: <https://github.com/tinted-theming/home/blob/main/builder.md>
- base24 styling guide: <https://github.com/tinted-theming/base24/blob/main/styling.md>
- Alacritty config reference: <https://alacritty.org/config-alacritty.html>
- Ghostty config reference: <https://ghostty.org/docs/config/reference>, theme feature doc: <https://ghostty.org/docs/features/theme>
- Kitty configuration docs (color table): manpage/docs at <https://sw.kovidgoyal.net/kitty/> (color-stack: <https://sw.kovidgoyal.net/kitty/color-stack/>)
- Omarchy manual, "Making your own theme": <https://omarchy.org/manual/making-your-own-theme/>; example theme file: <https://github.com/basecamp/omarchy/blob/dev/themes/nord/colors.toml>
- Helix themes documentation: <https://docs.helix-editor.com/themes.html>
- Zed themes documentation: <https://zed.dev/docs/themes>; theme JSON schema: <https://zed.dev/schema/themes/v0.2.0.json>
- VS Code Theme Color Reference: <https://code.visualstudio.com/api/references/theme-color>; Color Theme extension guide: <https://code.visualstudio.com/api/extension-guides/color-theme>
- `gpui-component` 0.5.1 (vendored dependency, pinned in this workspace's root `Cargo.toml`): `src/theme/theme_color.rs`, `src/theme/mod.rs`, `src/theme/default-theme.json`
- `catppuccin` crate docs: <https://docs.rs/catppuccin/latest/catppuccin/>; official palette page: <https://catppuccin.com/palette/>
- `ratatui-themes` crate docs: <https://docs.rs/ratatui-themes/latest/ratatui_themes/>
- ratatui core `Color`/`Style`: <https://docs.rs/ratatui/latest/ratatui/style/index.html>
