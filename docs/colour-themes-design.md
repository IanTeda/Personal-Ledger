# Colour Themes design

The developer-facing design for Colour Themes in the Desktop and TUI Clients. This page records the decisions reached on the [Colour Themes for the Desktop and TUI](https://github.com/IanTeda/Personal-Ledger/issues/292) Wayfinder map; each decision's detail lives on its ticket. The terms Colour Theme, Colour Role, Colour Variant and Colour Appearance are defined in `CONTEXT.md`.

## Status

In design. The Colour Role set is decided ([#298](https://github.com/IanTeda/Personal-Ledger/issues/298), [ADR-0022](adr/0022-seven-stored-colour-roles-with-calculated-shades.md)). Still open on the map: the TUI's terminal-theme vs Colour-Theme approach ([#299](https://github.com/IanTeda/Personal-Ledger/issues/299)), Preference storage and `[theme]` precedence ([#300](https://github.com/IanTeda/Personal-Ledger/issues/300)), the built-in Colour Themes ([#301](https://github.com/IanTeda/Personal-Ledger/issues/301)), the code architecture ([#302](https://github.com/IanTeda/Personal-Ledger/issues/302)) and the Settings controls ([#303](https://github.com/IanTeda/Personal-Ledger/issues/303)). Nothing here is built yet.

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

How `[theme]` layers against the synced Preference, and whether an override applies to one Colour Variant or both, is settled on [#300](https://github.com/IanTeda/Personal-Ledger/issues/300).

### Calculated colours

Every other colour is calculated from the seven stored roles by fixed rules in code, with the proportions as named constants. Calculated colours are not `[theme]` keys and cannot be overridden or set by a Colour Theme. Because each is a proportion between two stored roles rather than a fixed value, a Dark Colour Variant gets matching shades for free.

| calculated colour | rule | replaces today's |
| --- | --- | --- |
| Borders, hairlines, dividers, structural rule, hover tint, dialog scrim | `foreground` over `background` at fixed transparencies (e.g. hover 6%, border 30%, structural rule 38%) | `BORDER`, `HAIRLINE`, `HAIRLINE_LIGHT`, `DIVIDER`, `RAIL_DIVIDER`, `STRUCTURAL_RULE`, `HOVER_TINT`, `DIMMER` |
| Drop shadows | `foreground` at fixed transparency | `PALETTE_SHADOW`, `DIALOG_SHADOW` |
| Chrome (panel bands, status line, disabled fields, inset track) | `background` mixed a small step toward `foreground` | `CHROME`, `INSET_TRACK` |
| Faint text tier (placeholders, jump keys, meta) | halfway between `muted` and `background` | `INK_TERTIARY` |
| Selection (current/selected row) | swap: background is `foreground`, text is `background`; muted text on it calculated the same way | `INK` as a fill, `INK_ON_DARK*`, and the Desktop's other two selection styles |
| Text shades of `accent`, `positive`, `negative` | the role, darkened or lightened until it reaches 4.5:1 against the surface it sits on (`background`, or `foreground` on a selected row) | `ACCENT_TEXT`, `ACCENT_ON_DARK` |
| Accent tint background (the "base" flag pill) | `accent` at low opacity over `background`, text as an accent text shade | `TAG_ACCENT_BG`, `TAG_ACCENT_TEXT` |
| Info toast | chrome background, `foreground` text, `muted` border, a thin `foreground` bar on the leading edge | — (new) |
| Chart series | series 1 is `accent`; series 2–5 step from `foreground` toward `background` (e.g. 100%, 70%, 45%, 25%); a 6th or later repeats the pattern | the Desktop donut's hand-picked constants, the TUI pie's `Color::Rgb` slices |
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

Every role the TUI uses has a documented fallback to terminal colours, whichever way [#299](https://github.com/IanTeda/Personal-Ledger/issues/299) decides. The 16 ANSI colours are not Colour Roles themselves; they belong to the terminal side of that decision.

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

## Open items

- Everything on the tickets listed under [Status](#status).
- Rewriting the Modernist handoffs' "introduce no values outside this set" rule and the TUI README's Style table to speak in Colour Roles.
