# Clients reach Ledger data through a Ledger aggregate in lib-ledger

The domain lib crates are shallow. `lib-transactions` offers `TransactionService::edit(FnOnce(&mut Vec<Transaction>))` as its whole write interface, and `lib-bills` rules are free functions over six `&mut` slices. Rules that cross domains run in the Desktop's `app/*_ui.rs`: paying a Bill writes Transactions, deleting a Category re-points Splits and drops Category Limits, and deleting a Payee checks its Transactions. The domain crates also depend on each other (`lib-budgets` on seven others, `lib-documents` on seven), and the `lib_categories`/`lib_payees`/`lib_tags` → `lib_transactions` dependency left `default_transactions`, the pure parts of `transactions/query.rs`, and the tests that use them in the bin (#590). `TransactionsStore` and `BillsStore` emit no change event, which is the stale-copy risk named in ADR-0032's Consequences. Decided in #592 on the [Deepen the domain libs and finish the View Entities](https://github.com/IanTeda/Personal-Ledger/issues/588) map.

We're adding a `Ledger` aggregate in a new `lib-ledger` crate, and every read and write of Ledger data goes through it.

## Decision

- **Compose, don't merge.** Each domain crate keeps its model, its single-entity rules and its own seed. A rule that reads or writes another domain's rows moves up into `lib-ledger`.
- **Domain crates depend only on `lib-core`.** Ids and value types that cross domains (`CategoryId`, `PayeeId`, `AccountId`, `TransactionStatus`, …) move into `lib-core`. Entity structs stay in their domain crate. A domain crate never depends on another domain crate, so the dependency cycle cannot return.
- **`lib-ledger` holds orchestration, not types.** It depends on every domain crate, owns no entity types, and stays free of gpui and I/O so both Clients use it. It holds `Ledger`, the cross-domain commands and queries (Bill payment, Budget actuals, the Transactions query), the joined stub seed `Ledger::stub()` built from each domain's seed plus `default_transactions`, and every test that needs more than one domain.
- **`Ledger` owns every collection, and every write goes through it.** It offers named commands (`pay_bill`, `delete_payee`, `delete_category`, `rename_tag`, …), forwarding single-domain commands to the domain crate's rule. There is no `edit(FnOnce(&mut Vec<_>))` hatch on `Ledger` or on the domain crates. Reads are borrowed slices or query methods.
- **Rules are methods.** A single-entity rule is an inherent method on its type, in that type's domain crate (e.g. `BillScheduleEntry::status(&self, today)`, not `bills::status(entry, today)`). A cross-domain rule is a method on `Ledger`, because an inherent `impl` must live in its type's crate.
- **Clients depend on `lib-ledger` only.** The Desktop and TUI depend on `lib-ledger`, not the domain crates, so Cargo enforces that every write goes through `Ledger`.
- **Each command has its own refusal type.** A command returns `Result<Outcome, Refusal>` with a per-command enum, e.g. `DeletePayeeRefusal::HasTransactions { count }`, so each refusal maps to one Message and matches stay exhaustive.
- **One store Entity on the Desktop.** A single `LedgerStore` Entity wraps `Ledger` and emits `LedgerChanged { what }`, where `what` names the domains a command changed. The per-domain stores and their `Changed` events go, and Views filter on `what`.
- **This sharpens ADR-0032.** "A View reads ledger data through a service in a lib crate" becomes "a View reads and writes Ledger data through `Ledger`, held on the Desktop by `LedgerStore`".
- **In memory for now.** `Ledger` is synchronous and seeded by `Ledger::stub()`. Its persistence port is ADR-0039, and its ids become `RowID` newtypes (ADR-0040).

## Consequences

- Cross-domain rules leave `Shell`, and each one can be unit tested without gpui.
- A cross-domain write emits one event naming every domain it touched, so no View holds a stale copy.
- The domain crates lose most of their dependencies and some of their code: `lib-budgets` keeps single-Budget rules (overlap, effective dates), and actuals move up.
- `Ledger` is a large type. Its commands must stay deep (one call per user intent) or it becomes a façade over the old hatches.
- The TUI can adopt `Ledger` in place of its own models (#572).

## Considered Options

- **Fold the domain crates into `lib-core`, or merge them into one domain crate.** Rejected after [#628](https://github.com/IanTeda/Personal-Ledger/issues/628) ([research](https://github.com/IanTeda/Personal-Ledger/blob/research/domain-crates-vs-lib-core/docs/research/domain-crates-vs-lib-core.md)). Build times were about the same in a trial fold, so speed is not the reason. Cargo refuses a cycle between crates but does not check modules inside one crate, so folding would hide the coupling that caused #590 instead of removing it. Folding into `lib-core` would also rebuild every crate that uses `lib-core` (config, locale, database, the Sync Server) after any domain change.
- **Move the entity structs into `lib-core`.** Rejected: the domain crates would be left with rules over someone else's types, which is shallow in a new way, and `lib-core` would become the model crate.
- **Keep cross-domain rules in the owning domain crate, taking other domains' data as arguments.** Rejected: this is today's `&mut` slices of six collections, and no single place could emit one change event for a cross-domain write.
- **Keep per-domain stores.** Rejected: a cross-domain command changes several domains at once, and per-domain stores cannot announce that together.

## Amendment: commands are values, and a Bill Plan is a reference

The Ledger prototype ([#593](https://github.com/IanTeda/Personal-Ledger/issues/593)) kept the shape below, with two changes.

- **Commands are values as well as methods.** ADR-0037's `DialogForm::command()` returns a command, so each command is also a value type (`DeletePayee(id)`, `DeleteCategory(id)`) implementing `LedgerCommand { type Outcome; type Refusal; fn run(self, &mut Ledger) }`, and each value runs through its `Ledger` method. `LedgerStore::apply<C: LedgerCommand>` runs any of them, so there are no per-command store methods. `LedgerChanged { what }` is derived from the `LedgerChange`s a command records (ADR-0039), never declared by the command.
- **A Bill Plan is a reference.** A Payee that a Split or a Bill Plan names can only be deactivated, not deleted. Deleting a Category moves its Splits and Bill Plans to the Uncategorised Category of its type, clears it as any Payee's default and drops its Category Limits from every Budget, so no row is left naming a deleted id.
- **The migration runs through a temporary hatch.** Two copies of the data can't coexist, so `LedgerStore` takes the only copy first, the per-domain stores become read views over it, and their old writes go through a deprecated `Ledger::hatch()` that records no changes. Each domain's writes then move onto commands, and the hatch and the per-domain stores go last ([#637](https://github.com/IanTeda/Personal-Ledger/issues/637) to [#648](https://github.com/IanTeda/Personal-Ledger/issues/648)).

## Amendment: the Ledger is the service

Decided in the 2026-10-15 architecture review ([#666](https://github.com/IanTeda/Personal-Ledger/issues/666)). The "service" behind a Desktop page is the Ledger's commands and queries in `lib-ledger` (with single-entity rules in the domain crate), and its "repository" is `LedgerRepository` (ADR-0039). The Desktop gets no per-page service or store types: a page builds a command, runs it through `LedgerStore::apply`, and reads `LedgerStore`. New behaviour on a page is a new Ledger command, query or refusal.
