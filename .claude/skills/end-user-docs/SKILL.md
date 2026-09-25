---
name: end-user-docs
description: Write or review an end-user domain page (docs/<domain>.md, e.g. accounts, payees, tags, units, settings) and its matching developer page (docs/development/<domain>.md). Use when creating or editing a domain page so structure, voice, requirements and the developer page stay consistent. Not for guides such as getting-around, new-ledger-workflow, authentication, localisation or tracing.
---

# End-user domain documentation

Every domain gets one end-user page, `docs/<domain>.md`, and one developer page, `docs/development/<domain>.md`. The end-user page is for people using the apps. The developer page is where technical documentation for developers and Claude goes.

- End-user page: copy [end-user-template.md](end-user-template.md).
- Developer page: copy [developer-template.md](developer-template.md). A stub is fine when the domain isn't documented technically yet.

Also follow `docs/agents/markdown-style.md` (ATX headings, one line per paragraph, `-` bullets), Australian English, and the vocabulary in `CONTEXT.md`. Filenames are kebab-case.

## Scope

This skill covers domain pages only. Guides (`getting-around`, `new-ledger-workflow`, `authentication`, `localisation`, `tracing`) and `product-requirements.md` are out of scope; they follow the markdown style and the voice rules below, nothing else.

## Voice

- Talk to the user ("you"), not about the system. Conversational.
- Clarity beats concision. Shorten grammar only in bullets and requirement lines, never in prose.
- Describe what the user sees and does. Show keys in backticks.
- No implementation detail (crates, SQL, gRPC, IDs). That belongs on the developer page.
- `CONTEXT.md` terms are fine only when defined in the page's Terminology section.
- Describe intent, not code. If a feature isn't built, say so in the Heads up and leave its requirement unticked.

## Requirements

The domain page owns its requirements. `docs/product-requirements.md` only lists which requirement IDs are in scope for each cycle and app.

- Format: `- [x] ACC-003 (TUI, desktop): List accounts, filter by kind and active status, sort`.
- The tag names the apps where it is built. Ticked means built in at least one app. Unticked lines carry no tag.
- IDs use the domain prefix, are unique, and are never reused or renumbered.
- A dropped requirement stays, struck through with a reason: `- [ ] ~~ACC-014~~: dropped, <reason>`.
- No phase or priority field. Cycle scope lives in the PRD.
- Before ticking, find the screen or code path in the named app (the developer page's Traceability table gives the location). If you can't find it, leave it unticked. If you only edited docs and didn't check the code, say so in your reply.

### Prefix registry

A new domain needs a row here first.

| Domain | Prefix | Page |
| --- | --- | --- |
| Accounts | ACC | accounts.md |
| Balance checks | BAL | balance-checks.md |
| Bills | BIL | bills.md |
| Budgets | BUD | budgets.md |
| Categories | CAT | categories.md |
| Needs attention | NAT | needs-attention.md |
| Payees | PAY | payees.md |
| Reports | RPT | reports.md |
| Settings | SET | settings.md |
| Tags | TAG | tags.md |
| Transactions | TXN | transactions.md |
| Units | UNT | units.md |

## Reviewing a page

When asked to review, don't rewrite. Run the checklist, then report each deviation from the template, the voice rules and the requirements rules, grouped by section, with the fix you'd make. Rewrite only if asked. Existing pages that predate the template are brought into line when next touched, not swept in one go.

## Checklist before finishing

- Required sections are present: What you can do, Scope, Worked example. Only Heads up, Related and Getting around may be dropped, and only for a stated reason. No placeholder text.
- Matching `docs/development/<domain>.md` exists (a stub if nothing else), and is linked from the end-user page.
- Page is in `docs/SUMMARY.md`: end-user list alphabetical, developer page under Code Structure.
- Requirements follow the rules above, and their ticks match reality.
- Getting around is copied from the table in `docs/getting-around.md`, with both apps' keys. Never write keys from memory.
- Terms match `CONTEXT.md` and are defined in Terminology. Links resolve (`mdbook build`).
- The domain has a row in the prefix registry.
