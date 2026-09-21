# End-user page template

Copy into `docs/<domain>.md` and replace every `<...>`. Delete optional sections that are empty; never leave placeholder text. Sections marked (required) always stay.

```markdown
# <Domain>

<One to three lines: what the domain is in plain words, with a concrete example, and how Personal Ledger applies it.>

> **Heads up:** <Only while unfinished. What is built and what's still coming.>

## What you can do (required)

- <Task, in the user's words>
- <Task>

## Terminology

- **<Word>:** <What it means here.>

## <Domain> concept

<How Personal Ledger splits or models the domain, in a sentence or two.>

### <Area 1>

<What it is for.>

### <Area 2>

<What it is for.>

## Approach

<The typical workflow in the order you do it. Numbered steps for a sequence, bullets for options.>

## Worked example (required)

<One concrete walk-through with realistic numbers, start to finish.>

## Screens

<Optional. What each screen shows, in the order you meet them. Say where the terminal and desktop apps differ.>

## Rules to know

<Optional. Limits you'll run into, for example "transactions only move between accounts in the same unit".>

## Tips

<Optional. Short practical advice and common questions.>

## Scope (required)

<What this domain doesn't cover, one line each, pointing to where it lives.>

## Related

<How this domain differs from and works with its neighbours, one line each, linking their pages. Example: Payees vs Categories vs Tags.>

## Getting around

<Copy this domain's row from the table in getting-around.md: the `g` jump key in each app, and "not in the <app> yet" where it's missing. Link [Getting around](getting-around.md) for the full set.>

## Feature set and requirements

Intended features for <domain>. Ticked means built in at least one app; the tag says which.

- [x] <PREFIX>-001 (TUI, desktop): <Feature or requirement>
- [ ] <PREFIX>-002: <Feature or requirement>

## For developers

Curious how <domain> is structured in the codebase, or planning to change it? See the [<Domain> development documentation](development/<domain>.md).
```

## Notes

- Section order is the reading order: what it is, what you can do, the words, the model, how you use it, an example, then reference material, then requirements.
- Rules to know is where domain constraints go. They no longer live in the product requirements.
- Terminology may use `CONTEXT.md` terms, but only ones you define there.
