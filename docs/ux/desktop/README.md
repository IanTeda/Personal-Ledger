# Desktop Shell & Navigation

This is the gpui-facing spec for the Personal Ledger desktop client's application shell: window chrome, the two navigation rails, the command palette, the status line, and the keyboard model binding them together — the component tree, state machine, keybindings, and acceptance criteria the `docs/ux/desktop/Shell & Navigation/` bundle asks this document to carry. That bundle (`Ledger Desktop Shell.dc.html` + its own `README.md`) is the original Claude Design handoff — high-fidelity, HTML/CSS-shaped, and treated as the picture of the spec rather than the spec itself; this document is what actually gets built, in `gpui` terms, against `crates/bins/bin-desktop`.

`docs/ux/desktop/Settings/` is a second, companion handoff bundle — the `Noun::Settings` view's own ten-pane surface and its four modals — sharing this shell's design tokens and grammar but not yet translated into a `gpui`-terms living spec the way this document translates Shell & Navigation; treat its `README.md` as the current source of truth for that view until it is.

The desktop and TUI clients are visual siblings by deliberate design (see the handoff's own "governing constraint"): both speak `g`-jumps, a `:` command palette, and a modal status line, so a user moving between them re-uses muscle memory. [`docs/navigation.md`](../../navigation.md) is now the cross-client layer that grammar lives in — read it first for anything keyboard-shaped. Where it doesn't cover something, `docs/ux/tui/navigation.md` — the TUI's own living reference, playing the same role this document plays here — is the older, decided contract to fall back to.

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

enum Noun { Dashboard, Transactions, Accounts, Categories, Payees,
            Tags, Bills, Budgets, Reports, Settings }

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
2. `g` then a letter jumps directly to a noun: `g d` Dashboard, `g l` Transactions, `g a` Accounts, `g c` Categories, `g p` Payees, `g t` Tags, `g w` Bills, `g b` Budgets, `g r` Reports, `g s` Settings. Transactions moved off `g t` onto `g l` to free `t` for the newer Tags noun (see "Where this differs from the handoff"). An unbound completion is a silent no-op that clears the pending prefix and flashes the hint strip. The pending-prefix window is 1000ms.
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
- **The noun set itself changed mid-map.** The handoff bundle was refreshed after #148–#151 had already landed against the original ten nouns: `Reconcile` and `Units` are dropped from the primary rail entirely, and `Tags` and `Bills` are added (`docs/ux/desktop/Shell & Navigation/README.md`'s "1a" now groups LEDGER — Dashboard, Transactions, Accounts, Categories, Payees, Tags — and PLAN — Bills, Budgets, Reports — with no separate RECORDS heading). `Transactions` moved off its own `g t` jump chord onto `g l` to free `t` for `Tags`. `Units`' old ground (currencies and tracked units) moves into the new companion `docs/ux/desktop/Settings/` handoff as one of its ten panes rather than staying a rail-level noun. This document, `src/nav.rs`, `src/command.rs`, `src/rail/primary.rs`, `src/statusline.rs`, and `src/icon.rs` are all built against the refreshed set; `crates/bins/bin-desktop/src/view/dashboard.rs`'s "needs attention" block still names `:reconcile` in its representative dummy copy, a leftover from before the rework that's harmless (it's static placeholder text, not a live command binding — the palette itself has no `reconcile` command any more) but worth cleaning up whenever that block's content is next touched.

## Current implementation status

Every ticket on the [Desktop Shell & Navigation](https://github.com/IanTeda/Personal-Ledger/issues/144) map except this one is closed. `Shell` (`src/shell.rs`) assembles a fixed-pixel `TopBar`, `PrimaryRail` (all ten nouns, both its expanded grouped-row and collapsed icon-only states), a `ContextRail` (Dashboard's own account roll-call is real; every other noun with entities gets a placeholder frame), the Dashboard `View` interior (frame, figures, `LineChart` net-worth chart, hand-rolled in/out bars, `PieChart` donut, budget tracks, needs-attention block — all dummy content per the fidelity note), a `StatusLine`, and a floating `Palette`, all driven by a live `NavState` (`src/nav.rs`) that persists `noun`/`primary_rail`/window geometry across restart (`src/persistence.rs`). `feasibility_demo.rs` (the old `TabBar` chart demo) is kept compiling but disconnected, per ADR-0016.

### Acceptance criteria walk

1. **Met.** Every noun in `Noun::ALL` has a `g`-jump chord (`jump_noun_for_key`, `src/shell.rs`), a rail row (both `PrimaryRail` layouts, `src/rail/primary.rs`), and a `Navigate` command (`command::COMMANDS`, enforced by its own `every_noun_has_a_navigate_command` test) — all three call `NavState::set_noun` directly (`Shell::handle_rail_click`, the pending-`g` arm of `Shell::handle_key_down`, and each `goto_*` handler respectively), so all three land in identical state, never a browse-then-commit step.
2. **Met.** `NavState::cycle_focus_forward`/`cycle_focus_backward` skip `ContextRail` whenever `Noun::has_context_entities()` is `false`, tested directly (`focus_skips_context_rail_when_noun_has_no_entities` and its backward counterpart). The focused zone draws a 2px ink inner-edge border (`PrimaryRail`/`ContextRail`/`render_view` each take a `focused: bool`) so it's identifiable without hovering.
3. **Met.** `Shell::apply_movement` matches on `NavState::focus()` and dispatches to exactly one of `apply_primary_rail_movement`/`apply_context_rail_movement`/`apply_view_movement` — the other two zones' state (browsed row, context index, scroll offset) is untouched by construction, not by a runtime guard.
4. **Met by construction, not by a live resize test.** `toggle_primary_rail` only flips `primary_rail` (`toggling_primary_rail_preserves_focus_and_context` asserts focus and context both survive it). The collapsed row's tooltip (`rail::primary::collapsed_tooltip`) anchors `left: ROW_BOX + 12px`, vertically centred via a full-height wrapper and `items_center`, and paints through `gpui::deferred` specifically so the context rail (its next sibling in paint order) can't clip it — the fix issue #152 called out. This sandbox's Hyprland setup has no working synthetic click/focus path for `bin-desktop` (see the map's own Notes), so the tooltip's on-screen behaviour at the 960×640 floor is verified by this reasoning and its test coverage, not by an actual resize-and-look pass; a real display's own resize testing would be worth doing opportunistically before Concept-cycle sign-off.
5. **Met for every path built on this map.** `Esc` clears a pending `g` first (`handle_key_down`); failing that, while any non-`Normal` mode is active it closes the palette and calls `NavState::exit_mode`, which restores `pre_mode_focus` (`exit_mode_restores_the_focus_zone_from_before_entry`, `entering_a_second_mode_does_not_overwrite_the_remembered_focus`). `Insert`/`Search` have no real input surface yet (no dialog, no search box), so today this only has `Command` (the palette) to actually exercise — the mechanism itself doesn't distinguish modes and needs no further work when those surfaces land.
6. **Met.** `Shell::render`'s `.when(self.nav.noun().has_context_entities(), ...)` is the only place a `ContextRail` gets inserted at all; the `View` div is `flex_1()` and fills whatever space is left whether or not that call fires.
7. **Met.** `persistence::PersistedState` carries exactly `noun`, `primary_rail`, and `window: Option<WindowGeometry>` (`src/persistence.rs`); `main.rs` seeds a fresh `NavState::new()` and then only calls `set_noun`/`set_primary_rail` from the loaded state, so `context`, `focus`, and `mode` always come from `NavState::default()` (`Normal`, focus in `View`) regardless of what was saved.
8. **Met for the rows built so far; the underlying font metric is assumed, not confirmed.** Negatives resolve to U+2212 wherever an amount can be negative (`rail::context::Account::balance`, the Dashboard's own `LIABILITIES` figure). `budget_row` (`view/dashboard.rs`) right-aligns its `spent / limit` text in a fixed `w(118px)` column, the one place this map builds a real multi-row numeric column; the account roll-call's own balances align visually via `justify_between` inside a shared-width row rather than an explicit column, which reads the same but isn't the same mechanism. Neither this map nor its handoff calls for a `tabular-nums`-equivalent font feature, and `gpui` 0.2.2's `Styled` trait has no such property to set (the same ceiling already noted for letter-spacing) — whether Archivo's own digit metrics are tabular by default hasn't been visually confirmed, for the same sandbox reason noted under criterion 4.
9. **Met.** No `.rounded_*()` call appears anywhere in `crates/bins/bin-desktop/src/` — `gpui`'s own unstyled-`div` default (`0`) stands in for the handoff's own "radius: 0 everywhere" rule, deliberately left unencoded as a constant (`theme.rs`'s own doc comment explains why). `.shadow(...)` appears in exactly two places, the command palette (`palette.rs`) and the collapsed-rail tooltip (`rail/primary.rs`) — the criterion's own two named exceptions. Nothing in the crate calls an animation/transition API. No code sets up a `gpui`-native focus-ring style (`.focus(...)`) anywhere; the three zones' own focus indicator is the hand-rolled 2px ink border from criterion 2, not a browser/toolkit-style ring.
10. **Met.** `Palette::matches` filters and ranks over `command::all()` (a plain iterator over `command::COMMANDS`) with no per-command special-casing; adding a new `Command` to that array is the entire change needed for it to appear, ranked and rendered, in the palette.

### Not yet built

`?`'s help overlay has no ticket anywhere on the map (see its "Not yet specified") and isn't built. Every noun besides `Dashboard` still renders `render_view`'s generic `"{noun:?} -- not yet built"` placeholder and `ContextRail`'s own placeholder frame — each noun's real view interior is separate future work, per-noun, out of this map's scope (issue #144's "Out of scope"). The new `docs/ux/desktop/Settings/` handoff bundle (the `Settings` noun's own ten-pane surface and four modals) exists only as design intake so far — it hasn't been translated into a `gpui`-terms living spec the way this document translates Shell & Navigation, and no code against it has landed.
