# Clients reach Ledger data through a Ledger aggregate in lib-ledger

The domain lib crates are shallow. `lib-transactions` offers `TransactionService::edit(FnOnce(&mut Vec<Transaction>))` as its whole write interface, and `lib-bills` rules are free functions over six `&mut` slices. Rules that cross domains run in the Desktop's `app/*_ui.rs`: paying a Bill writes Transactions, deleting a Category re-points Splits and drops Category Limits, and deleting a Payee checks its Transactions. The domain crates also depend on each other (`lib-budgets` on seven others, `lib-documents` on seven), and the `lib_categories`/`lib_payees`/`lib_tags` → `lib_transactions` dependency left `default_transactions`, the pure parts of `transactions/query.rs`, and the tests that use them in the bin (#590). `TransactionsStore` and `BillsStore` emit no change event, which is the stale-copy risk named in ADR-0032's Consequences. Decided in #592 on the [Deepen the domain libs and finish the View Entities](https://github.com/IanTeda/Personal-Ledger/issues/588) map.

We're adding a `Ledger` aggregate in a new `lib-ledger` crate, and every read and write of Ledger data goes through it.

## Decision

- **Compose, don't merge.** Each domain crate keeps its model, its single-entity rules and its own seed. A rule that reads or writes another domain's rows moves up into `lib-ledger`.
- **Domain crates depend only on `lib-core`.** Ids and value types that cross domains (`CategoryId`, `PayeeId`, `AccountId`, `TransactionStatus`, …) move into `lib-core`. Entity structs stay in their domain crate. A domain crate never depends on another domain crate, so the dependency cycle cannot return.
- **`lib-ledger` holds orchestration, not types.** It depends on every domain crate, owns no entity types, and stays free of gpui and I/O so both Clients use it. It holds `Ledger`, the cross-domain commands and queries (Bill payment, Budget actuals, the Transactions query), the joined stub seed `Ledger::stub()` built from each domain's seed plus `default_transactions`, and every test that needs more than one domain.
- **`Ledger` owns every collection, and every write goes through it.** It offers named commands (`pay_bill`, `delete_payee`, `delete_category`, `rename_tag`, …), forwarding single-domain commands to the domain crate's rule. There is no `edit(FnOnce(&mut Vec<_>))` hatch on `Ledger` or on the domain crates. Reads are borrowed slices or query methods.
- **Each command has its own refusal type.** A command returns `Result<Outcome, Refusal>` with a per-command enum, e.g. `DeletePayeeRefusal::HasTransactions { count }`, so each refusal maps to one Message and matches stay exhaustive.
- **One store Entity on the Desktop.** A single `LedgerStore` Entity wraps `Ledger` and emits `LedgerChanged { what }`, where `what` names the domains a command changed. The per-domain stores and their `Changed` events go, and Views filter on `what`.
- **This sharpens ADR-0032.** "A View reads ledger data through a service in a lib crate" becomes "a View reads and writes Ledger data through `Ledger`, held on the Desktop by `LedgerStore`".
- **In memory for now.** `Ledger` is synchronous and seeded by `Ledger::stub()`. Its persistence port, and whether ids stay `u32` newtypes or become `RowID` (ADR-0009), are left to #597. Keeping the ids as `lib-core` newtypes makes that a change in one place.

## Consequences

- Cross-domain rules leave `Shell`, and each one can be unit tested without gpui.
- A cross-domain write emits one event naming every domain it touched, so no View holds a stale copy.
- The domain crates lose most of their dependencies and some of their code: `lib-budgets` keeps single-Budget rules (overlap, effective dates), and actuals move up.
- `Ledger` is a large type. Its commands must stay deep (one call per user intent) or it becomes a façade over the old hatches.
- The TUI can adopt `Ledger` in place of its own models (#572).

## Considered Options

- **Merge the domain crates into one.** Rejected: it gives up incremental builds and the one-crate-per-domain split, and the problem was where cross-domain rules ran, not the split.
- **Move the entity structs into `lib-core`.** Rejected: the domain crates would be left with rules over someone else's types, which is shallow in a new way, and `lib-core` would become the model crate.
- **Keep cross-domain rules in the owning domain crate, taking other domains' data as arguments.** Rejected: this is today's `&mut` slices of six collections, and no single place could emit one change event for a cross-domain write.
- **Keep per-domain stores.** Rejected: a cross-domain command changes several domains at once, and per-domain stores cannot announce that together.
