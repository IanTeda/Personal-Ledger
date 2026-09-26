# A Colour Theme stores seven Colour Roles and every other colour is calculated from them

Colour Themes (see Colour Theme, Colour Role, Colour Variant and Colour Appearance in `CONTEXT.md`) need a fixed set of Colour Roles that every Colour Theme defines and that a `[theme]` Configuration section can override on one Client. Today the Desktop has 25 fixed Modernist constants in `bin-desktop/src/theme.rs`, many of them alpha shades of one ink, and the TUI draws almost everything in one `Color::Red` accent plus `DIM` and `REVERSED`. In both Clients the red accent is overloaded: it marks negative amounts, focus, over-budget, validation errors, a chart series and brand marks at once.

We're storing exactly seven Colour Roles per Colour Variant, shared by both Clients: `foreground`, `background`, `accent`, `cursor`, `muted`, `positive` and `negative`. Their snake_case names are also the `[theme]` keys, with hex values. Every other colour is calculated from them by fixed rules in code:
- borders, hairlines, hover, the scrim and shadows are `foreground` over `background` at fixed transparencies
- chrome is a mix between `foreground` and `background`
- selection swaps `foreground` and `background`
- text shades of `accent`, `positive` and `negative` are corrected until they reach 4.5:1 contrast
- chart series step from `accent` through `foreground` toward `background`
- the info toast is built from chrome and `muted`

Calculated colours cannot be set by a Colour Theme or overridden in `[theme]`.

`negative` means only an amount. Warnings and validation errors use `accent`, keeping the "one accent" rule. The TUI's fallback is a documented terminal colour or text effect for every role (for example `muted` is `DIM`, selection is `REVERSED`), whichever way the TUI's terminal-theme question is decided. Contrast follows WCAG 2.x: 4.5:1 for text, 3:1 for non-text marks and 7:1 for text in the high-contrast Colour Theme, enforced by unit tests over every built-in Colour Theme. A failing `[theme]` override is still applied, with a `warn` in the log rather than a rejection. The full role and calculation tables are in `docs/colour-themes-design.md`.

## Considered Options

Storing every slot, as `gpui-component`'s ~90-field `ThemeColor` or today's 25 Modernist constants do, was rejected. It makes a Colour Theme, or a `[theme]` override, tedious to write. It lets hover, dividers and scrims drift out of step with the colours they are shades of. And the Modernist names (`rail_divider`, `ink_on_dark_tertiary`) would bake one design's layout into user-facing Configuration keys. Allowing calculated colours to be overridden was rejected for now because it lengthens the key list and makes the contrast guarantees uncheckable. Adding overrides later breaks nothing, whereas removing them would.

Separate `warning`, `error`, `info`, `selection` and chart-series roles were rejected as stored roles because each is either an alert (served by `accent`) or expressible as a calculation from the seven. A per-Client or prefixed role set was rejected because there is one Colour Theme and one `[theme]` section syncing across both Clients, and a TUI ignoring a few Desktop-only colours costs nothing.

Rejecting a `[theme]` override that fails contrast, and falling back to the Colour Theme's value, was rejected: it is the user's own Configuration on their own machine, and silently ignoring it would look like a bug. APCA (the WCAG 3 draft) was rejected in favour of the stable WCAG 2.x ratios that the existing `ACCENT_TEXT` already reasons in.
