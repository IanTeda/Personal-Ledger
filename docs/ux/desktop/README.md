# Desktop Shell & Navigation

This is the gpui-facing spec for the Personal Ledger desktop client's application shell: window chrome, the two navigation rails, the command palette, the status line, and the keyboard model binding them together — the component tree, state machine, keybindings, and acceptance criteria the `docs/ux/desktop/Shell & Navigation/` bundle asks this document to carry. That bundle (`Ledger Desktop Shell.dc.html` + its own `README.md`) is the original Claude Design handoff — high-fidelity, HTML/CSS-shaped, and treated as the picture of the spec rather than the spec itself; this document is what actually gets built, in `gpui` terms, against `crates/bins/bin-desktop`.

The desktop and TUI clients are visual siblings by deliberate design (see the handoff's own "governing constraint"): both speak `g`-jumps, a `:` command palette, and a modal status line, so a user moving between them re-uses muscle memory. Where the two disagree, `docs/ux/tui/navigation.md` — the TUI's own living reference, playing the same role this document plays here — is the older, decided contract; read it before implementing anything keyboard-shaped.

The full build-out trail for this document lives on the [Desktop Shell & Navigation](https://github.com/IanTeda/Personal-Ledger/issues/144) Wayfinder map and its children (issues #145–#154).

## Philosophy

The TUI's own navigation model (`docs/ux/tui/navigation.md`) makes a deliberate zero-footprint-at-rest bet: no persistent chrome, everything reached through one floating command popup, because terminal real estate is scarce and precious. The desktop shell makes the opposite trade in the same currency: screen space is comparatively abundant, so Option 1a keeps two persistent rails — a primary rail of nouns and a context rail scoped to whichever noun is active — visible at all times, trading a small fixed footprint for at-a-glance orientation a terminal can't afford. What doesn't change is the underlying grammar: the same `g`-jump chords, the same `:` command palette (now a floating overlay rather than the only navigation surface), and the same modal status line naming whatever mode isn't `Normal`. The rails are the desktop's own addition on top of that shared grammar, not a replacement for it — every noun the rails expose is still reachable by jump and by palette, per acceptance criterion 1.

## Component tree

```text
Shell
├── TopBar            — rail toggle, brand mark + open file name, palette hint, sync indicator
├── content row
│   ├── PrimaryRail    — Expanded (206px, grouped rows) | Collapsed (52px, icon-only, tooltip on hover)
│   ├── ContextRail     — Option<_>, scoped to the active Noun; None renders no rail at all (rule 4)
│   └── View            — the active Noun's own interior; owns its own scrolling
├── StatusLine         — mode badge, hint strip / error, breadcrumb
└── CommandPalette      — Option<_>, floating overlay over the dimmed shell (mode == Command)
```

Each of `TopBar`, `PrimaryRail`, `ContextRail`, `StatusLine`, and `CommandPalette` is its own module under `crates/bins/bin-desktop/src/` (`topbar.rs`, `rail/primary.rs`, `rail/context.rs`, `statusline.rs`, `palette.rs`), rendered by the root `Shell` `Render` impl from a single `NavState`, mirroring the TUI's own `Shell`/`View` split (ADR-0013) rather than inventing a second navigation shape.

## State machine

Lifted directly from the handoff, since it's already stated in Rust and needs no HTML-to-gpui translation:

```rust
struct NavState {
    noun: Noun,                  // active rail item
    context: ContextSelection,   // active entity within the noun, if any
    focus: FocusZone,
    primary_rail: RailMode,      // Expanded | Collapsed
    mode: InputMode,             // Normal | Insert | Command | Search
}

enum Noun { Dashboard, Transactions, Accounts, Reconcile,
            Budgets, Reports, Categories, Payees, Units, Settings }

enum FocusZone { PrimaryRail, ContextRail, View }
```

Transition rules, restated from the handoff's numbered list:

1. `noun` is the single source of truth for the context rail's contents; changing it resets `context` to the noun's first entity (`None` for Dashboard/Settings) and moves focus to `View`.
2. Changing `context` never changes `noun` and never moves focus.
3. `focus` cycles `PrimaryRail → ContextRail → View → PrimaryRail`, skipping empty zones.
4. A noun with no entities renders no context rail at all — never an empty rail.
5. Collapsing the primary rail (`RailMode`) doesn't change focus or selection.
6. Mode transitions are global and pre-empt zone key handling; `Esc` returns to `Normal` and restores the pre-mode focus zone.

`noun`, `primary_rail`, and window geometry survive restart; `context`, `focus`, and `mode` don't — every launch starts in `Normal` with focus in `View`. The mechanism for that persistence (a small dedicated state file vs. extending `lib_config`) isn't decided yet — `lib_config`'s layered INI config is for static settings, not transient UI state, so it's likely the wrong home; see "Where this differs from the handoff" below.

## Keybindings

Checked in this order, a key that means something at a higher tier always wins:

1. Mode transitions are global and pre-empt everything below: `:` opens the command palette (`Command` mode), `/` searches the active view (`Search` mode), `a` opens the transaction-entry dialog (`Insert` mode), `?` opens the help overlay, `Esc` leaves the current mode, dismisses an overlay, or clears a pending `g` prefix (in that precedence).
2. `g` then a letter jumps directly to a noun: `g d` Dashboard, `g t` Transactions, `g a` Accounts, `g b` Budgets, `g r` Reports, `g c` Categories, `g p` Payees, `g u` Units, `g s` Settings. Reconcile is deliberately unbound (it's a task, not a place) — reach it by rail click or `:reconcile`. An unbound completion is a silent no-op that clears the pending prefix and flashes the hint strip. The pending-prefix window is 1000ms.
3. `b` toggles the primary rail between Expanded and Collapsed, preserving focus and selection either way.
4. `Tab` / `Shift-Tab` cycle `FocusZone` forward/back, skipping empty zones.
5. Within the focused zone: `j`/`k` or `Down`/`Up` move one row, `gg`/`G` jump to first/last, `Ctrl-d`/`Ctrl-u` move a half-page, `Enter` activates (on the primary rail this also moves focus to `View`; on the context rail it doesn't move focus).

No `Ctrl` chords beyond what's listed above — divergence from the TUI's own grammar is the failure mode this shell is built to avoid. GNOME-owned chords (`Ctrl-w`, `Ctrl-q`, `F10`) stay with the window manager, not the app.

## Command palette and registry

The palette is driven by a single command registry (`src/command.rs`), not a hand-written list — every rail item, every context-rail footer affordance (e.g. "+ new account · `:account new`"), and every future view action registers a `Command { name, description, kind, binding, handler }`. This is the same shape as the TUI's own per-domain command registry (`crates/bins/bin-tui/src/popup/command/commands/`, documented in `docs/ux/tui/navigation.md`'s "Commands" section) — reuse that pattern rather than a bespoke one. Results rank across every command kind by substring match, not grouped by domain, so a query like `bud` can surface `:budget edit` alongside `:report variance` in one list. A command with no real behaviour yet should say so explicitly when run (mirroring the TUI's "not yet built" fallback), rather than silently doing nothing.

## Design tokens and assets

The handoff's Design Tokens and Assets tables (colors, type, spacing, radius, shadows, Lucide icon names) are the literal specification — "introduce no values outside this set" is a system rule, not a preference, so they're reproduced there rather than duplicated here. Translation notes specific to `gpui`:

- The palette is fixed, not theme-adaptive (no dark-mode variant in the handoff), so it's implemented as plain constants in `src/theme.rs` rather than plumbed through `gpui-component`'s own `ActiveTheme` — that theme system stays scoped to the chart/table widgets it already renders (`LineChart`, `PieChart`, `Table`) from the feasibility cycle.
- Archivo must be bundled as a font asset and loaded offline at startup — the client is local-first and the handoff's own Google Fonts load is explicitly called out as a mockup-only convenience.
- The Lucide icon set isn't used anywhere else in this repo yet, and `gpui-component` isn't vendored locally to check what it ships — sourcing them (an icon crate vs. bundled SVGs) is an open item, not yet resolved.
- The transaction status glyphs (`○ ◐ ● ⚑`) are typographic marks, not icons, and must render identically to the TUI's own (`docs/ux/tui/README.md`).

## Acceptance criteria

Restated from the handoff, unchanged — these are what "the shell is built" means:

1. Every noun is reachable by `g`-jump, rail click, and palette, and all three land in the same state.
2. `Tab` visits exactly the zones that have content, in rail → context → view order, and the focused zone is unambiguous without hovering.
3. `j`/`k` act only on the focused zone; no zone scrolls while another is focused.
4. `b` preserves focus and selection, and the collapsed-rail tooltip opens to the right, vertically centred, without clipping or overlapping the context rail at any window size at or above the 960×640 minimum.
5. `Esc` from any mode or overlay returns to `Normal` with the pre-mode focus restored.
6. Nouns without entities render no context rail; the view area reflows to fill.
7. Restart restores noun, rail mode, and window geometry — and nothing else.
8. Every amount is tabular-aligned, right-aligned in its column, and uses U+2212 for negatives.
9. No rounded corners, no shadows outside the palette and tooltip, no unthemed focus ring, no animated chrome.
10. The palette is registry-driven: adding a view action makes it appear in the palette with no palette-side code change.

## Where this differs from the handoff

- The handoff's own `docs/ux/desktop/Shell & Navigation/README.md` describes four explored options; only 1a (plus 1a's own collapsed-rail and command-palette states, 1c/1d) is in scope here — 1b is provenance only, kept for its transaction-table column frame and the "a noun's own actions live in the view header" convention, neither of which this document repeats since neither is part of the accepted shell chrome.
- Restart persistence for `noun`/`primary_rail`/window geometry has no chosen mechanism yet (see "State machine" above) — the handoff doesn't specify one either, since it's a static mockup.
- Icon sourcing (Lucide via which crate/pipeline) is unresolved, noted above.

## Current implementation status

Nothing in this document is built yet. `crates/bins/bin-desktop/src/main.rs` is still the FC-DESKTOP feasibility-cycle chart-demo (a `TabBar` over dummy Line/Doughnut/Candlestick/Divergent/Table/Live-Categories screens, closed out by the [Desktop App feasibility map](https://github.com/IanTeda/Personal-Ledger/issues/23)) — real shell/navigation work, deferred by that map's own "Out of scope" note, starts with the Wayfinder map linked at the top of this document. This section will track progress the way `docs/ux/tui/navigation.md`'s own "Current implementation status" section does, once the first piece of chrome lands.
