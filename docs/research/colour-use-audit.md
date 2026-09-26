# Colour use audit: Desktop and TUI

An inventory of every colour the Clients use as of `concept` at `4329e38`, for [#294](https://github.com/IanTeda/Personal-Ledger/issues/294) on the [Colour Themes](https://github.com/IanTeda/Personal-Ledger/issues/292) map. Each use is mapped to one of the seven stored Colour Roles decided in [#298](https://github.com/IanTeda/Personal-Ledger/issues/298) (`foreground`, `background`, `accent`, `cursor`, `muted`, `positive`, `negative`) or to a colour calculated from them (see `docs/colour-themes-design.md`). The audit landed after the role decision, so it checks that decision and feeds the slicing of the implementation sweep rather than informing the role set itself.

Counts are `grep` hits outside `theme.rs` (Desktop) and across `bin-tui/src` (TUI), tests included, so treat them as proportions rather than exact call sites.

## Desktop (`bin-desktop`)

### `theme::color` constants

| Constant | Value | Uses | Purpose today | Maps to |
| --- | --- | --- | --- | --- |
| `GROUND` | `#f3f2f2` | 21 | Page/view background, dialog body | `background` |
| `CHROME` | `#eae9e9` | 35 | Topbar, statusline, footers, table header band | calculated: chrome shade of `background` |
| `INSET_TRACK` | `#e2e0df` | 2 | Progress/scroll track (dashboard, explorer) | calculated: inset shade of `background` |
| `INK` | `#201e1d` | 112 | Primary text, icons, selected-row fill | `foreground` |
| `INK_SECONDARY` | `#605d5d` | 55 | Secondary text, labels | calculated: text shade of `foreground` |
| `INK_TERTIARY` | `#9b9797` | 64 | Hints, kickers, disabled text | `muted` |
| `INK_ON_DARK` | `#f3f2f2` | 20 | Text on an inverted (selected) row or accent fill | calculated: `background` on `foreground`/`accent` |
| `INK_ON_DARK_SECONDARY` | `#bab6b6` | 4 | Secondary text on a selected row | calculated: selected-row muted (contrast floor) |
| `INK_ON_DARK_TERTIARY` | `#d7d3d3` | 3 | Tertiary text on a selected row | calculated: selected-row muted (contrast floor) |
| `HAIRLINE` | `#d7d3d3` | 24 | Row separators in rails and lists | calculated: border shade |
| `HAIRLINE_LIGHT` | `#eae9e9` | 2 | Table rules | calculated: border shade (lighter) |
| `STRUCTURAL_RULE` | `foreground` @ .38 | 17 | 2px band/column rules | calculated: `foreground` alpha |
| `BORDER` | `foreground` @ .30 | 49 | Unfocused input/button border | calculated: `foreground` alpha |
| `DIMMER` | = `BORDER` | 1 | Settings dialog scrim | calculated: scrim |
| `RAIL_DIVIDER` | `foreground` @ .20 | 2 | In-rail group divider | calculated: `foreground` alpha |
| `HOVER_TINT` | `foreground` @ .06 | 12 | Row/result hover | calculated: hover |
| `DIVIDER` | `foreground` @ .40 | 7 | Segmented-control borders | calculated: `foreground` alpha |
| `ACCENT` | `#ec3013` | 41 | Primary action, focus ring, mode badge, segmented selection, over-budget, caret | `accent` (caret sites → `cursor`, over-budget → `negative`) |
| `ACCENT_ON_DARK` | `#ff9783` | 2 | Matched substring on a selected row | calculated: accent contrast-corrected on `foreground` |
| `ACCENT_TEXT` | `#ae1800` | 11 | Accent text at ≤13px (contrast) | calculated: accent text shade (contrast floor) |
| `PALETTE_SHADOW` | `#2d2b2b` @ .30 | 2 | Command palette shadow | calculated: shadow |
| `DIALOG_SHADOW` | `foreground` @ .40 | 3 | File explorer / dialog shadow | calculated: shadow |
| `POSITIVE` | `#2ecc71` | 2 | Sync server "connected" dot/text | `positive` |
| `TAG_ACCENT_BG` | `#fff2ef` | 3 | Base-unit flag pill fill | calculated: accent tint |
| `TAG_ACCENT_TEXT` | `#7c1405` | 2 | Base-unit flag pill text | calculated: accent deep shade |

Every Desktop colour is covered by the seven roles plus calculated colours; nothing needs an eighth stored role.

### Outside `theme.rs`

- `explorer.rs:354` and `explorer.rs:547`: inline `gpui::rgba(0x201e1d4d)`, the same value as `BORDER`. These should use `color::BORDER` (or its resolved equivalent) now.
- `feasibility_demo.rs:407`: a `HexColor` for demo data, not UI chrome. Out of scope.

### `gpui-component` `ActiveTheme` reads

These are all in `feasibility_demo.rs` (charts and table): `success`, `danger`, `chart_1`..`chart_5`, `border`, `muted_foreground`, `background`, `foreground`. ADR-0025 feeds `gpui_component::Theme` from `ResolvedColours`, so these map as `success` → `positive`, `danger` → `negative`, `chart_n` → calculated chart series, `border` → calculated border, `muted_foreground` → `muted`.

## TUI (`bin-tui`)

The TUI mostly styles with modifiers and leaves colour to the terminal. Colours in use:

| Use | Count | Where | Purpose today | Maps to |
| --- | --- | --- | --- | --- |
| `const ACCENT: Color = Color::Red` | 25 files, ~85 uses | every `view/` and most `popup/` modules | See the breakdown below | split: `accent` / `negative` / `cursor` |
| `Modifier::DIM` | 129 | nearly every view/popup (plus 11 local `fn dim()` helpers) | Labels, hints, secondary detail | `muted` |
| `Modifier::REVERSED` | 25 | list selection in every view, key chips, command-popup selection | Selected row / focused choice | calculated: selection (`background` on `foreground`) |
| `Modifier::BOLD` | 38 | headings, titles, key figures | Emphasis (weight, not colour) | stays a modifier |
| `Color::DarkGray` | 4 | `shell.rs:1684` footer, `popup/command` `DIM`, dashboard chart slice | Dim text in the footer and command popup | `muted` |
| `Color::Rgb(90,90,90)` | 1 | `popup/command` `FOOTER_LABEL` | Footer hint labels (added because `DarkGray` is remapped by terminal themes) | `muted` (calculated shade) |
| `Color::Yellow` | 1 | `popup/command` `MATCH_HIGHLIGHT` | Matched substring in the command popup | `accent` (matches the Desktop's matched-substring rule) |
| `Color::Rgb(139,0,0)`, `Gray`, `DarkGray`, `Rgb(211,211,211)` + `ACCENT` | 5 | `view/dashboard.rs` `fake_spending` | Spending chart slices | calculated: chart series |
| `Color::Gray` | 1 | `view/dashboard.rs:854` | Budget bar fill when not over budget | `muted` (over-budget arm → `negative`) |
| `Color::Reset` / `Color::Red` | 2 | tests (`shell.rs:2127`, `dashboard.rs:1327`) | Assertions on rendered colour | follow whatever the resolved `Colours` hold |

### What `ACCENT` (red) means in the TUI

Per the doc comments on each `const ACCENT`, it covers three different things:

- **Negative values** (`negative`): negative balances (`accounts`), negative totals (`payees`), negative weekly Δ% (`units`), over-budget, variance and Liabilities (`dashboard`).
- **Input cursor** (`cursor`): `popup/unit/{new,edit,delete}`, `popup/category/move_popup`, `popup/settings/edit`, and the account/tag/payee/category forms.
- **Emphasis and warnings** (`accent`): the override dot and "N overridden" figure and consequence warnings (`settings`), the delete-confirm line (`tags`), the permanence and precision warnings (`popup/unit`), the chart's marked last point (`categories`), the `preview` figure (`popup/settings/edit`), the conflict flag (`payees`), and the moving node's landing row (`move_popup`).

The Desktop has the same overlap: `ACCENT` covers both over-budget (`dashboard.rs`) and the block caret (`palette.rs:346`, `statusline.rs:235`). Separating these into `accent`, `cursor` and `negative` is most of the migration work. The TUI's current `README.md` rule, one accent reserved for negatives, is a special case of `negative`.

## Findings for the implementation sweep

1. **The role set holds up.** Every current use maps onto the seven roles or a calculated colour, and nothing in use calls for an eighth stored role. Warnings/destructive confirms were the only close call: today they use `accent`, and they could reasonably use `negative`. That is a design call for the sweep tickets, not a new role.
2. **The TUI needs `ACCENT` split three ways.** The 25 identical `const ACCENT` definitions collapse into the `Colours` set on `Shell` (ADR-0025). Each use site has to pick `negative`, `cursor` or `accent`, using the doc comment above each const as the guide.
3. **The TUI's `DIM` and `REVERSED` become colours.** With RGB drawing on by default (ADR-0024), `Modifier::DIM` (129 uses) should become the `muted` role colour, and `REVERSED` selection (25) the calculated selection pair. In terminal-colours mode they can stay modifiers. Together these make up most of the "~340 sites", so the sweep is best sliced per view/popup module, not per role.
4. **Duplicated per-module helpers** (11 `fn dim()`, 25 `const ACCENT`, `popup/command`'s `DIM`/`FOOTER_LABEL`/`MATCH_HIGHLIGHT`) are removed in the first TUI ticket that introduces `Colours`.
5. **Desktop cleanups before the sweep:** replace the two inline `rgba(0x201e1d4d)` in `explorer.rs` with `color::BORDER`, and fold `DIMMER` (an alias) and `HAIRLINE_LIGHT` (equal to `CHROME`) into the calculated set rather than carrying them over as separate names.
6. **Chart colours** (TUI `fake_spending`, Desktop `chart_1..5`) are the only multi-hue uses, and both become the calculated chart series.
