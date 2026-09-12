# Tag as an independent, many-to-many Transaction label, not a grouping over Category/Payee

`CONTEXT.md`'s Category entry previously listed `_Avoid_: Tag, group` — Tag had been weighed as just another word for Category and rejected. Revisiting it for "totals across Categories and Payees" surfaced a genuinely different need: a cross-cutting label that doesn't nest inside the fixed, required, exactly-one Category classification.

We modelled Tag as its own entity: a freeform, globally-unique (case-insensitive) label attached many-to-many directly to Transactions, independent of their Category and Payee — not a grouping layered over Category or Payee themselves. The rejected alternative (tagging Categories/Payees and rolling totals up through them) can't express a Transaction that should count toward a cross-cutting total despite sitting under an untagged Category or Payee (e.g. one stray "Japan Trip 2026" expense filed under "Miscellaneous"), which is exactly the case this feature exists for.

Tag gets the same `is_active` soft-delete lifecycle as Unit/Category/Account/Payee, but — unlike Payee — no rename-alias history: nothing auto-creates a Tag from parsed import text that would later need reconciling under a rename, so a plain in-place rename is enough. A Tag's total only sums across Transactions sharing one Unit; a Tag spanning mixed Units reports per-Unit subtotals rather than ever summing across them, extending the "never silently sum across Units" rule the Accounts design (`docs/ux/tui/accounts/README.md`) already established for type-grouped subtotals.

Scope for now: this ADR settles the domain model and vocabulary only — schema, UI, and reporting are deferred to a future build effort.
