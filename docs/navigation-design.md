# Navigation and keyboard grammar: design

This is the developer-facing design. The plain-language guide for people using the apps is [Getting around](navigation.md).

This is the shared keyboard/navigation grammar both Personal Ledger clients — `bin-tui` (terminal, ratatui) and `bin-desktop` (GUI, gpui) — follow. It's the cross-client layer: `docs/ux/tui/navigation.md` and `docs/ux/desktop/README.md` are each client's own living reference for everything client-specific (screen layout, per-view keys, exact component structure); this document is what the two are meant to have in common, so a user moving between them keeps their muscle memory, and so a contributor building out either client's own grammar has one place that states the shared rules rather than two documents quietly drifting apart.

The full decision trail behind this document lives on the [TUI Keybindings & Shared Navigation Grammar](https://github.com/IanTeda/Personal-Ledger/issues/155) Wayfinder map and its children.

## Philosophy

Both clients commit to the same three things, independently arrived at before this document existed and confirmed here rather than invented fresh:

- **Every action must be reachable by name.** A keybinding is a shortcut to a command, never the only path to it. `bin-tui`'s own navigation doc states this outright; `bin-desktop`'s command palette exists for the same reason.
- **A command with no real behaviour yet says so explicitly when run**, rather than doing nothing or opening something half-built. Both clients' command surfaces show a `"not yet built"` message rather than silently swallowing the attempt.
- **The interface always says what mode it's in.** Both clients are modal: a status line names the mode whenever it isn't the resting one, so a keystroke's meaning is never ambiguous.

## Three key shapes, not one leader-key scheme

The grammar has three genuinely different key shapes, each solving a different problem. [Research: leader-key conventions](https://github.com/IanTeda/Personal-Ledger/issues/156) considered and rejected unifying them into one mechanism — vim and Helix both keep their equivalents separate, and collapsing this app's own two into one buys nothing mechanically since the underlying state (a pending-chord flag, a mode flag) is still needed either way. Read together, in increasing order of what they open onto:

1. **A held-modifier chord for atomic, single-step global commands** (`Ctrl+<key>`-shaped, WM-style — mirrors Hyprland's/i3's `$mainMod`). No pending state, can never collide with typed text. Configurable per-command via `lib-config`'s `KeyBindingConfig::super_key` (`crates/libs/lib-config/src/keybindings.rs`) — see "What's actually configurable" below for which commands currently use it (as of this document, none in the truly-global set; see [#158](https://github.com/IanTeda/Personal-Ledger/issues/158)).
2. **A released-prefix leader, `g` then a letter, for jump navigation.** A finite, enumerable, mnemonic namespace — one letter per domain noun — safely explorable because an unrecognised completion just aborts the chord rather than doing something unexpected. Both clients already implement this identically in shape: a pending-chord flag armed by a bare `g`, cleared unconditionally on the very next keypress regardless of outcome (`bin-tui`'s `pending_leader` in `crates/bins/bin-tui/src/shell.rs`; `bin-desktop`'s `pending_g` in `crates/bins/bin-desktop/src/shell.rs`), plus a timeout in `bin-desktop`'s case (1000ms) that `bin-tui` doesn't currently have.
3. **A bare, unmodified key opening the command surface** — typed, fuzzy-matched, open-ended, the escape hatch behind "every action must be reachable by name." Not part of the `g`-namespace and not modifier-gated, matching vim's and k9s's own ex-command-line convention: **`:`**.

## Where the two clients agree today, and where they don't

Both clients already independently arrived at shape 2 (a `g`-then-letter jump leader) in the same form. They disagree on shape 3:

- `bin-desktop` opens its command palette on a bare `:` already — this document's own recommendation, already shipped.
- `bin-tui` now agrees too: it opens its command popup on a bare `:` (the `open_command_popup` binding, `KeyBindingConfig`-configurable, `Shell::is_open_command_popup` in `crates/bins/bin-tui/src/shell.rs`) as of [Wire KeyBindingConfig into bin-tui's Shell/View global key handling](https://github.com/IanTeda/Personal-Ledger/issues/160). It previously opened on `Ctrl+;` — a deviation from its own original handoff that was never a considered decision (flagged, and never revisited, in issue #85's own closing comment) and carried a real fragility: on a terminal without the Kitty keyboard protocol's disambiguation extension, `Ctrl+;` collapsed onto the same control code as `Esc`.

Full rationale: [`docs/research/leader-key-conventions.md`](https://github.com/IanTeda/Personal-Ledger/blob/research/leader-key-conventions/docs/research/leader-key-conventions.md) (on the `research/leader-key-conventions` branch, not yet merged).

## The jump namespace

Both clients bind the same idea — `g` then a letter jumps straight to a noun — to their own noun set, which currently differs between them (the clients don't show the same set of domains yet):

`bin-tui` (`crates/bins/bin-tui/src/shell.rs`'s `pending_leader` handling): `g a` Accounts, `g b` Budgets, `g c` Categories, `g d` Dashboard, `g g` Tags (doubled — `t` was already Transactions'), `g k` Balance Checks, `g p` Payees, `g r` Reports, `g s` Settings, `g t` Transactions, `g u` Units.

`bin-desktop` (`crates/bins/bin-desktop/src/shell.rs`'s `jump_noun_for_key`): `g a` Accounts, `g b` Budgets, `g c` Categories, `g d` Dashboard, `g l` Transactions, `g p` Payees, `g r` Reports, `g s` Settings, `g t` Tags, `g w` Bills.

The two disagree on more than just which nouns exist (`bin-desktop` has no Balance Checks or Units nouns yet; `bin-tui` has no Bills noun yet): where both have Tags and Transactions, `bin-tui` gives Tags the doubled `g g` and keeps Transactions on `g t`, while `bin-desktop` moved Transactions to `g l` to give Tags the single `g t`. Reconciling this fully means the two clients agreeing on one noun set and one letter-per-noun assignment — out of scope for this document today (it documents the grammar's *shape*, not a forced noun-for-noun remap), and worth its own decision once both clients' noun sets actually converge.

## Modes

Both clients share the same four-mode model — `Normal`, `Insert`, `Command`, `Search` — but they carry it at different fidelity today:

- `bin-desktop`: all four are real, Shell-level state (`nav::InputMode` in `crates/bins/bin-desktop/src/nav.rs`), named in the status line whenever the mode isn't `Normal`. `Search` is entered on `/` and pre-empts other key handling like the other three, though nothing real is wired to it yet — a state flag, not a built search feature.
- `bin-tui`: `Normal` (rest), `Command` (the popup is open), and `Insert` (inside a form) are named the same way, but `bin-tui` has no unified mode enum in its live `Shell`/`View` architecture at all — these are informal names for ad-hoc state (`Option<CommandPopup>`, a popup's own presence), not one shared type the way `bin-desktop`'s `InputMode` is. `Search` exists too, and unlike `bin-desktop`'s, it's not a placeholder: `/` already opens real, working filter-typing in `AccountsView`, `PayeesView`, and `TagsView` (each view's own `filtering`/`filter` fields, e.g. `crates/bins/bin-tui/src/view/accounts.rs`). But it's per-view state, not a `Shell`-level mode — the status line doesn't announce `SEARCH` the way it announces `COMMAND`, and a view without its own `/` handling has no search at all.

So the four-mode *shape* is already shared; making `bin-tui`'s own four real and uniform at the `Shell` level (matching `bin-desktop`'s architecture, or migrating the three views' existing filter behaviour into it) is real follow-on work, not done as part of this document.

## Quit

Quit is deliberately **not** a single keybinding in either client's grammar. `bin-tui` today has three separate, coexisting mechanisms, none of them configurable, and this document doesn't collapse them into one:

- `Ctrl+C` — a hard-quit safety net, recognised before any view sees the key, with no graceful variant (`is_hard_quit` in `crates/bins/bin-tui/src/shell.rs`).
- `Q` / `q` — graceful quit, routed through its own `Action::GracefulQuit` so a future confirm-before-quit check has one place to slot in (`is_quit`/`is_graceful_quit`, same file).
- `:quit` — the named path, already real in the command popup's registry (`crates/bins/bin-tui/src/popup/command/commands/quit.rs`).

`bin-desktop` doesn't have an equivalent hard-quit safety net today (no analogue to `Ctrl+C`) — quitting a GUI app conventionally happens via window chrome, not a keyboard panic-key. Its own path is a close-button affordance, filling in the "3 window controls" slot its original design handoff specified but never built ([Add a close-button window control to bin-desktop's TopBar](https://github.com/IanTeda/Personal-Ledger/issues/162)).

See [#158](https://github.com/IanTeda/Personal-Ledger/issues/158)'s own resolution for the full reasoning: `quit` was deliberately kept out of `KeyBindingConfig`'s remappable set rather than forced into a `super_key`-composition rule that only one command would ever use.

## What's actually configurable

`lib-config`'s `KeyBindingConfig` (`crates/libs/lib-config/src/keybindings.rs`, `[keybindings]` in `personal-ledger.conf`, `PERSONAL_LEDGER_KEYBINDINGS__*` env vars) is `bin-tui`-only today — `bin-desktop` doesn't consume it, and bringing it in is explicitly out of scope for this map (see "Out of scope" on the map). Within `bin-tui`, only the truly-global set is in scope for real remapping, per the map's own boundary — no per-view key (`n`/`e`/`d`/`a`/... inside any one domain's own `handle_key`) is configurable:

- `back` (`Esc`), `help` (`?`), and `open_command_popup` (`:`) — bare keys, no modifier by default; all three are read from `KeyBindingConfig` at `Shell` startup (`Shell::with_keybindings`, wired from `main.rs`) as of [#160](https://github.com/IanTeda/Personal-Ledger/issues/160).
- `quit` is **not** in this set — see "Quit" above.

`super_key` (the modifier held for globally-gated commands, mirroring a window manager's `$mainMod`) remains defined in `KeyBindingConfig` for any future command that needs it, but gates nothing in the current truly-global set — `quit`, the one command it was shaped around, dropped out of the remappable set entirely rather than being forced through it.

## Not yet specified

- A single, converged jump-noun set across both clients (see "The jump namespace" above) — waits on the clients' own noun sets converging first, not a grammar question this document can settle alone.
- `bin-tui`'s command popup already has ranking, Tab-complete, and `Ctrl+r` history (`crates/bins/bin-tui/src/popup/command/`); `bin-desktop`'s palette doesn't yet. Whether that's worth converging is a future call, out of scope here — `bin-desktop` code changes are out of scope for this map entirely (see the map's own "Out of scope").
- Giving `bin-tui`'s live `Shell` a real, unified mode concept (its first — `bin-tui` has none today, unlike `bin-desktop`'s `nav::InputMode`), with `Search` as a genuine `Shell`-level mode rather than per-view filter state — the three existing views' own working `/` filtering would need to either stay as-is under a new umbrella or actually migrate into it. Confirmed with the user as documentation-only for now (see "Modes" above); real enough to ticket once someone picks it up.
