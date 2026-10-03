# Document Types are user-managed Ledger data

Settings v4 adds a Document types page (`docs/ux/desktop/design_handoff_settings_v4/`, frame 16p) where the user adds, edits, removes and reorders the kinds of Document, and the Documents Type filter lists them in that order. The glossary said a Document Type was one of a fixed eight, not user-editable, and the code models it as an enum with `Receipt` as a hard-coded fallback. We're making a **Document Type** user-managed Ledger data, synced as Change Sets like Tags and Payees, and dropping any fixed kind behind it. We chose pure user types over mapping each user type onto a hidden built-in kind because two parallel concepts would leak into every form, filter and extraction rule, and a household's kinds of paperwork genuinely differ.

## Consequences

- A new Ledger is seeded with nine types: Receipts, Statements, Tax, Insurance, Warranties & manuals, Contracts, Identity, Bills and Other (the Default, which cannot be removed). 16p draws seven; Bills is kept because the Documents handoff still shows it and a rates notice is not a Statement. Apart from Other, seeds have no special status once created.
- A type has one name, shown everywhere. Seeds are plural, so Documents rows read "RECEIPTS" rather than the handoff's singular "RECEIPT" — a deliberate deviation.
- Extracted Facts can only suggest one of the Ledger's own types, matched by name; with no match the extracted type is absent.
- Nothing hard-codes a particular type. Where code fell back to Receipt (Accept with no extracted type, the Add Document form default), it falls back to the Default Document Type, **Other** (amended by the removal decision: it replaces "the first type in the user's order").
- Removal, reorder, rename and the per-type attributes (Tracks date, Remind, Financial year) are decided on the Desktop Settings Document Types map (page: Settings › Documents), not here.
