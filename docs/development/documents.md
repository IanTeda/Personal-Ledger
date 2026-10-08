# Documents (development)

End-user documentation: [Documents](../documents.md).

## Overview

Documents is a desktop-only surface built against in-memory stubs, from the handoff `docs/ux/desktop-mockups/04-documents/` (frames 4a Library and 4b Inbox) under the Desktop Documents Surface map ([#430](https://github.com/IanTeda/Personal-Ledger/issues/430)). Everything lives in `crates/bins/bin-desktop`. There is no `lib-core` type, no table and no TUI screen, and nothing persists except the last mode, scope and sort.

## Data model

Document Types are user-managed ledger data ([ADR-0031](../adr/0031-document-types-are-user-managed-ledger-data.md)): `src/documents/types.rs` holds the in-memory list (stable ids, order, Tracks date, Remind, Financial year flag, the fixed Default type Other), and `documents/mod.rs` reads it for the Type filter, the Add and Edit form, the Inbox's extracted type (matched by name) and the Key Date kind. The Settings page that edits it is `src/view/settings/documents.rs` with its dialogs in `src/shell/document_types_ui.rs`; tests are `settings_documents_keyboard.rs`, `settings_documents_mouse.rs` and `document_type_dialogs.rs`. Persistence and sync of Document Types, and of per-Document Key Dates, are not specified yet (map [#472](https://github.com/IanTeda/Personal-Ledger/issues/472)).

Not yet built. No migration exists. The rules the schema must carry are in `CONTEXT.md` (Document, Document Type, Financial Year, Key Date, Need Review, Document Link, Inbox, Extracted Facts, Suggested Link, Filing) and the doc comment at the top of `src/documents/mod.rs`:

- One entity, the `Document`, Unfiled or Filed. Only the user files one; gaining a Link never does.
- The Financial Year is derived from the document date (`FINANCIAL_YEAR_START_MONTH`) and never stored.
- A Link targets a whole Transaction (never a Split), an Inventory Item, an Account, a Payee or a Bill Plan (never a Bill Schedule entry).
- The Suggested Link is derived on every call and never stored.

## Domain types

None in `lib-core`. The pure model is `src/documents/mod.rs` (`gpui`-free and unit-tested): `Document`, `DocumentType`, `KeyDate`, `KeyDateBand` and `band` (60 days either side, inclusive), `DocumentLink`, `LibraryScope`, `LibrarySort`, `DocumentsMode`, `suggestion` (candidates within 7 days, amount or payee required, date signal within 3), `accept`, `accept_all_strong`, `FilingUndo` and `undo` (one level). The seed writes sample purchases into the stub Transactions, so it must run after `default_bills`.

## Persistence

`src/persistence.rs` stores the last mode, scope and sort only. Documents, Links and filings are in memory.

## UI

- `src/documents/form.rs` — `DocumentsDialog`, `DocumentForm`, `ImportForm`, `FactsForm` and their validation.
- `src/documents/picker.rs` — the link picker: `PickerState`, `Purpose` (link, file, follow) and the derived `rows`.
- `src/view/documents/` — `mod.rs` (page and Library), `inbox.rs` (4b), `dialogs.rs`, `model.rs`.
- `src/shell/documents_ui.rs` — `Shell`'s side: `handle_documents_key` (keys ahead of the global router), search keys, status-line hints, the `?` cheat-sheet group, link navigation and `DocumentsSnapshot` for tests.
- `src/navigation/command.rs` — `DocumentsVerb` and the `documents` / `documents inbox|library|accept-all|add|import` commands. `Noun::Documents` is bound to `g f`.
- `src/chrome/rail/` — the index rail and count badges.
- Messages in `i18n/en-US/documents.ftl`.

Link targets: a Transaction row, Settings › Accounts or Payees, Bills › Planner, and the Inventory placeholder. `Open` and `Show in folder` call the real OS when the path exists.

## Traceability

| Requirement | Code | Test |
| --- | --- | --- |
| DOC-001 | `view/documents/mod.rs`, `documents::LibraryScope` | `tests/documents_mouse.rs::index_rail_facets_narrow_the_library` |
| DOC-002 | `shell/documents_ui.rs`, `documents::LibrarySort` | `tests/documents_keyboard.rs::slash_searches_the_library`, `s_toggles_the_sort_in_the_library_only` |
| DOC-003 | `view/documents/mod.rs` | `tests/documents_mouse.rs::clicking_a_list_row_selects_it_and_focuses_the_list` |
| DOC-004 | `documents/picker.rs`, `shell/documents_ui.rs` | `tests/documents_keyboard.rs::enter_in_the_link_picker_toggles_a_link`, `shift_l_follows_a_single_link` |
| DOC-005 | `documents/form.rs`, `view/documents/dialogs.rs` | `tests/documents_keyboard.rs::a_opens_the_add_dialog_in_the_library` |
| DOC-006 | `documents/form.rs::ImportForm` | `tests/documents_keyboard.rs::shift_i_opens_the_import_dialog` |
| DOC-007 | `documents::suggestion`, `view/documents/inbox.rs` | unit tests in `src/documents/mod.rs` |
| DOC-008 | `documents::accept`, `accept_all_strong`, `undo` | `tests/documents_keyboard.rs::y_accepts_the_focused_strong_match_and_u_undoes_it`, `shift_y_opens_accept_all_and_esc_cancels` |
| DOC-009 | `documents/form.rs::FactsForm` | unit tests in `src/documents/form.rs` |
| DOC-010 | `documents::band`, `Document::band` | unit tests in `src/documents/mod.rs` |
| DOC-011 | `shell/documents_ui.rs::documents_open_selected` | none: it calls the OS, which the headless tests avoid |

## Decisions

- [ADR-0030](../adr/0030-desktop-ui-is-tested-in-process-through-gpui-test-support.md) for the headless test approach.
- Decision tickets #431–#435 on the map, summarised in its "Decisions so far".
- `CONTEXT.md` — the Document terms above, plus Transaction, Payee, Account, Bill Plan and Inventory Item.

## Open questions and known gaps

- **Drop overlay:** the full-window dashed overlay (`view/documents/drop_overlay.rs`) is stateless: gpui sets its active drag on a file drag entering the window, and the overlay turns opaque through `group_drag_over` at paint time. The headless harness can't read paint styles back, so it is checked live only.
- **Search highlighting** inside the preview, since the preview is a placeholder.
- **Missing-file state:** what "locate…" does without a file-picker decision.
- **Persistence and sync** of metadata and file bytes.
- **Watched folder** and **Notifications** from key dates are separate efforts.
