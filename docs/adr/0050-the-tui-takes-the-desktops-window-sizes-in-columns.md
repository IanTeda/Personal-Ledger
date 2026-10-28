# The TUI takes the Desktop's Window Sizes, in columns

[ADR-0042](0042-window-size-from-window-width-with-a-medium-floor.md) gave the Desktop Client three Window Sizes picked from the window's width. The TUI had one layout at every width, with an ad hoc order for degrading below 96 columns (`docs/ux/tui-mockups/01-shell/README.md`), so a terminal as wide as a Large window still got the Medium drill-down, and the two Clients' designs could not be drawn or compared size for size. Decided while drawing the TUI Settings Window Size cards (TUI-049–057).

## Decision

- The TUI uses the Desktop's three Window Sizes and Breakpoints, read in terminal columns at **7.5 logical px per column**:

| Window Size | Desktop width | Columns | Reference frame for mockups (cells) |
| --- | --- | --- | --- |
| Medium | 720 – 1199 | 96 – 159 | 128 × 69 (the Desktop's 960 × 1040) |
| Large | 1200 – 1599 | 160 – 213 | 171 × 53 (1280 × 800) |
| X-Large | ≥ 1600 | ≥ 214 | 256 × 72 (1920 × 1080) |

- The 7.5px column is a fixed design scale, not a measurement of the user's font. It is chosen so the TUI's existing 96-column minimum is exactly the Desktop's 720px floor, and a mockup card drawn at 7.5 × 15px a cell is as wide as its Desktop counterpart.
- Width alone picks the Window Size, from `frame.area().width` on every draw. Height never picks one. Nothing stores it: crossterm's resize event already triggers a redraw, which reads the new width.
- The floor is **96 columns × 24 rows**. A terminal cannot be held to a minimum size, so below it the Shell draws a "terminal too small" screen naming the current and the minimum size, in place of the View. This replaces the old degrade order: no View has a layout narrower than Medium.
- A View decides its own layout per Window Size, following its Desktop design: at Large the Settings index docks beside the page, and at X-Large a View may dock a detail pane. Page content caps at 112 columns (the Desktop's 840px) at every size.
- `WindowSize` and its Breakpoints move to `lib-shell`, toolkit-free, with one constructor from logical px and one from columns, so both Clients read one definition and cannot drift. The Desktop keeps its gpui adapter (`WindowSize::of(window)`) and its window-only constants (`MIN_WINDOW`, overlay widths) in the bin.

## Consequences

- `GLOSSARY.md`'s **Window Size** covers both Clients' widths.
- `bin-tui` takes a dependency on `lib-shell`, which #805 plans for the TUI's move onto VIPER anyway; `deny.toml` already allows any bin to depend on it.
- TUI View render tests use `TestBackend` at each reference frame and at the 96 × 24 floor.
- TUI mockups draw a View at the three reference frames as it is built or redrawn, starting with Settings (`docs/ux/tui-mockups/05-settings/`, turn 5).
- A user with a large terminal font gets a smaller Window Size than the same terminal in px would on the Desktop. That is wanted: what fits is decided by columns, not pixels.

## Considered Options

- **Read the terminal's pixel width** (`crossterm::terminal::window_size()`) and apply the px Breakpoints directly. Rejected: many terminals report 0, others report physical rather than logical pixels under display scaling, and the layout has to fit columns, not pixels.
- **Breakpoints chosen for terminals** (80 / 120 / 180 columns). Rejected: the TUI's designs follow the Desktop's per Window Size, and a separate set of widths would make a "Large" mean different things in the two Clients.
- **Keep one layout at every width** and degrade below 96 columns. Rejected: it wastes a wide terminal and leaves every View with an untested set of degraded layouts.
- **A separate `TerminalSize` type in `bin-tui`.** Rejected: it would copy the Breakpoints and could drift from the Desktop's.
