# Budgets are named, overlapping views over one Ledger

Budgets v2 (`docs/ux/desktop-mockups/14-budgets-v2/`) lets the user keep several Budgets and switch between them, each with a Method. The glossary said a Budget was a cap on one Category and "a Category has at most one Budget", which can't hold a household Budget and a personal one over the same Groceries Category. We're making a **Budget** a named container: a Method, one Unit locked at creation, a set of on-budget Accounts in that Unit, a default flag and an archived date. The per-Category cap inside a Category limits Budget is now a **Category Limit**. A Budget never owns Transactions. A Split counts in every Budget whose Accounts and Categories cover it, so Budgets may overlap. We chose views over partitions because partitioning the Ledger would force every Transaction to pick one Budget, and the same spending really does belong to more than one plan.

## Consequences

- A Category holds at most one Category Limit **per Budget**. ADR-0028's chains are keyed by (Budget, Category), and its "the Unit is stored on the record" is superseded: the Unit belongs to the Budget, and every Budget Amount in it shares that Unit.
- A Category limits Budget counts only Splits in Transactions on its on-budget Accounts. A new Account joins no Budget until the user adds it. An Account in another Unit can't be added.
- Exactly one Budget is the default. It is a field of the Budget, synced with the Ledger, and the Dashboard, the Budgets rail badge and first launch read it. The Budget last opened is a Client-scoped Preference (ADR-0027 precedent).
- A Budget is archived, never deleted. An archived Budget is still computed and viewable, but it is read-only, feeds no rail badge or Dashboard, and can be restored. The default can't be archived.
- The existing per-Category limits migrate to a Category limits Budget named **Personal spending**, on all Accounts in its Unit. Categories 5c reads and writes it by id, so renaming it changes nothing, and while it is archived 5c is read-only.
- Only the Category limits Method is built. Envelope, Percentage split and Project are named Methods deferred to their own maps, and FR.22 and §7 are reworded to match.
