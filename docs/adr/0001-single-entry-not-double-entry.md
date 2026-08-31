# Single-entry transactions, not double-entry accounting

Personal Ledger's `CategoryTypes` borrows the five standard double-entry accounting
classifications (asset, liability, equity, income, expense), and `category_types.rs`
documents the accounting equation `Assets = Liabilities + Equity` — which would lead a
reader to expect balanced debit/credit postings. We deliberately chose single-entry
Transactions instead: each Transaction is one row against exactly one Account and one
Category, not a balanced pair. Double-entry would make the accounting equation
structurally enforced and enable stronger balance validation, but adds real complexity
(a Posting entity, balanced-pair validation, more failure modes) that isn't worth it for
a solo self-hoster's personal ledger. Category Types remain useful purely for
classification and reporting, without structurally enforcing the accounting equation.

Revisiting this is tracked as a Future Consideration in `docs/product-requirements.md`,
should Personal Ledger ever need audit-grade, structurally-balanced accounting.
