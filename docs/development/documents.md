# Documents (development)

End-user documentation: [Documents](../documents.md).

## Overview

Documents is a desktop-only surface built from the handoff `docs/ux/desktop-mockups/04-documents/` (frames 4a Library and 4b Inbox) under the Desktop Documents Surface map ([#430](https://github.com/IanTeda/Personal-Ledger/issues/430)). The model is `lib-documents`, the View and Dialogs live in `crates/bins/bin-desktop`, and the Documents and Document Types persist through `lib-database` ([#963](https://github.com/IanTeda/Personal-Ledger/issues/963)). There is no TUI screen.

## Data model

Document Types are user-managed ledger data ([ADR-0031](../adr/0031-document-types-are-user-managed-ledger-data.md)): `src/documents/types.rs` holds the in-memory list (stable ids, order, Tracks date, Remind, Financial year flag, the fixed Default type Other), and `documents/mod.rs` reads it for the Type filter, the Add and Edit form, the Inbox's extracted type (matched by name) and the Key Date kind. The Settings page that edits it is its own `DocumentsPage` Entity in `src/settings/documents/` (`mod.rs`, `render.rs`, `form.rs`, `apply.rs` and `dialogs/`, ADR-0037); `src/documents/types.rs` only re-exports the rows the destination shares; tests are `settings_documents_keyboard.rs`, `settings_documents_mouse.rs` and `document_type_dialogs.rs`. Sync of Document Types and Documents is not specified yet (map [#472](https://github.com/IanTeda/Personal-Ledger/issues/472)).

Three client tables, `document_types`, `documents` and `document_links`, carry the rules in `GLOSSARY.md` (Document, Document Type, Financial Year, Key Date, Need Review, Document Link, Inbox, Extracted Facts, Suggested Link, Filing) and the doc comment at the top of `src/documents/mod.rs`:

- One entity, the `Document`, Unfiled or Filed. Only the user files one; gaining a Link never does.
- The Financial Year is derived from the document date (`FINANCIAL_YEAR_START_MONTH`) and never stored.
- A Link targets a whole Transaction (never a Split), an Inventory Item, an Account, a Payee or a Bill Plan (never a Bill Schedule entry).
- The Suggested Link is derived on every call and never stored.

## Domain types

The Financial Year (`financial_year_start`) is `lib-core`'s, shared with Bills. The pure model is `lib-documents` (`gpui`-free and unit-tested), which reads the records a Link names through its own read ports (`DocumentTransactions`, `DocumentPayees`, `DocumentAccounts`, `DocumentBillPlans`, `DocumentItems`, read together as `LinkRecords`) and the disk through `DocumentFiles`, so it depends on no other feature crate: `Document`, `DocumentType`, `KeyDate`, `KeyDateBand` and `band` (60 days either side, inclusive), `DocumentLink`, `LibraryScope`, `LibrarySort`, `DocumentsMode`, `suggestion` (candidates within 7 days, amount or payee required, date signal within 3), `accept`, `accept_all_strong`, `FilingUndo` and `undo` (one level). The seed reads the stub's records as those rows, its sample purchases being the ones `lib-transactions`' stub books, and links the Camera House receipt and the filler receipts to them.

## Persistence

`lib-database`'s `DocumentsAdapter` (`adapter/documents.rs`, [#963](https://github.com/IanTeda/Personal-Ledger/issues/963)) holds the Documents and Document Types in memory and sends writes to the shared writer, answering `lib-documents`' `DocumentRepository`:

- `document_types` keeps its order in `position`; its name is unique ignoring case, one row is the Default type, and the Files count is a stored column for now.
- `documents` holds the Key Date and the Inbox's Intake (source, received date, Extracted Facts). A Document names its type by a deferred foreign key with no cascade, so a type removal moves its Documents in the same batch, and the file refuses deleting a type a Document still names.
- `document_links` holds one row per Link, its one target column set; a Link drops with the record it names (`ON DELETE CASCADE`), and the commit that deleted the record has the Desktop reload the Documents, so memory drops it too ([#964](https://github.com/IanTeda/Personal-Ledger/issues/964)). A write still skips a Link to a record the file no longer has. A Bill Plan has no delete yet, so its cascade is the schema's alone. The Remove location Dialog counts the Documents it unlinks from the Documents store (`LedgerStore::documents_linking_location`).
- A new Ledger is seeded with the stub's Document Types, Documents and Links last, after the domains they name (`seed::stub_documents`); an existing Ledger with no Document Types is given the nine default ones on open.
- The Inbox's skipped Documents are the session's (`DocumentsState::skipped`), never written; editing a Document's facts clears its skip.
- Every Documents writer is still a `LedgerCommand`, carried to the store by `documents::ports::forward`, with the `Ledger`'s copy refreshed after each write and on rollback (`LedgerStore::reload_documents`), until the Documents View and Settings › Documents move onto `lib-documents`' Interactors (`lib_documents::interactor`, [#965](https://github.com/IanTeda/Personal-Ledger/issues/965)) in #966 and #967. Each Interactor reads the Documents and Document Types from the repository, applies the rule `lib-documents` holds on a copy and writes the rows it changed, so a refusal writes nothing. Filing writes only Documents, so Accept and Accept all strong are Interactors here, not a cross-domain module; Import reaches the disk through `DocumentFiles`. The records a Link names are read from their own stores (`lib_database::StoredDocumentReads`).
- `src/persistence.rs` stores the last mode, scope and sort.

## UI

- `src/documents/form.rs` — `DocumentsDialog`, `DocumentForm`, `ImportForm`, `FactsForm` and their validation.
- `src/documents/picker.rs` — the link picker: `PickerState`, `Purpose` (link, file, follow) and the derived `rows`.
- `src/view/documents/` — `state.rs` the `DocumentsView` Entity, which renders the page and handles its own keys (ahead of the global router), Search, movement, `Esc`, filing, the OS file hand-off and link navigation (as `ViewEvent::Reveal`), and builds the dialogs (ADR-0037); `mod.rs` (page and Library), `inbox.rs` (4b), `hints.rs` the status-line legends, `snapshot.rs` what the UI tests read back, `dialogs.rs`.
- `src/app/documents_ui.rs` — `Shell`'s side: the Dialog host's Documents dialogs (drawn, confirmed, and the link picker's picks), the `documents <verb>` palette commands, dropped files and the persisted mode, scope and sort.
- `src/navigation/command.rs` — `DocumentsVerb` and the `documents` / `documents inbox|library|accept-all|add|import` commands. `Noun::Documents` is bound to `g f`.
- `src/chrome/rail/` — the index rail and count badges.
- Messages in `i18n/en-US/documents.ftl`.

Link targets: a Transaction row, Settings › Accounts or Payees, Bills › Planner, and the Inventory placeholder. `Open` and `Show in folder` call the real OS when the path exists.

## Traceability

| Requirement | Code | Test |
| --- | --- | --- |
| DOC-001 | `view/documents/mod.rs`, `documents::LibraryScope` | `tests/documents_mouse.rs::index_rail_facets_narrow_the_library` |
| DOC-002 | `view/documents/state.rs`, `documents::LibrarySort` | `tests/documents_keyboard.rs::slash_searches_the_library`, `s_toggles_the_sort_in_the_library_only` |
| DOC-003 | `view/documents/mod.rs` | `tests/documents_mouse.rs::clicking_a_list_row_selects_it_and_focuses_the_list` |
| DOC-004 | `documents/picker.rs`, `app/documents_ui.rs` | `tests/documents_keyboard.rs::enter_in_the_link_picker_toggles_a_link`, `shift_l_follows_a_single_link` |
| DOC-005 | `documents/form.rs`, `view/documents/dialogs.rs` | `tests/documents_keyboard.rs::a_opens_the_add_dialog_in_the_library` |
| DOC-006 | `documents/form.rs::ImportForm` | `tests/documents_keyboard.rs::shift_i_opens_the_import_dialog` |
| DOC-007 | `documents::suggestion`, `view/documents/inbox.rs` | unit tests in `src/documents/mod.rs` |
| DOC-008 | `documents::accept`, `accept_all_strong`, `undo` | `tests/documents_keyboard.rs::y_accepts_the_focused_strong_match_and_u_undoes_it`, `shift_y_opens_accept_all_and_esc_cancels` |
| DOC-009 | `documents/form.rs::FactsForm` | unit tests in `src/documents/form.rs` |
| DOC-010 | `documents::band`, `Document::band` | unit tests in `src/documents/mod.rs` |
| DOC-011 | `view/documents/state.rs::open_selected` | none: it calls the OS, which the headless tests avoid |

## Decisions

- [ADR-0030](../adr/0030-desktop-ui-is-tested-in-process-through-gpui-test-support.md) for the headless test approach.
- Decision tickets #431–#435 on the map, summarised in its "Decisions so far".
- `GLOSSARY.md` — the Document terms above, plus Transaction, Payee, Account, Bill Plan and Inventory Item.

## Open questions and known gaps

- **Drop overlay:** the full-window dashed overlay (`view/documents/drop_overlay.rs`) is stateless: gpui sets its active drag on a file drag entering the window, and the overlay turns opaque through `group_drag_over` at paint time. The headless harness can't read paint styles back, so it is checked live only.
- **Search highlighting** inside the preview, since the preview is a placeholder.
- **Missing-file state:** what "locate…" does without a file-picker decision.
- **Persistence and sync** of metadata and file bytes.
- **Watched folder** and **Notifications** from key dates are separate efforts.
