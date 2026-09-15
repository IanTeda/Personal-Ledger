# Desktop Shell & Navigation

This is the gpui-facing spec for the Personal Ledger desktop client's application shell: window chrome, the two navigation rails, the command palette, the status line, and the keyboard model binding them together — the component tree, state machine, keybindings, and acceptance criteria the `docs/ux/desktop/Shell & Navigation/` bundle asks this document to carry. That bundle (`Ledger Desktop Shell.dc.html` + its own `README.md`) is the original Claude Design handoff — high-fidelity, HTML/CSS-shaped, and treated as the picture of the spec rather than the spec itself; this document is what actually gets built, in `gpui` terms, against `crates/bins/bin-desktop`.

`docs/ux/desktop/Settings/` is a second, companion handoff bundle — the `Noun::Settings` view's own ten-pane surface and its four modals — sharing this shell's design tokens and grammar but not yet translated into a `gpui`-terms living spec the way this document translates Shell & Navigation; treat its `README.md` as the current source of truth for that view until it is.

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

1. `noun` is the single source of truth for the context rail's contents; changing it resets `context` to the noun's first entity (`None` only for `Settings` — see "Where this differs from the handoff" below) and moves focus to `View`.
2. Changing `context` never changes `noun` and never moves focus.
3. `focus` cycles `PrimaryRail → ContextRail → View → PrimaryRail`, skipping empty zones.
4. A noun with no entities renders no context rail at all — never an empty rail.
5. Collapsing the primary rail (`RailMode`) doesn't change focus or selection.
6. Mode transitions are global and pre-empt zone key handling; `Esc` returns to `Normal` and restores the pre-mode focus zone.

`noun`, `primary_rail`, and window geometry survive restart; `context`, `focus`, and `mode` don't — every launch starts in `Normal` with focus in `View`. Persistence is a dedicated JSON file (`src/persistence.rs`) under `dirs::state_dir()` (Linux) or `dirs::data_local_dir()` (macOS/Windows, which have no separate state concept) — not `lib_config`'s layered INI config, which is for static settings, not transient UI state. A missing or corrupt file resolves to the default state rather than a startup error.

**Primary rail highlight vs. selection**, an implementation field the handoff's own struct doesn't name: `set_noun` (rule 1) unconditionally moves focus to `View`, which would make repeated `j`/`k`/`gg`/`G` browsing on the primary rail impossible if movement called `set_noun` directly (the first keypress would bounce focus away before a second one could land). `NavState` carries a `primary_highlight: Noun` field alongside `noun` — movement updates only `primary_highlight`; `Enter` (`commit_primary_highlight`) is the sole path that promotes it into `noun`. `primary_highlight` stays equal to `noun` at every other time (any `set_noun` call, or whenever focus (re-)enters `PrimaryRail`), so the primary rail's own rendering can use it unconditionally as "the row to draw selected."

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
- Archivo is bundled and loaded offline at startup (`include_bytes!` + `cx.text_system().add_fonts`) — the client is local-first and the handoff's own Google Fonts load is explicitly called out as a mockup-only convenience. Google Fonts only ships Archivo as a variable font now, so the 400/600/800 weights are static instances produced with `fonttools varLib.instancer`.
- Icon sourcing: `gpui-component`'s `Icon` element ships no SVGs of its own (confirmed from its own README once its source was checked). Every icon this handoff names is bundled as a Lucide SVG under `crates/bins/bin-desktop/assets/icons/` (stroke normalized to 1.5 per the handoff) and served through the app's own `gpui::AssetSource` (`src/assets.rs`) — `align-justify` (Transactions) had to be pulled from an older Lucide release, since current Lucide dropped that exact icon name.
- Letter-spacing (`.11em` on section kickers, `-.02em`/`-.01em` elsewhere) isn't rendered anywhere in the shell: `gpui` 0.2.2's `Styled` trait has no letter-spacing property. Everything else in the Design Tokens table is exact.
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
- **`Dashboard` does have context entities.** The handoff's own state-machine section (rule 1's parenthetical, and its "nouns with none" list) names `Dashboard` alongside `Settings` as having no context entities — but its "Context rail" component spec ("On Dashboard it shows accounts") and its own accepted 1a mockup both show Dashboard's context rail populated with a real, load-bearing seven-account roll-call, not an absent rail. That's a genuine inconsistency inside the handoff itself. Built against the concrete, unambiguous mockup: only `Settings` has `Noun::has_context_entities() == false`; `Dashboard`'s "first entity" (rule 1) is the first account, same shape as any other noun with entities.
- Icon sourcing and Archivo bundling are both resolved, noted above.

## Current implementation status

The static chrome is real and running: `Shell` (`src/shell.rs`) assembles a fixed-pixel `TopBar`, `PrimaryRail` (all ten nouns, grouped, jump-key column, Reconcile badge), a `ContextRail` (Dashboard's own account roll-call is real; every other noun with entities gets a placeholder frame), the Dashboard `View` interior (frame, figures, `LineChart` net-worth chart, hand-rolled in/out bars, `PieChart` donut, budget tracks, needs-attention block — all dummy content per the fidelity note), and a `StatusLine`, all driven by a live `NavState` (`src/nav.rs`) that persists `noun`/`primary_rail`/window geometry across restart (`src/persistence.rs`). `feasibility_demo.rs` (the old `TabBar` chart demo) is kept compiling but disconnected, per ADR-0016.

`Tab`/`Shift-Tab` focus-zone cycling and `j`/`k`/`Down`/`Up`/`gg`/`G`/`Ctrl-d`/`Ctrl-u`/`Enter` movement are real too, scoped strictly to whichever zone `NavState::focus` names — acceptance criteria 2 and 3 are met. The focused zone's 2px ink inner-edge indicator is real on all three zones. `Shell` holds a single `gpui::FocusHandle` for the whole window (the three zones are our own conceptual state, not `gpui`'s native focus system) and dispatches keys via a plain `on_key_down` listener, not the `actions!`/keymap system — a stateful chord parser with a timeout doesn't fit that system's assumptions well.

The `g`-prefix jump grammar (`g d`, `g t`, `g a`, `g b`, `g r`, `g c`, `g p`, `g u`, `g s`) is real, with its 1000ms pending-prefix timeout, an unbound completion flashing the status line's hint strip (reusing the same "hint strip replaced until the next keypress" mechanism the handoff specifies for write failures), and mode transitions (`:` `Command`, `/` `Search`, bare `a` `Insert`) are real too, pre-empting zone/movement handling while active. `Esc` clears a pending `g` first, else leaves the current mode and restores the pre-mode focus zone (rule 6) -- acceptance criterion 5 is met for every path this map has built so far. A pending `g` is checked before the mode-entry keys, so `g a` (jump to Accounts) and bare `a` (enter `Insert`) never collide.

Not yet built: `b`'s rail toggle, `?`'s help overlay, the command palette, the collapsed rail, and every other noun's real view interior. Most are separate tickets still open on the map linked at the top of this document -- `?`'s help overlay currently has no ticket on the map at all (see the map's "Not yet specified").
