# Developer page template

Copy into `docs/development/<domain>.md`. This page is read by developers and by Claude working in the domain, so be precise: real paths, real names, no marketing. Add it under Code Structure in `docs/SUMMARY.md`.

## Stub

Use this when the domain isn't documented technically yet. It still gets a Code Structure entry.

```markdown
# <Domain> (development)

Not yet written. End-user documentation: [<Domain>](../<domain>.md).

## Overview

<One paragraph, and which crates own the domain.>

## Data model

Not yet written.

## Domain types

Not yet written.

## Persistence

Not yet written.

## UI

Not yet written.

## Decisions

Not yet written.
```

## Full template

```markdown
# <Domain> (development)

End-user documentation: [<Domain>](../<domain>.md).

## Overview

<One paragraph on what the domain is in the code, and which crates own it.>

## Data model

<Tables and columns. Then the invariants the schema doesn't enforce.>

## Domain types

<Types in `lib-core` and the rules they carry.>

## Persistence

<`lib-database` files, queries and migrations for this domain.>

## RPC and proto

<Services and messages, if any. Delete if none.>

## UI

<TUI and desktop modules, and how they map to the screens on the end-user page.>

## Traceability

Required for every ticked requirement. Stubs and planned work may omit it.

| Requirement | Code | Test |
| --- | --- | --- |
| <PREFIX>-001 | `<path>::<item>` | `<path>::<test>` |

## Decisions

<Linked ADRs and the `CONTEXT.md` terms this domain relies on.>

## Open questions and known gaps

<What isn't decided or built, so nobody assumes it is.>
```

Delete RPC and proto when the domain has none. Keep every other section, even if only "Not yet written."
