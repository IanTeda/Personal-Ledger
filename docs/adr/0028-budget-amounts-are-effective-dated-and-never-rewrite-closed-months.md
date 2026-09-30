# Budget Amounts are effective-dated and never rewrite closed months

A Budget's amount is held as a chain of effective-dated records per Category — an **Onward** Budget Amount (applies from its month until the next record), a **Month-only** Budget Amount (its month alone) and a **Stop** (no amount from its month) — with at most one record per Category per month, rather than the single `limit_amount` + `BudgetPeriod` + `is_active` row the `budgets` table and FR.23/FR.26 describe. We chose this so the Budgets History tab can show the amount that actually applied in each closed month, and so a Plan edit, a Stop or a Categories 5c edit never rewrites the past; a single mutable amount (or an `active` flag, which can't say *when* budgeting stopped) would silently restate history the moment the user changed it. The trade-off is that every month's amount is resolved by walking the chain rather than read from one field.

## Consequences

- The period is monthly only; FR.23's weekly, quarterly and yearly periods are superseded for the desktop surface. Irregular costs (a quarterly insurance bill) are Month-only amounts in the months they fall due.
- "No Budget Amount" (Unbudgeted) and an explicit 0.00 are different states.
- Only leaf Categories hold records; a parent is always a rollup. A leaf that gains a child gets an automatic Stop from the current month.
- ~~A Category has at most one Budget, in one Unit; the Unit is stored on the record, not part of its key.~~ Superseded by [ADR-0029](0029-budgets-are-named-overlapping-views-over-one-ledger.md): a chain is kept per (Budget, Category), that is per Category Limit, and the Unit belongs to the Budget.
- The `lib-database` `budgets` table is reshaped to match when Budgets persistence is wired; until then the desktop's stub model follows this ADR.
