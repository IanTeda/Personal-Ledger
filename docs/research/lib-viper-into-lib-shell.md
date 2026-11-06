# Moving lib-viper into lib-shell

Research for the question: what is the impact, blast radius and non-VIPER consequence of moving `lib-viper` into `lib-shell`, given that "VIPER modules feel like they are global shell modules"? Research only, as of `eea493f` on `concept` (2026-11-06).

## Executive summary

**Verdict: don't merge.** The move is mechanically small (one crate deleted, ~360 import paths rewritten) but it reverses a rule the architecture states three times and an option ADR-0048 already considered and rejected: "Feature crates depend on `lib-shell`. Rejected: it couples every module to the Shell, and the Shell's Router already depends on the modules" (`docs/adr/0048-modules-notify-the-shell-through-one-lib-toast-trait.md:25`). The premise conflates the Shell module (one VIPER module, the frame) with the shared VIPER vocabulary every module is written in.

Top three consequences:

1. **The dependency rule inverts.** Every one of the 19 feature and cross-domain crates would have to depend on `lib_shell`, which `deny.toml:82` bans and `viper_dependencies.rs:95-96` fails. Lifting both makes the Shell's Presenter, `NavState`, `Noun`, `ActiveView`, `ShellRouter` and `ShellRailCounts` visible to every module, and forbids the Shell from ever depending on a module without a Cargo cycle.
2. **Rebuild fan-out grows from 4 crates to 23.** Today a `lib-shell` edit rebuilds `lib_database` and the three bins; merged, it rebuilds every feature crate too. `lib-shell` is the faster-moving crate: 21 commits since 2026-10-24 against `lib-viper`'s 13 since it was created on 2026-10-22.
3. **Docs and tests churn for no new capability.** ~360 `lib_viper` sites in 195 `.rs` files, 22 `Cargo.toml`s, `deny.toml`, `viper_dependencies.rs` (its `SHARED_CRATES`, `SHELL_DEPENDANTS` and two tests), CLAUDE.md, `viper-architecture.md`, two ADRs, the glossary's sense of "Shell", and the workspace dependency diagram.

Recommendation: keep `lib-viper` as it is. If the name is what grates, rename it (option C below) rather than merge it.

## 1. Current dependents

### Of `lib-viper`

`lib_viper` is a direct normal dependency of 22 crates (`cargo tree --offline -i lib_viper -e normal --workspace --depth 1`, run with `SQLX_OFFLINE=true`):

- Domain feature crates, each at `Cargo.toml:20`: `lib-accounts`, `lib-bills`, `lib-budgets`, `lib-categories`, `lib-documents`, `lib-institutions`, `lib-inventory`, `lib-payees`, `lib-tags`, `lib-transactions`, `lib-units`.
- Other feature crates: `lib-dashboard` (`crates/libs/lib-dashboard/Cargo.toml:18`), `lib-settings` (`crates/libs/lib-settings/Cargo.toml:26`).
- Cross-domain modules: `lib-bill-payments` (`Cargo.toml:23`), `lib-account-deletion` (`:24`), `lib-unit-deletion` (`:21`), `lib-transaction-import` (`:25`), `lib-tag-removal` (`:23`), `lib-category-deletion` (`:26`).
- The Shell: `lib-shell` (`crates/libs/lib-shell/Cargo.toml:20`).
- The bins: `bin-desktop` (`crates/bins/bin-desktop/Cargo.toml:50`), `bin-tui` (`crates/bins/bin-tui/Cargo.toml:33`).

`lib-database` does not depend on it directly; its three mentions are in comments or reach it through feature crates (`grep -rn lib_viper crates/libs/lib-database`).

`lib-viper` itself depends only on `lib_config`, `lib_core` and `lib_toast` (`crates/libs/lib-viper/Cargo.toml:16-18`): `lib_config` for the `KeyBindingConfig` it re-exports (`crates/libs/lib-viper/src/keymap.rs:16`) and `lib_toast` for the `Notify` supertrait of `DialogRouter` (`crates/libs/lib-viper/src/dialog.rs:10`, `:36`). It is 890 lines in four modules: `keymap`, `view`, `dialog`, `display` (`crates/libs/lib-viper/src/lib.rs:11-14`).

### Of `lib-shell`

Only three crates depend on `lib_shell`: `bin-desktop` (`crates/bins/bin-desktop/Cargo.toml:58`), `bin-tui` (`crates/bins/bin-tui/Cargo.toml:29`) and `lib-database` (`crates/libs/lib-database/Cargo.toml:27`), the last only for `RailCounts` and `ShellRailCounts` (`crates/libs/lib-database/src/adapter/store.rs:90`). `bin-sync-server` reaches it transitively through `lib_database` (`cargo tree -i lib_shell --depth 3`; `crates/bins/bin-sync-server/Cargo.toml:29`).

### What `lib-shell` depends on

`lib_config`, `lib_core`, `lib_list_cursor`, `lib_locale`, `lib_toast`, `lib_viper`, `chrono` and `serde` (`crates/libs/lib-shell/Cargo.toml:15-24`). No feature crate, no toolkit, no `lib_database`. So a feature crate would not pull in anything heavy: every feature crate already has `lib_core`, `lib_locale` and `lib_toast`, and most have `lib_config`, `lib_list_cursor` and `chrono` (their `[dependencies]` sections). The new transitive arrivals are small: `serde` for all, and `lib_config` and `lib_list_cursor` for the few without them (`lib-dashboard`, the cross-domain modules). `lib-settings` already gets `lib_config` transitively through `lib_viper`'s re-export, as its manifest says (`crates/libs/lib-settings/Cargo.toml:19-20`).

The cost is therefore not in the external tree; it is in what the crates may see and in which direction the rule points.

## 2. What breaks under the dependency rule

### The checks that would fail

- `deny.toml:82` lists `lib_shell` with `wrappers = ["bin_desktop", "bin_tui", "lib_database"]`, so cargo-deny fails for each of the 19 crates that would gain it.
- `viper_dependencies.rs` fails in `forbidden`: any dependency on `SHELL` by a crate outside `SHELL_DEPENDANTS` is a violation (`crates/libs/lib-database/tests/viper_dependencies.rs:72-77`, `:95-96`). The fixture tests `violations_flags_any_module_depending_on_lib_shell_and_lib_shell_reaching_a_module` (`:281-315`) and `violations_flags_every_crate_but_the_bins_and_lib_database_depending_on_lib_shell` (`:317-340`) encode the rule directly and would have to be deleted or inverted. `SHARED_CRATES` (`:23`) loses `lib_viper`, and the name check (`:162-172`) would fail until it does.
- `viper-architecture.md:96` says `lib-database` "may depend on `lib-shell` only to answer its read port", and `:217` that it is "the one crate besides the bins that may depend on `lib-shell`". `crates/libs/lib-shell/src/lib.rs:17-18` says "No module depends on it". All three become false.

### The ADR already decided this

ADR-0048's context starts from "A feature crate can't depend on `lib-shell`, so the message type needs a crate that both sides may use" (`docs/adr/0048-modules-notify-the-shell-through-one-lib-toast-trait.md:7`), and it rejects "Feature crates depend on `lib-shell`" for coupling every module to the Shell (`:25`). The same ADR also rejected putting `Notice`/`Notify` in `lib-viper` so that "`lib-viper` stays about keymaps and form view models" (`:21`), which is the evidence that the project treats `lib-viper` as a deliberately narrow vocabulary crate. A merge would need a superseding ADR.

### Cycle risk

Today there is no cycle: `lib-shell` depends on no feature crate (`crates/libs/lib-shell/Cargo.toml:15-21`), and `lib-database` depends on both. After a merge, Cargo would still accept the graph, but the direction becomes load-bearing: the Shell could never again take a type from a module (for example a module's keymap or outcome type in the Help Dialog, `crates/libs/lib-shell/src/help.rs:1-15`, which today gets a View's keys as plain strings through `HelpGroup`) without a cycle. The architecture expects the opposite direction to be the free one: "the Shell's Router already depends on the modules" (ADR-0048:25), and the root Router "hosts whichever module's View is active" (`docs/development/viper-architecture.md:220`).

### What the modules would see

`lib-shell`'s public surface is the Shell's own state and ports: `NavState`, `FocusZone`, `InputMode`, `Noun`, `ActiveView`, `ShellPresenter`, `ShellInput`, `ShellRouter`, `ShellRailCounts`, `RailCounts`, `global_keymap`, `dialog_keymap`, the Help and Toast History Presenters, and `WindowSize` (`crates/libs/lib-shell/src/lib.rs:32-41`). The architecture keeps a module ignorant of where it is hosted: a Dialog's Router "opens nothing" (`viper-architecture.md:164`), and navigating stays a typed method on each module's own Router (ADR-0048:13). Once every module may `use lib_shell::Noun` or call `ShellRailCounts`, the only thing stopping a module from navigating by `Noun` or reading another domain's counts through the Shell is review, since `viper_dependencies.rs` checks crate edges, not paths. That is exactly the cross-domain read the stored-reads design routes through each module's own ports (`viper-architecture.md:97`).

## 3. Blast radius

### Code

`grep -rn lib_viper --include=*.rs crates` finds 362 sites in 195 files (excluding the stale `.claude/worktrees/issue-871` copy, which also mentions it):

| Crate | Sites | Files |
|---|---|---|
| `bin-desktop` | 117 | 78 |
| `lib-documents` | 32 | 13 |
| `lib-budgets` | 30 | 13 |
| `lib-units` | 25 | 12 |
| `lib-settings` | 22 | 9 |
| `lib-inventory` | 16 | 8 |
| `lib-bills` | 13 | 7 |
| `bin-tui` | 12 | 6 |
| `lib-shell` | 11 | 6 |
| `lib-institutions`, `lib-payees`, `lib-transactions` | 10 each | 5, 5, 4 |
| `lib-categories` | 9 | 5 |
| `lib-accounts`, `lib-tags` | 8 each | 4 each |
| `lib-tag-removal`, `lib-transaction-import` | 6 each | 3, 2 |
| `lib-account-deletion`, `lib-bill-payments`, `lib-category-deletion`, `lib-unit-deletion`, `lib-database` | 3 each | 1-2 |
| `lib-core`, `lib-dashboard` | 1 each | 1 each |

Most edits are a path rename (`lib_viper::keymap::` to `lib_shell::keymap::` or a new `lib_shell::viper::`), so the work is mechanical, but `lib-shell`'s own `keymap` and `dialog` modules already exist (`crates/libs/lib-shell/src/lib.rs:21-23`), so `lib_viper::keymap` and `lib_shell::keymap` would collide and need a sub-module or a merge of two different things (the shared command names versus the global keymap).

Manifests: 22 `Cargo.toml`s plus the workspace member list (`Cargo.toml:102`) and `Cargo.lock` (23 lines).

### The `Hint` re-export

`lib-shell` re-exports `lib_viper::view::Hint` "so the bins name it here" (`crates/libs/lib-shell/src/lib.rs:43-44`); the bins have 36 `lib_shell::Hint` uses and none of `lib_viper::view::Hint` (grep). `Hint` cannot move to `lib-shell` on its own: 14 feature crates build `Hint`s into their view models (grep for `Hint` under `crates/libs/lib-*/src`), and `DialogFrame::legend` returns `Vec<Hint>` (`crates/libs/lib-viper/src/view.rs:171`, `:179`, `:205`). The re-export is the existing compromise that already gives the bins the "it's the Shell's" spelling.

### Docs, skills and diagrams

Mentions of `lib-viper` or `lib_viper` (`git ls-files | xargs grep -c`):

- `CLAUDE.md` (4: the Domain crates, `lib-dashboard`, `lib-settings` and `lib-viper` entries, and the `lib-shell` entry's "only the bins and `lib-database`" rule).
- `docs/development/viper-architecture.md` (9, including the Layout tree at `:67` and the Dependencies rules at `:91`, `:94`, `:96`).
- `docs/adr/0037-…md:98` and `docs/adr/0048-…md:21`, `:25` (accepted ADRs: a reversal needs a new ADR, not an edit).
- `docs/development/documents.md`, `payees.md`, `settings.md`, `transactions.md`; `docs/ux/tui-mockups/navigation.md`; `.claude/skills/priorities-tickets/SKILL.md`; `docs/research/shell-messaging-options.md` (18, historical) and `desktop-tracing-inventory.md`.
- `docs/diagrams/workspace-dependencies.{html,json,finalize.json}` (19 together).
- `deny.toml:65` (comment) and `:82`.
- `GLOSSARY.md` doesn't mention `lib-viper`, but its **Shell** entry says "the View's own content is never the Shell" (`GLOSSARY.md:108`), which a merge would contradict in spirit.

### Tests

`viper_dependencies.rs` as above. No other test names the crate; unit tests inside `lib-viper` (`crates/libs/lib-viper/src/view.rs:231`, `dialog.rs:44`) move with the code.

## 4. Non-VIPER consequences

- **Incremental rebuild fan-out.** A change to `lib-viper` already rebuilds 22 direct dependants plus `lib_database` and `bin_sync_server`. A change to `lib-shell` today rebuilds only `lib_database` and the three bins (`cargo tree -i lib_shell`). Merged, every Shell Presenter change (status line, Toasts, rail counts, global keymap, Help) rebuilds all 19 module crates and re-runs their tests under `bacon`. `lib-shell` is 3,272 lines and growing ticket by ticket (#820; `crates/libs/lib-shell/src/lib.rs:6`), with 21 commits since 2026-10-24, against 13 for `lib-viper` since its creation (`git log --oneline -- crates/libs/lib-shell | wc -l`; `git log --since=2026-10-22 -- crates/libs/lib-viper`).
- **The TUI.** `bin-tui` depends on both (`crates/bins/bin-tui/Cargo.toml:29`, `:33`) but uses `lib_viper` only for the Units View (`src/units/`) and two `MOVE_FIRST` references in `src/shell.rs:4118`, `:4179`. Merging changes its imports only; it already builds `lib-shell`.
- **`bin-sync-server`** pulls `lib_shell` in transitively through `lib_database` today; nothing changes for it.
- **`lib-toast`, `lib-list-cursor`, `lib-colour-theme`.** These stay separate shared crates either way. ADR-0048 placed `Notice`/`Notify` in `lib-toast` precisely because a feature crate can't depend on `lib-shell` (`:7`); with a merge that reason disappears, inviting a second churn to fold `lib-toast` into `lib-shell` too. `lib-list-cursor` and `lib-colour-theme` are unaffected.
- **cargo-deny.** `deny.toml:82` has to lose its `lib_shell` entry (or list all 19 crates as wrappers, which makes the ban meaningless). The `deny.toml:62-75` rationale comment needs rewriting. Note that its wording "a new lib such as `lib_viper` is banned by not being listed" (`:65`) is misleading today: an unlisted crate is unrestricted, which is why `lib_viper` is reachable by all.
- **Crates using `lib-viper` that are not VIPER modules.** None outside the bins and `lib-shell`: every direct dependant is a feature crate, a cross-domain module, the Shell or a bin. `lib-core` only names it in a doc comment (`crates/libs/lib-core/src/display.rs:5`).
- **Docs build, graphify, SQLx offline.** `mise run docs-build` regenerates rustdoc per crate, so one crate fewer; the intra-doc links in the 362 sites move with the paths. graphify needs a `graphify update .` after the move (AST only). SQLx's `.sqlx` cache is untouched: neither crate has queries.

## 5. Is a VIPER module a "global Shell module"?

No, by the project's own definitions:

- **The Shell is one VIPER module, not the container of them.** "`lib-shell` is a VIPER module like any other" (`docs/development/viper-architecture.md:214`); its Presenter holds the frame's state and its Router is the root Router that hosts the active module's View (`:216-220`). The glossary defines the Shell as "the always-present frame … the View's own content is never the Shell" (`GLOSSARY.md:108`), with ADR-0045 naming the frame the Shell.
- **`lib-viper` is the vocabulary, not a module.** Its crate doc says it holds "the generic types every VIPER module's Presenters and keymap use", that it is its own crate "so the next module imports them instead of copying them, and no feature crate depends on another for them", and that it is "Not a feature crate: it holds no domain" (`crates/libs/lib-viper/src/lib.rs:1-9`). `viper-architecture.md:94` repeats this.
- **Its origin.** It was created by #760 ("CORE: move the shared keymap and form view types out of lib-units", parent #747), commit `a60baae` on 2026-10-22, so that slice 2 (#757) wouldn't copy `Keymap`, `Binding`, `Choice`, `TextFieldView`, `SelectView` and `DialogFrame` out of `lib-units`. The issue specified "The crate is not a feature crate: it depends only on `lib-core`, `lib-config` … and `lib-list-cursor` if needed" (`gh issue view 760`). `lib-shell` came two days later (`6c2573d`, #822). So `lib-viper` predates the Shell module and was never about the Shell.
- **What is Shell-flavoured in `lib-viper`.** Only `Hint` (the status line's row) and, arguably, the shared command names that both the Shell's Dialog keymap and module keymaps use (`SELECT`, `BACK`, `NEXT_FIELD`, … at `crates/libs/lib-viper/src/keymap.rs:187-226`). Both are consumed by the modules as much as by the Shell, which is why they sit below both. The Shell-only parts (the global keymap, the Dialog keymap's chords, `NavState`) already live in `lib-shell` (`crates/libs/lib-shell/src/lib.rs:32-36`).

The intuition is understandable: every module's Dialog keys resolve through the Shell's Dialog keymap, every module hands the Shell `Hint`s and `Notice`s, and the Shell hosts every View. But that makes the Shell the *consumer and host* of the shared vocabulary, not its *owner*. In the classic layering, the host sits above the hosted, and a shared contract sits below both; `lib-viper` is that contract.

## 6. Alternatives

| Option | What changes | Gains | Costs |
|---|---|---|---|
| A. Merge `lib-viper` into `lib-shell` | As in sections 2-3 | One fewer crate; "VIPER types come from the Shell" reads naturally in the bins | Inverts the dependency rule, reverses ADR-0048, exposes Shell internals to modules, 4-to-23 rebuild fan-out, ~360 sites plus docs |
| B. `lib-shell` re-exports `lib-viper` wholesale (`pub use lib_viper as viper;` or `pub use lib_viper::*` under a module) | One line in `crates/libs/lib-shell/src/lib.rs`, and optionally the bins drop their direct `lib_viper` dependency | The bins see one crate for "the frame and its vocabulary", as `Hint` already does (`lib.rs:43-44`); feature crates untouched; no rule changes | Two spellings for the same type in the bins unless they migrate fully (195 files, of which 84 are bin files); a small doc note in `viper-architecture.md` |
| C. Rename `lib-viper` (e.g. `lib-module-kit`, `lib-presenter-kit`, `lib-viper-kit`) | Crate rename, same mechanical churn as A for paths, but no rule change | Name says "shared toolkit for modules", removing the impression it is a module or the Shell | ~360 sites, 22 manifests, docs; churn without behaviour change, so worth it only if the name is genuinely misleading people |
| D. Move only Shell-specific items into `lib-shell` | e.g. `Hint` | Tidier ownership on paper | Doesn't work: `Hint` and the shared command names are built by feature crates (14 crates use `Hint`; `DialogFrame::legend` returns it), so they'd have to stay below both |
| E. Keep as is, sharpen the wording | Fix `deny.toml:65`'s misleading comment; add one sentence to `viper-architecture.md`'s Shell section that the Shell consumes `lib-viper` like every module | Zero code churn; records the distinction the question surfaced | None meaningful |

**Recommendation: E, optionally with B for the bins.** E costs two comment edits and answers the confusion at its source. B is cheap if the bins' imports feel scattered, and keeps every rule intact because only bins depend on `lib-shell`. C is defensible only if the name keeps causing this question; A is not, without first superseding ADR-0048 and accepting that the Shell can never depend on a module.
