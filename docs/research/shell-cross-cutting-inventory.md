# `shell.rs` cross-cutting concerns: fact sheet

Facts only, for [Inventory shell.rs's cross-cutting concerns](https://github.com/IanTeda/Personal-Ledger/issues/539) on the [Desktop root layout and Chrome](https://github.com/IanTeda/Personal-Ledger/issues/533) map. No decisions. Line numbers are `crates/bins/bin-desktop/src/shell.rs` at commit `22e6f19` (10,241 lines).

"Cross-cutting" means not one domain's wiring. Per-domain methods (`handle_<domain>_key`, `apply_<domain>_movement`, `open_<domain>_dialog`, `handle_<domain>_*_click`, the Budgets and Bills dialogs' `render_*` methods, lines ~2207–7250 and 7468–7650) belong to Move domain wiring and state out of `shell.rs` (#531) and are listed here only where a cross-cutting concern calls into them.

## Overview

| # | Concern | Main span | `Shell` fields owned | Called from |
|---|---|---|---|---|
| 1 | Construction (composition root) | 781–918 | all | `lib.rs:118` (`build_shell`), tests |
| 2 | Persisted state and startup setters | 920–962 | `settings_selected_section`, `explorer_filters`, `settings_start_sidebar_minimised`, `dismiss_toasts_binding`, `toast_history_binding`, `documents_mode/scope/sort` (via `documents_ui`) | `lib.rs:119–124`, `lib.rs:261` (quit) |
| 3 | Toasts: model, clock, pause, history | 964–983, 1039–1100, 2142 | `toasts`, `toasts_hovered`, `dismiss_toasts_binding`, `toast_history_binding`, `debug_toast_kind` | `lib.rs:125`, key dispatch, commands, domain code via `raise_toast` |
| 4 | Log capture feed (Tracing page) | 985–1037 | `settings_log`, `settings_log_list` | `lib.rs:126` |
| 5 | Test and accessor surface | 1102–1136, 7770–7774 | reads `nav`, `focus_handle`, `status_message`, `command_history`, `inventory` | `tests/`, `lib.rs` |
| 6 | Key dispatch | 1138–1382, 9130–9150, 9959–10000 | `pending_g`, `status_message`, plus touches many | gpui `on_key_down` |
| 7 | Input-mode tiers (palette, search, dialog) | 1507–1510, 1913–1992 | `palette`, `dialog` | key dispatch |
| 8 | Dialog host plumbing | 1994–2205 | `dialog` | every domain |
| 9 | Focus, movement and view scroll | 1532–1555, 1766–1911 | `nav`, `view_scroll_handle` | key dispatch |
| 10 | Status line: flash, hints, page status, help sheet | 177–550, 1456–1499, 1514–1530, 8961–9114 | `status_message` | Render |
| 11 | Command palette execution | 148–153, 7252–7416, 7753–7768 | `command_history`, `palette` | palette `Enter`, empty-state click |
| 12 | Rail interaction | 7418–7466 | `collapsed_rail_tooltip`, `hover_generation` | Render closures, key dispatch |
| 13 | File explorer (`:open` / `:new`) | 140–146, 7650–7751 | `file_explorer`, `explorer_filters` | `run_command`, Render closures |
| 14 | Deferred effects needing `App` | 9142–9149 (drain) | `pending_colour_change`, `pending_budgets_export`, `pending_file_action` | key handlers set, listener drains |
| 15 | Frame rendering | 7776–9665, 9667–9957 | reads all | gpui |

## 1. Construction (composition root)

- `Shell::new` (781) delegates to `Shell::with_today` (788–918), which seeds every domain's stub data in dependency order: `accounts::default_accounts`, `categories::default_categories`, `payees::default_payees`, `tags::default_tags`, `transactions::default_transactions`, `bills::default_bills` (writes settling Transactions), `inventory::default_inventory`, `documents::default_documents` (writes Transactions), `budgets::default_budgets`, `documents::types::default_types`, `settings::default_units/price_sources/institutions`.
- Then a ~95-line struct literal initialising all ~95 fields (`Shell` struct itself: 552–779).
- `accounts::default_accounts()` is called twice (once for seeding Transactions, once for the field).
- The log capture starts as a private empty `LogBuffer` until concern 4 replaces it.

## 2. Persisted state and startup setters

- `set_documents_state` (921) → `documents_ui`'s `set_documents_persisted`.
- `settings_page` / `set_settings_page` (931, 935), `explorer_filters` / `set_explorer_filters` (939, 943), `start_sidebar_minimised` / `set_start_sidebar_minimised` (947, 951): plain field accessors.
- `set_dismiss_toasts_binding` (955), `set_toast_history_binding` (959): `[keybindings]` config.
- Callers: `build_shell` in `lib.rs:118–126` after construction; `lib.rs:261` reads them back into `persistence::PersistedState` at quit.

## 3. Toasts

- `open_toast_history` (964): closes the palette, opens `OpenDialog::ToastHistory` in the Dialog host.
- `set_toasts_on` (971): the Toasts Preference, via `lib_toast::Toasts::set_display`.
- `raise_toast` (979): the one entry point every domain uses (`Local::now()` stamp).
- `start_toast_clock` (1039–1062): `cx.spawn` loop on `Timer::after(crate::toast::TICK)`, calls `advance_toasts`, stops when the entity is gone. Called once from `lib.rs:125`.
- `advance_toasts` (1064–1089): pauses while hovered or `modal_open()`, advances, reports whether a redraw is needed.
- `modal_open` (1091–1100): reads `palette`, `file_explorer`, `nav.mode()` (`Command | Dialog | Filter | Help`). Its only caller is `advance_toasts`.
- `toast_history_open` (2142): `dialog` is `OpenDialog::ToastHistory`.
- In key dispatch: `key_router::dismisses_toasts` / `opens_toast_history` (1182–1204) and the debug-only `F9` raiser (1206–1214).
- In `run_command`: `DismissToast`, `DismissAllToasts`, `SetToasts`, `OpenToastHistory` effects.
- In Render: `on_toast_dismiss` / `on_toast_hover` closures (7852–7868), `toast_layer` (7869–7882), `on_toast_history_close`, the status line's `toast_echo`, the history overlay, and `toast_layer` as the very last child (9661).
- Draws via `crate::toast` and `view::toast_history`; the model is `lib_toast`.

## 4. Log capture feed

- `set_log_capture` (985–1021): installs a waker on the `lib_tracing::LogBuffer` (`tokio::sync::Notify`), replaces `settings_log` with a live `LogView`, resets `settings_log_list`, spawns a loop that coalesces for `LOG_COALESCE` on the background executor's timer (so headless tests can advance it) and calls `apply_log_change`.
- `apply_log_change` (1023–1037): splices the gpui `ListState`, keeping a reader at the top anchored to the newest.
- Constants `LOG_COALESCE` (120, `pub #[doc(hidden)]`, used by tests), `LOG_LIST_OVERDRAW` (123), `LOG_LINE_STEP` (126).
- Feeds only Settings › Tracing; the Tracing page's key handling (`handle_settings_tracing_key` 1674, `step_tracing_level` 1711, `set_tracing_level` 7593, `handle_clear_logs_click` 7634) is Settings domain code.

## 5. Test and accessor surface

- `#[doc(hidden)] pub`: `open_ledger_for_test` (1103), `empty_inventory_for_test` (1110), `status_message` (1116), `command_history` (1122).
- `pub`: `nav` (1126), `focus_handle` (1130); `impl Focusable for Shell` (7770–7774).
- The snapshot accessors already live in `shell/snapshots/` (one file per destination plus `overlays.rs` and `palette.rs`, each an `impl Shell` block), re-exported at 26–31.

## 6. Key dispatch

Two layers.

**The gpui listener in Render** (`on_key_down`, 9130–9150) calls, in order, `handle_settings_form_key`, `handle_settings_tracing_key`, `handle_settings_focus_key`, `handle_colour_theme_grid_key`, `handle_bills_tab_key`, `handle_budgets_tab_key` and finally `handle_key_down`; notifies on any `true`; then drains concern 14's deferred effects. The pre-`handle_key_down` handlers are there because they need an `App` (e.g. `colour_theme::chosen_index(cx)`) or run before the router.

**`handle_key_down`** (1166–1382), in order:

1. Takes `pending_g` and checks it against `PENDING_G_TIMEOUT` (130).
2. Toast dismiss and toast-history bindings (concern 3), debug `F9`.
3. Pre-router domain handlers, each gated on no pending `g`: `handle_budgets_plan_edit_key`, `handle_import_key`, `handle_documents_key`, `handle_settings_documents_reorder_key`, `handle_settings_inventory_reorder_key`.
4. `key_router::route_key(mode, pending_g, key, ctrl, shift)` → `KeyOutcome`.
5. `Esc` (`ClosePopupsAndExitMode`, 1250–1289) clears state across domains directly: `Dialog::close_open_select`, `bills_filter_focus`, `palette`, `file_explorer`, `transactions_filter_form` (and its open select), `close_dialog`, `budgets_plan_edit`, `transactions_search` (Search on Transactions), `documents_cancel_search` (Search on Documents), `nav.exit_mode`.
6. Clears `status_message` on any other key.
7. Applies the outcome: delegate to palette / search / dialog / filter tiers (concern 7), `JumpToNoun`, `PendingGUnbound` flash, `EnterCommand`, `EnterSearch` (Documents special-cased, Settings inert), `EnterHelp` / `CloseHelp`, `EnterInsert`, `ToggleRail`, focus cycling, `ArmPendingG`, `Movement` (concern 9).
8. `NoOp` falls through a fixed chain of domain handlers: accounts, categories, payees, settings documents, settings inventory, tags, bills, budgets, transactions.

Helpers: `command_echo` (1138–1164, the status line's COMMAND echo from `palette` or `file_explorer`), free fns `dialog_key` (9959) and `typed_char` (9988).

The routing decision itself is already `gpui`-free in `key_router.rs`; `shell.rs` holds the impure application and both fallthrough chains.

## 7. Input-mode tiers

- `open_palette` (1507): enters `InputMode::Command` with a `Palette` seeded from `command_history`.
- `handle_palette_key` (1913–1962): backspace, up/down, tab-complete, `ctrl-r` history, `enter` → `run_command`, printable chars.
- `handle_search_key` (1964–1977): dispatches by noun to `handle_transactions_search_key` or `handle_documents_search_key`.
- `handle_dialog_key` (1979–1992): `dialog_host::handle_key` on `dialog`; `Confirm` → `confirm_open_dialog`.
- `handle_filter_key` (2377) is the fourth tier but Transactions-only (domain).

## 8. Dialog host plumbing

- `open_dialog` (1994), `close_dialog` (2000): the only writers of `dialog`, keeping `InputMode::Dialog` in step.
- `confirm_open_dialog` (2008–2030): validity check, then a `match` on `OpenDialog` dispatching to each domain's `apply_<domain>_dialog` (Settings, Accounts, Categories, Payees, DocumentTypes, Inventory, Tags, Bills, Budgets, Documents; ToastHistory is read-only).
- Typed accessors `<domain>_dialog()` / `<domain>_dialog_mut()` (2032–2205, 11 pairs: settings, categories, document types, inventory, bills, budgets, tags, payees, accounts) narrowing `dialog` to one variant; `apply_budgets_dialog` (2106) and `open_budgets_dialog` (2160) sit among them.

## 9. Focus, movement and view scroll

- `apply_movement` (1532–1542): dispatches on `nav.focus()` (`FocusZone::{PrimaryRail, ContextRail, View}`), resets scroll if the noun changed.
- `apply_primary_rail_movement` (1766–1784, `PRIMARY_RAIL_HALF_PAGE` 155), `apply_context_rail_movement` (1786–1813, `rail::context::entity_count`).
- `apply_view_movement` (1833–1911): a fallthrough chain of 11 checks (`accounts_page_has_focus`, `settings_categories/tags/payees/documents/inventory_page_has_focus`, then noun == Transactions / Documents / Bills / Budgets, then the Settings index) to per-domain movement; otherwise scrolls `view_scroll_handle` by `VIEW_LINE_STEP` (160).
- `reset_view_scroll` (1549–1555): zeroes the scroll and resets domain focus fields `settings_focus`, `colour_theme_focus`, `documents_focus`. Called on every noun change (key jump, rail click, `run_command`).
- Settings' own focus model (`open_settings_page` 1562, `select_settings_page` 1572, `focus_settings_page` 1583, `focus_settings_index` 1592, `handle_settings_form_key` 1602, `handle_settings_focus_key` 1720, `apply_settings_section_movement` 1815, `handle_colour_theme_grid_key` 1388, `leave_colour_theme_grid` 1502) is Settings domain code, but `open_settings_page` is the hand-off target that commands and other domains call.

## 10. Status line

- `status_message` field: set by pending-`g` aborts, `NotYetBuilt` commands, `run_accounts_command` lookups; cleared by any key (1296) and hint clicks.
- ~30 free `*_hints()` fns (177–550): per-domain status-line legends (Settings pages, Bills, Budgets, dialogs, Transactions, filter, import). Per-domain content, held at the top of `shell.rs`.
- `settings_hints` (1456), `settings_cheat_sheet` (1473): Settings' legend and `?` group.
- `handle_hint_click` (1514–1530): hint-strip clicks → palette / search / help / rail toggle.
- Render's `page_status` (8961–9114): a `match self.nav.noun()` with guards producing `PageStatus { hints, right }` per noun, tab, open dialog and mode.
- Help overlay assembly in Render: `settings_cheat_sheet` + `documents_cheat_sheet` → `help_view::render`.

## 11. Command palette execution

- `record_history` (148) free fn; `command_history` field.
- `run_command` (7252–7368): records history, then a `match` on `CommandEffect`: `OpenDialog` (file explorer), `Navigate`, `OpenSettingsPage`, `CloseLedger`, `Accounts(verb)` → `run_accounts_command`, `Colour` (deferred), four Toast effects, `MergeTags`, `Import`, `Documents(verb)` → `run_documents_command`, `Budgets(verb)` (eight verbs handled inline), `NotYetBuilt`.
- `run_accounts_command` (7370–7416): Accounts name lookup and dialog opening (domain).
- `handle_empty_state_command_click` (7753–7768): runs a named command as the palette would, then applies any pending colour change.

## 12. Rail interaction

- `handle_toggle_rail` (7418): top bar button; same as the `b` key arm.
- `handle_rail_hover` (7429–7454): bumps `hover_generation`, spawns a `TOOLTIP_REVEAL_DELAY` (115) timer that sets `collapsed_rail_tooltip` if the generation still matches.
- `handle_rail_click` (7456–7466): `nav.set_noun` + `reset_view_scroll`.
- Rail badges are computed in Render (concern 15): bill attention, budget over-count, documents inbox count.

## 13. File explorer

- `explorer_start_dir` (140) free fn.
- `handle_explorer_entry_click` (7650), `handle_explorer_breadcrumb_click` (7668), `handle_explorer_filter_toggle` (7677), `handle_explorer_cancel` (7691), `handle_explorer_open` (7704), `confirm_explorer_open` (7717–7751): the last calls `nav.open_ledger()` and raises the ledger opened/created Toast.
- `handle_help_close` (7698) sits among these but is the help overlay's.

## 14. Deferred effects needing `App`

Key handlers run without an `App`/`Context`, so three fields queue an effect the Render key listener drains after dispatch (9142–9149):

- `pending_colour_change`: set by the Colour Theme grid and `CommandEffect::Colour`; applied via `ColourChange::apply(cx)` (also drained in `handle_empty_state_command_click`).
- `pending_budgets_export`: set by `x` on Budgets › History; drained into `export_budgets_history(cx)` (3734, which `cx.spawn`s the save dialog).
- `pending_file_action`: set by Documents; drained by `run_pending_file_action` in `documents_ui`.

## 15. Frame rendering

`impl Render for Shell` (7776–9665):

- 7778–7781: a side effect inside render: leaving Transactions drops `import`.
- 7782–7796: `content_opacity` (0.3 while palette, explorer or help is up).
- 7803–7955: chrome closures through an `Entity` handle: rail toggle, row hover, row click, three explorer row/breadcrumb/filter closures, hint, toast dismiss, toast hover, toast layer, toast-history close, help close, explorer cancel and open, empty-state command click. Settings unit and institution dialog closures (7904–7947) are interleaved.
- 7956–8112: Settings page closures (index click, units, price sources, institutions, sync, backup, tracing, display).
- 8113–8958: per-domain props and closures (accounts, categories, payees, document types, inventory, tags, bills, dashboard, budgets, import, transactions, documents). The file has 108 `let on_*` closure bindings, nearly all here.
- 8961–9114: `page_status` (concern 10).
- 9116–9661: the frame tree. Root `div` (theme colours and type scale, `track_focus`, documents drop target at 9126, key listener at 9130) → content column at `content_opacity`: `TopBar` (9157, context label from Settings page or import) → row of `PrimaryRail` (9172, with badges) + `ContextRail` (when the noun has context entities and a ledger is open) + `render_view` (9205) → `StatusLine` (9272: mode, flash, command echo, toast echo, page status, import mode label, hint click). Then overlays in z-order: palette, file explorer, filter popover, help, toast history, documents dialog, drop overlay, accounts dialogs (9312), bills dialog, six budgets dialogs/popovers, payees, tags, document types, inventory, categories and settings dialogs, and last `toast_layer` (9661).

Supporting items after the impl:

- `PageProps` (9667) and `SettingsPanelProps` (9695): prop bags carrying each page's props into `render_view`.
- `render_view` (9750–9875): the View zone switch on `Noun`, with the no-ledger empty state.
- `placeholder_view` (9877), `empty_state` (9906) and the `OnEmptyStateCommandClick` type (135).

## Other facts relevant to the split

- `Noun::` appears 76 times: 18 in key dispatch, mode tiers, movement and dialog host (1166–2206); 22 in per-domain methods; 2 in commands and rail; 25 in Render; 9 in free fns and tests.
- Four `cx.spawn` tasks: log feed (998), Toast clock (1040), Budgets export (3755), rail tooltip (7438).
- Existing `shell/` submodules: `documents_ui.rs` (1,624 lines), `document_types_ui.rs` (291), `inventory_ui.rs` (567) are per-domain `impl Shell` wiring; `snapshots/` (7 files) is the tests' read-back surface.
- Free domain helpers `delete_account` (10002) and `delete_category` (10026) sit at the bottom of the file (domain, #531's concern). Unit tests: 10068–10241.
- Every cross-cutting method is a `Shell` method reaching `self.<field>` directly; no concern has its own state struct today except what `NavState`, `Palette`, `FileExplorer`, `Toasts` and `LogView` already encapsulate.
