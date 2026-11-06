# Documents (development)

End-user documentation: [Documents](../documents.md).

## Overview

Documents is a desktop-only surface built from the handoff `docs/ux/desktop-mockups/04-documents/` (frames 4a Library and 4b Inbox) under the Desktop Documents Surface map ([#430](https://github.com/IanTeda/Personal-Ledger/issues/430)). The model is `lib-documents`, the View and Dialogs live in `crates/bins/bin-desktop`, and the Documents and Document Types persist through `lib-database` ([#963](https://github.com/IanTeda/Personal-Ledger/issues/963)). There is no TUI screen.

## Data model

Document Types are user-managed ledger data ([ADR-0031](../adr/0031-document-types-are-user-managed-ledger-data.md)): the `document_types` table holds the list (stable ids, order, Tracks date, Remind, Financial year flag, the fixed Default type Other), and `lib-documents` reads it for the Type filter, the Add and Edit form, the Inbox's extracted type (matched by name) and the Key Date kind. The Settings page that edits it runs on `lib-documents`' Presenters (#966): `DocumentTypesPagePresenter` and the Add/Edit, Remove and Other-notice Dialog Presenters (`lib_documents::presenter`), the page's keys (`lib_documents::keymap`, rebindable in `[keybindings]`) and its Messages (`lib_documents::msg`), with the Desktop's Views in `src/document_types/` (`view/page.rs`, `view/dialog.rs`, one Dialog Entity on the Dialog host); `src/documents/types.rs` only re-exports the rows the destination shares; tests are `settings_documents_keyboard.rs` and `settings_documents_mouse.rs`. Sync of Document Types and Documents is not specified yet (map [#472](https://github.com/IanTeda/Personal-Ledger/issues/472)).

Three client tables, `document_types`, `documents` and `document_links`, carry the rules in `GLOSSARY.md` (Document, Document Type, Financial Year, Key Date, Need Review, Document Link, Inbox, Extracted Facts, Suggested Link, Filing) and the doc comment at the top of `src/documents/mod.rs`:

- One entity, the `Document`, Unfiled or Filed. Only the user files one; gaining a Link never does.
- The Financial Year is derived from the document date (`FINANCIAL_YEAR_START_MONTH`) and never stored.
- A Link targets a whole Transaction (never a Split), an Inventory Item, an Account, a Payee or a Bill Plan (never a Bill Schedule entry).
- The Suggested Link is derived on every call and never stored.

## Domain types

The Financial Year (`financial_year_start`) is `lib-core`'s, shared with Bills. The pure model is `lib-documents` (`gpui`-free and unit-tested), which reads the records a Link names through its own read ports (`DocumentTransactions`, `DocumentPayees`, `DocumentAccounts`, `DocumentBillPlans`, `DocumentItems`, read together as `LinkRecords`) and the disk through `DocumentFiles`, so it depends on no other feature crate: `Document`, `DocumentType`, `KeyDate`, `KeyDateBand` and `band` (60 days either side, inclusive), `DocumentLink`, `LibraryScope`, `LibrarySort`, `DocumentsMode`, `suggestion` (candidates within 7 days, amount or payee required, date signal within 3), `accept`, `accept_all_strong`, `FilingUndo` and `undo` (one level). The seed reads the sample's records as those rows, its sample purchases being the ones `lib-transactions`' sample books, and links the Camera House receipt and the filler receipts to them.

## Persistence

`lib-database`'s `DocumentsAdapter` (`adapter/documents.rs`, [#963](https://github.com/IanTeda/Personal-Ledger/issues/963)) holds the Documents and Document Types in memory and sends writes to the shared writer, answering `lib-documents`' `DocumentRepository`:

- `document_types` keeps its order in `position`; its name is unique ignoring case, one row is the Default type, and the Files count is a stored column for now.
- `documents` holds the Key Date and the Inbox's Intake (source, received date, Extracted Facts). A Document names its type by a deferred foreign key with no cascade, so a type removal moves its Documents in the same batch, and the file refuses deleting a type a Document still names.
- `document_links` holds one row per Link, its one target column set; a Link drops with the record it names (`ON DELETE CASCADE`), and the commit that deleted the record has the Desktop reload the Documents, so memory drops it too ([#964](https://github.com/IanTeda/Personal-Ledger/issues/964)). A write still skips a Link to a record the file no longer has. A Bill Plan has no delete yet, so its cascade is the schema's alone. The Remove location Dialog counts the Documents it unlinks from the Documents store (`lib-database`'s `StoredInventoryReads`).
- The Sample Ledger writes the sample's Document Types, Documents and Links last, after the domains they name (`seed::sample_documents`). A new Ledger gets the nine default Document Types from `NewLedger`, and `open_storage` refuses a Ledger with none as incomplete rather than giving it them.
- The Inbox's skipped Documents are the session's, held by the page's Presenter (`DocumentsPagePresenter::skipped`) beside the one-level `FilingUndo`, and never written; editing a Document's facts clears its skip.
- Delete ([#959](https://github.com/IanTeda/Personal-Ledger/issues/959)) is single-domain, since Document Links are Documents' own rows: `DeleteDocument` calls `DocumentRepository::delete_document`, which `DocumentsAdapter` sends to the writer as one change deleting the Document's `document_links` rows and then its `documents` row. It never touches the file or the linked records. `DeleteDocumentPresenter` is a plain confirm (no typed-back name) that raises its own Toast and closes with `DocumentsOutcome::Deleted(id)`; a Document already gone is refused with a Message and the Dialog stays open. The page then trims the pending `FilingUndo` (`FilingUndo::without`, clearing it when no entries are left), drops the id from its skipped set and keeps the selection's position, which is the next row, or the previous one at the end. The `d` key is `lib_viper`'s flat `DELETE` command, listed as `documents delete`; the Delete button is last in the detail footer in both modes and wraps to its own row where the pane is too narrow.
- The Document Types page writes the store itself through `lib-documents`' Interactors (`lib_documents::interactor`, [#965](https://github.com/IanTeda/Personal-Ledger/issues/965), [#966](https://github.com/IanTeda/Personal-Ledger/issues/966)): `AddDocumentType`, `EditDocumentType`, `RemoveDocumentType` and `MoveDocumentType`, in the one commit of the input that sent the event, announcing `Domain::Documents` and `Domain::DocumentTypes` after each so the Documents page redraws (`document_types::ports::announce`). The Documents page writes through them too ([#967](https://github.com/IanTeda/Personal-Ledger/issues/967)): Accept, filing by hand, a Link's ×, Undo and a drop (`Accept`, `FileByHand`, `ToggleLink`, `UndoFiling`, `ImportDocuments`), and so do its Dialogs ([#968](https://github.com/IanTeda/Personal-Ledger/issues/968)): Add, Edit, Import, Edit facts, Accept all and the link picker (`AddDocument`, `EditDocument`, `ImportDocuments`, `SetFacts`, `AcceptAllStrong`, `ToggleLink`, `FileByHand`). No read copy holds the Documents: a rollback reloads the store and announces both domains (`shell/router/write_failures.rs`), and no store-wide change list names a Document or Document Type. Each Interactor reads the Documents and Document Types from the repository, applies the rule `lib-documents` holds on a copy and writes the rows it changed, so a refusal writes nothing. Filing writes only Documents, so Accept and Accept all strong are Interactors here, not a cross-domain module; Import reaches the disk through `DocumentFiles`. The records a Link names are read from their own stores (`lib_database::StoredDocumentReads`).
- `src/persistence.rs` stores the last mode, scope and sort.

## UI

- `lib_documents::presenter` — the page's Presenter (`DocumentsPagePresenter`, [#967](https://github.com/IanTeda/Personal-Ledger/issues/967)): mode, scope, sort, Search, focus zone, selections, Skip and the one-level Undo, the Library's and the Inbox's view models (`presenter::library`, `presenter::inbox`), its legend and `?` group. Each Dialog has its own Presenter beside it ([#968](https://github.com/IanTeda/Personal-Ledger/issues/968)): `DocumentFormPresenter` (Add and Edit), `ImportPresenter`, `FactsFormPresenter`, `AcceptAllPresenter` and `LinkPickerPresenter` (`Purpose`: link, file, follow), built on the page's `DialogPorts`, each closing through `lib_viper::dialog::DialogRouter<DocumentsOutcome>` with the `DocumentsOutcome` it asks of the page. It reads today from the Clock, so a Key Date's band moves as the day does, and drops a Financial Year or Type scope that has left the rail. Its keys are `lib_documents::keymap::DOCUMENTS_PAGE_BINDINGS`, rebindable in `[keybindings]`, and the palette lists `documents inbox|library|add|import|delete|accept-all` from them.
- `src/documents/view/` — `page.rs` the `DocumentsPage` Entity holding the Presenter: it offers each key the keymap binds when the Presenter `takes` it, sends clicks as events, maps the Search onto the Shell's Search mode and draws the view model (`mod.rs` the page and Library, `inbox.rs` 4b); `snapshot.rs` what the UI tests read back; `dialog.rs` the `DocumentsDialog` Entity the Dialog host holds, one View per Presenter in `dialog/` (`document_form.rs`, `import.rs`, `facts.rs`, `accept_all.rs`, `picker.rs`), handing its outcome back through `DocumentsPage::dialog_landed`; `drop_overlay.rs` the window's drop overlay, whose drop the page takes as an Import event.
- `src/documents/router.rs` — `DocumentsRouter` for the Desktop: a Link revealed through the Transactions entry point or a `ViewEvent::Reveal` (an Inventory Item shows the Inventory), each Dialog built on the page's `DialogPorts` and hosted as an Entity, files handed to the OS, Notices queued.
- `src/shell/router.rs` — `Root`'s side: the persisted mode, scope and sort, and a window drop handed to the page. `Noun::Documents` is bound to `g f`.
- `src/chrome/rail/` — the index rail and count badges.
- The page's and its Dialogs' Messages are `lib-documents`' (`lib_documents::msg`); the Desktop's `i18n/en-US/documents.ftl` keeps only the drop overlay's.

Link targets: a Transaction row, Settings › Accounts or Payees, Bills › Planner, and the Inventory placeholder. `Open` and `Show in folder` call the real OS when the path exists.

## Traceability

| Requirement | Code | Test |
| --- | --- | --- |
| DOC-001 | `documents/view/mod.rs`, `documents::LibraryScope` | `tests/documents_mouse.rs::index_rail_facets_narrow_the_library` |
| DOC-002 | `lib_documents::presenter::DocumentsPagePresenter`, `documents::LibrarySort` | `tests/documents_keyboard.rs::slash_searches_the_library`, `s_toggles_the_sort_in_the_library_only` |
| DOC-003 | `documents/view/mod.rs` | `tests/documents_mouse.rs::clicking_a_list_row_selects_it_and_focuses_the_list` |
| DOC-004 | `lib_documents::presenter::LinkPickerPresenter`, `documents/view/dialog/picker.rs`, `documents/router.rs` | `tests/documents_keyboard.rs::enter_in_the_link_picker_toggles_a_link`, `shift_l_follows_a_single_link`, `tests/documents_mouse.rs::clicking_a_follow_picker_row_goes_to_that_record` |
| DOC-005 | `lib_documents::presenter::DocumentFormPresenter`, `documents/view/dialog/document_form.rs` | `tests/documents_keyboard.rs::add_refuses_a_missing_file_an_unsupported_kind_and_one_already_in_the_library`, `e_edits_the_selected_documents_title` |
| DOC-006 | `lib_documents::presenter::ImportPresenter` | `tests/documents_keyboard.rs::an_import_that_takes_nothing_stays_open_and_says_why` |
| DOC-007 | `documents::suggestion`, `lib_documents::presenter::inbox` | unit tests in `src/documents/mod.rs` |
| DOC-008 | `lib_documents::interactor::{Accept, AcceptAllStrong, UndoFiling}` | `tests/documents_keyboard.rs::y_accepts_the_focused_strong_match_and_u_undoes_it`, `shift_y_opens_accept_all_and_esc_cancels`, `a_skip_is_gone_after_a_restart` |
| DOC-018 | `lib_documents::interactor::DeleteDocument`, `lib_documents::presenter::DeleteDocumentPresenter`, `DocumentRepository::delete_document`, `documents/view/dialog/delete_document.rs` | `lib-documents` unit tests, `lib-database`'s `deleting_a_document_removes_it_and_its_links_and_keeps_the_records`, `tests/documents_keyboard.rs::d_in_the_library_confirms_then_deletes_the_document_and_esc_keeps_it`, `d_in_the_inbox_confirms_then_deletes_and_u_has_nothing_to_undo_for_it`, `tests/documents_mouse.rs::the_library_detail_deletes_after_a_confirm_and_cancel_keeps_the_document`, `the_inbox_detail_deletes_after_a_confirm_and_cancel_keeps_the_document` |
| DOC-009 | `lib_documents::presenter::FactsFormPresenter` | unit tests in `lib-documents`' `presenter/facts_form.rs`, `tests/documents_keyboard.rs::e_in_the_inbox_edits_the_extracted_facts_and_refuses_a_bad_total` |
| DOC-010 | `documents::band`, `Document::band` | unit tests in `lib-documents`, `tests/documents_keyboard.rs::the_month_turning_moves_a_key_date_into_need_review` |
| DOC-011 | `documents/router.rs::open_file` | none: it calls the OS, which the headless tests avoid |

## Decisions

- [ADR-0030](../adr/0030-desktop-ui-is-tested-in-process-through-gpui-test-support.md) for the headless test approach.
- Decision tickets #431–#435 on the map, summarised in its "Decisions so far".
- `GLOSSARY.md` — the Document terms above, plus Transaction, Payee, Account, Bill Plan and Inventory Item.

## Open questions and known gaps

- **Drop overlay:** the full-window dashed overlay (`documents/view/drop_overlay.rs`) is stateless: gpui sets its active drag on a file drag entering the window, and the overlay turns opaque through `group_drag_over` at paint time. The headless harness can't read paint styles back, so it is checked live only.
- **Search highlighting** inside the preview, since the preview is a placeholder.
- **Missing-file state:** what "locate…" does without a file-picker decision.
- **Sync** of metadata and file bytes; the metadata persists locally.
- **Files count:** stored on `document_types` and able to drift from the Documents that name the type ([#971](https://github.com/IanTeda/Personal-Ledger/issues/971)).
- **Watched folder** and **Notifications** from key dates are separate efforts.
