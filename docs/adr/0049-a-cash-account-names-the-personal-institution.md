---
status: accepted
---

# A Cash Account names the Personal Institution, an ordinary Institution

ADR-0017 made every Account's Institution link mandatory and gave Cash, which has no bank, a system-seeded placeholder row, rejecting a real Institution on a Cash Account as meaningless for physical cash. The code then went further than the ADR: the placeholder (`NO_INSTITUTION`) has no row, a Cash Account stores a NULL `institution_id`, and the Account form locks the Institution while Type is Cash. Deciding Institution edit and delete ([#869](https://github.com/IanTeda/Personal-Ledger/issues/869)) raised the question again, since a placeholder needs special cases on both. The cash does have a holder, the person keeping the Ledger, so we decided Cash names a real Institution that stands for them: the Personal Institution (`GLOSSARY.md`).

## Decision

- **The Personal Institution is an ordinary Institution.** It is listed, edited and deleted under the same rules as any other, its Products default to "cash" and its name to the "Personal" Message, which the person may rename to their own name. Nothing about it is special after creation.
- **Every new Ledger starts with one.** `open_storage` writes it when it creates a Ledger file, so both Clients get it, with a fixed id in `lib-core` (`InstitutionId::SEED_PERSONAL`) so two devices' Ledgers agree once they sync, and its name from the Locale the Ledger was made in. The stub seed lists it first and its Cash Account names it.
- **Cash is not special to the Institution link.** `NO_INSTITUTION`, the NULL mapping and the Account form's Cash lock go, and `accounts.institution_id` is `NOT NULL`. The Account form defaults a Cash Account's Institution to the Personal Institution, but a Cash Account may name any Institution, which keeps Transaction Account Type and the Institution link independent, as the glossary has them.

## Considered Options

- **Keep the placeholder with no row (today's code).** Rejected: every Institution and Account rule needs a case for an id that isn't a row, the foreign key can't be `NOT NULL`, and the placeholder's name has to be rendered by the Client rather than stored.
- **ADR-0017's seeded placeholder row, undeletable and unlisted.** Rejected: a row the person can't see or manage still needs special cases in listing, edit and delete, for a link that means nothing.

## Consequences

- If the person deletes the Personal Institution once nothing names it, a new Cash Account just names another Institution; nothing recreates it.
- A Ledger file made before this has NULL `institution_id`s and no Personal Institution. While the schema is in concept the dev database is reset rather than backfilled.
