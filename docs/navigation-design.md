# Navigation and keyboard grammar: design

This is the developer-facing design. The plain-language guide for people using the apps is [Getting around](getting-around.md).

This is the shared keyboard/navigation grammar both Personal Ledger clients — `bin-tui` (terminal, ratatui) and `bin-desktop` (GUI, gpui) — follow. It's the cross-client layer: `docs/ux/tui-mockups/navigation.md` and `docs/ux/desktop-mockups/README.md` are each client's own living reference for everything client-specific (screen layout, per-view keys, exact component structure); this document is what the two are meant to have in common, so a user moving between them keeps their muscle memory, and so a contributor building out either client's own grammar has one place that states the shared rules rather than two documents quietly drifting apart.

The full decision trail behind this document lives on the [TUI Keybindings & Shared Navigation Grammar](https://github.com/IanTeda/Personal-Ledger/issues/155) Wayfinder map and its children.

## Philosophy

Both clients commit to the same three things, independently arrived at before this document existed and confirmed here rather than invented fresh:

- **Every action must be reachable by name.** A keybinding is a shortcut to a command, never the only path to it. `bin-tui`'s own navigation doc states this outright; `bin-desktop`'s command palette exists for the same reason.
- **A command with no real behaviour yet says so explicitly when run**, rather than doing nothing or opening something half-built. Both clients' command surfaces show a `"not yet built"` message rather than silently swallowing the attempt.
- **The interface always says what mode it's in.** Both clients are modal: a status line names the mode whenever it isn't the resting one, so a keystroke's meaning is never ambiguous.

## Three key shapes, not one leader-key scheme

The grammar has three genuinely different key shapes, each solving a different problem. [Research: leader-key conventions](https://github.com/IanTeda/Personal-Ledger/issues/156) considered and rejected unifying them into one mechanism — vim and Helix both keep their equivalents separate, and collapsing this app's own two into one buys nothing mechanically since the underlying state (a pending-chord flag, a mode flag) is still needed either way. Read together, in increasing order of what they open onto:

1. **A held-modifier chord for atomic, single-step global commands** (`Ctrl+<key>`-shaped, WM-style — mirrors Hyprland's/i3's `$mainMod`). No pending state, can never collide with typed text. Configurable per-command via `lib-config`'s `KeyBindingConfig::super_key` (`crates/libs/lib-config/src/keybindings.rs`) — see "What's actually configurable" below for which commands currently use it (as of this document, none in the truly-global set; see [#158](https://github.com/IanTeda/Personal-Ledger/issues/158)).
2. **A released-prefix leader, `g` then a letter, for jump navigation.** A finite, enumerable, mnemonic namespace — one letter per domain noun — safely explorable because an unrecognised completion just aborts the chord rather than doing something unexpected. Both clients implement the same shape: a bare `g` arms a pending chord, and the very next keypress ends it whatever the outcome. `bin-tui` keeps a `pending_leader` flag in `crates/bins/bin-tui/src/shell.rs`. `bin-desktop` binds each jump as a two-key chord (`g d`) in `lib-shell`'s global keymap (`crates/libs/lib-shell/src/keymap.rs`, `GLOBAL_BINDINGS`): the Shell's Presenter waits on the `g` prefix, and the bin's 1000ms timer (`PENDING_G_TIMEOUT` in `crates/bins/bin-desktop/src/shell/router/key_dispatch.rs`) ends the wait with `ShellInput::PrefixExpired`. `bin-tui` has no timeout.
3. **A bare, unmodified key opening the command surface** — typed, fuzzy-matched, open-ended, the escape hatch behind "every action must be reachable by name." Not part of the `g`-namespace and not modifier-gated, matching vim's and k9s's own ex-command-line convention: **`:`**.

## Where the two clients agree today, and where they don't

Both clients already independently arrived at shape 2 (a `g`-then-letter jump leader) in the same form. They disagree on shape 3:

- `bin-desktop` opens its command palette on a bare `:` already — this document's own recommendation, already shipped.
- `bin-tui` now agrees too: it opens its command popup on a bare `:` (the `open_command_popup` binding, `KeyBindingConfig`-configurable, `Shell::is_open_command_popup` in `crates/bins/bin-tui/src/shell.rs`) as of [Wire KeyBindingConfig into bin-tui's Shell/View global key handling](https://github.com/IanTeda/Personal-Ledger/issues/160). It previously opened on `Ctrl+;` — a deviation from its own original handoff that was never a considered decision (flagged, and never revisited, in issue #85's own closing comment) and carried a real fragility: on a terminal without the Kitty keyboard protocol's disambiguation extension, `Ctrl+;` collapsed onto the same control code as `Esc`.

Full rationale: [`docs/research/leader-key-conventions.md`](https://github.com/IanTeda/Personal-Ledger/blob/research/leader-key-conventions/docs/research/leader-key-conventions.md) (on the `research/leader-key-conventions` branch, not yet merged).

## The jump namespace

Both clients bind the same idea — `g` then a letter jumps straight to a noun — to their own noun set, which currently differs between them (the clients don't show the same set of domains yet):

`bin-tui` (`crates/bins/bin-tui/src/shell.rs`'s `pending_leader` handling): `g a` Accounts, `g b` Budgets, `g c` Categories, `g d` Dashboard, `g g` Tags (doubled — `t` was already Transactions'), `g k` Balance Checks, `g p` Payees, `g r` Reports, `g s` Settings, `g t` Transactions, `g u` Units.

`bin-desktop` (`lib_shell::keymap::GLOBAL_BINDINGS`, each jump a `jump_<noun>` command `[keybindings]` can rebind): `g a` Notifications, `g b` Budgets, `g c` Cash, `g d` Dashboard, `g f` Documents, `g i` Investments, `g k` Credit Cards, `g l` Transactions, `g n` Loans, `g o` Inventory, `g r` Reports, `g s` Settings, `g w` Bills.

The two disagree on more than just which nouns exist. `bin-desktop`'s nouns are its primary rail's: Accounts, Categories, Payees, Tags and Units are Settings pages there, reached through `g s`, and it has no Balance Checks noun. `bin-tui` has no Bills, Cash, Documents, Inventory, Investments, Loans, Credit Cards or Notifications noun. Where both have a noun the letter agrees (`g b`, `g d`, `g r`, `g s`) except Transactions, `g t` in `bin-tui` and `g l` in `bin-desktop`; and three letters jump to different nouns: `g a` (Accounts or Notifications), `g c` (Categories or Cash) and `g k` (Balance Checks or Credit Cards). Reconciling this fully means the two clients agreeing on one noun set and one letter-per-noun assignment — out of scope for this document today (it documents the grammar's *shape*, not a forced noun-for-noun remap), and worth its own decision once both clients' noun sets actually converge.

## Modes

Both clients share the same four-mode model — `Normal`, `Insert`, `Command`, `Search` — but they carry it at different fidelity today:

- `bin-desktop`: all four are real, Shell-level state (`nav::InputMode` in `crates/libs/lib-shell/src/nav.rs`, held by the Shell's Presenter), named in the status line whenever the mode isn't `Normal`. It adds two modal surfaces of its own, `Dialog` and `Filter` (the Transactions filter popover), each of which owns every key while it is open. `/` opens the active View's Search where the View has one, and is inert where it doesn't. The order a key is offered in (an open Dialog, a mode left in the bin, the global keymap, the focused module's keymap, a View not yet a module) is `crates/bins/bin-desktop/src/input.rs`'s `route_key`, described in [VIPER architecture](development/viper-architecture.md#input).
- `bin-tui`: `Normal` (rest), `Command` (the popup is open), and `Insert` (inside a form) are named the same way, but `bin-tui` has no unified mode enum in its live `Shell`/`View` architecture at all — these are informal names for ad-hoc state (`Option<CommandPopup>`, a popup's own presence), not one shared type the way `bin-desktop`'s `InputMode` is. `Search` exists too, and unlike `bin-desktop`'s, it's not a placeholder: `/` already opens real, working filter-typing in `AccountsView`, `PayeesView`, and `TagsView` (each view's own `filtering`/`filter` fields, e.g. `crates/bins/bin-tui/src/view/accounts.rs`). But it's per-view state, not a `Shell`-level mode — the status line doesn't announce `SEARCH` the way it announces `COMMAND`, and a view without its own `/` handling has no search at all.

So the four-mode *shape* is already shared; making `bin-tui`'s own four real and uniform at the `Shell` level (matching `bin-desktop`'s architecture, or migrating the three views' existing filter behaviour into it) is real follow-on work, not done as part of this document.

## Quit

Quit is deliberately **not** a single keybinding in either client's grammar. `bin-tui` today has three separate, coexisting mechanisms, none of them configurable, and this document doesn't collapse them into one:

- `Ctrl+C` — a hard-quit safety net, recognised before any view sees the key, with no graceful variant (`is_hard_quit` in `crates/bins/bin-tui/src/shell.rs`).
- `Q` / `q` — graceful quit, routed through its own `Action::GracefulQuit` so a future confirm-before-quit check has one place to slot in (`is_quit`/`is_graceful_quit`, same file).
- `:quit` — the named path, already real in the command popup's registry (`crates/bins/bin-tui/src/popup/command/commands/quit.rs`).

`bin-desktop` doesn't have an equivalent hard-quit safety net today (no analogue to `Ctrl+C`) — quitting a GUI app conventionally happens via window chrome, not a keyboard panic-key. Its own path is a close-button affordance, filling in the "3 window controls" slot its original design handoff specified but never built ([Add a close-button window control to bin-desktop's TopBar](https://github.com/IanTeda/Personal-Ledger/issues/162)).

See [#158](https://github.com/IanTeda/Personal-Ledger/issues/158)'s own resolution for the full reasoning: `quit` was deliberately kept out of `KeyBindingConfig`'s remappable set rather than forced into a `super_key`-composition rule that only one command would ever use.

## A page with several tables

When one page draws several tables, they share one selection, and the keys treat them as one list (`bin-desktop`'s Settings › Units, #714, following the Categories and Inventory pages, whose rows of two kinds already walk as one list):

- **`j`/`k` walk the tables as one list, in drawn order.** `g g`, `G` and `Ctrl-d`/`Ctrl-u` move over the same list, so `j` on the first table's last row selects the second table's first row. One row is selected at a time, in whichever table it is in. In the TUI `g g` is the Tags jump, so `Home` moves to the first row there.
- **`Tab` jumps between the tables.** It moves to the other table's last-selected row, or its first row if none has been selected yet. The page claims `Tab` ahead of the router while it has focus, as Bills claims it for its tabs; `Shift-Tab` is untouched and still cycles the focus zones.
- **The row keys follow the selected row's table.** `n`, `e`, `d` and the like act on the table the selection is in, and a row that offers no such action ignores the key. The status line switches with the table, and its `tab` hint names the table it jumps to.
- **A click on any table's row selects it**, at every Window Size, and the keys carry on from there.
- **A page's own keys never take a global key.** The Shell's global keys win in both Clients, so a module's default chord is one neither Client takes: Set base unit is `B`, since `b` toggles the Desktop's rail. The TUI has no `Ctrl+U` shortcut to Units (`g u` and `:unit` open it), so `Ctrl-u` moves up half a page in both. A test in each Client checks every module's defaults against its global keys (#764).

## What's actually configurable

`lib-config`'s `KeyBindingConfig` (`crates/libs/lib-config/src/keybindings.rs`, `[keybindings]` in `personal-ledger.conf`, `PERSONAL_LEDGER_KEYBINDINGS__*` env vars) is read by both Clients.

`bin-desktop` reads the whole section once at startup and hands it to `shell::router::build` (`ShellBindings`). Every key it binds is remappable by command name: the Shell's global keymap (`lib_shell::global_keymap`, the Toast keys included), the keys every module Dialog shares (`lib_shell::dialog_keymap`), and each VIPER module's page keymap (Units, Payees, Bills). The status-line hints, the `?` Help Dialog and the command palette show the key each command is bound to now: the palette lists every Command a keymap marks as listed, with its live key ([#846](https://github.com/IanTeda/Personal-Ledger/issues/846)). Views not yet on a VIPER module still hardcode their keys.

Within `bin-tui`, only the truly-global set is in scope for real remapping, per the map's own boundary — no per-view key (`n`/`e`/`d`/`a`/... inside any one domain's own `handle_key`) is configurable:

- `back` (`Esc`), `help` (`?`), and `open_command_popup` (`:`) — bare keys, no modifier by default; all three are read from `KeyBindingConfig` at `Shell` startup (`Shell::with_keybindings`, wired from `main.rs`) as of [#160](https://github.com/IanTeda/Personal-Ledger/issues/160).
- `quit` is **not** in this set — see "Quit" above.

`super_key` (the modifier held for globally-gated commands, mirroring a window manager's `$mainMod`) remains defined in `KeyBindingConfig` for any future command that needs it, but gates nothing in the current truly-global set — `quit`, the one command it was shaped around, dropped out of the remappable set entirely rather than being forced through it.

## Not yet specified

- A single, converged jump-noun set across both clients (see "The jump namespace" above) — waits on the clients' own noun sets converging first, not a grammar question this document can settle alone.
- `bin-tui`'s command popup already has ranking, Tab-complete, and `Ctrl+r` history (`crates/bins/bin-tui/src/popup/command/`); `bin-desktop`'s palette doesn't yet. Whether that's worth converging is a future call, out of scope here — `bin-desktop` code changes are out of scope for this map entirely (see the map's own "Out of scope").
- Giving `bin-tui`'s live `Shell` a real, unified mode concept (its first — `bin-tui` has none today, unlike `bin-desktop`'s `nav::InputMode`), with `Search` as a genuine `Shell`-level mode rather than per-view filter state — the three existing views' own working `/` filtering would need to either stay as-is under a new umbrella or actually migrate into it. Confirmed with the user as documentation-only for now (see "Modes" above); real enough to ticket once someone picks it up.
