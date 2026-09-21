---
name: end-user-docs
description: Write or review end-user domain documentation (docs/.md, e.g. payees, tags, units, accounts). Use when creating a new domain page or editing an existing one, so structure, tone and the matching developer page stay consistent.
---

End-user domain documentation

Every domain (payees, tags, accounts, …) gets one end-user page, docs/<domain>.md, and one matching developer page, docs/development/<domain>.md. Follow the template below. Also follow docs/agents/markdown-style.md (ATX headings, one line per paragraph, - bullets), Australian English, and the vocabulary in CONTEXT.md.

Voice

* Extremely concise. Sacrifice grammar for concision.
* Conversational. Talk to the user (“you”), not about the system.
* No jargon or implementation details (crates, SQL, gRPC, IDs). That belongs on the developer page.
* Describe what the user sees and does. Show keys in backticks.
* Describe intent, not code. If a feature isn’t built yet, say so in the Heads up, and leave its requirement unticked.

Template

Copy this into docs/<domain>.md, replace <...>, delete any section that genuinely doesn’t apply (never leave placeholder text), and add the page to docs/SUMMARY.md (alphabetical, under the end-user list) and the developer page under Code Structure.

# <Domain>

<One to three lines: what the domain is in plain words, with a concrete example, and how Personal Ledger applies it.>

> **Heads up:** <Only while unfinished. What is built vs where it's headed.>

## Terminology

- **<Word 01>:** <What it means here.>
- **<Word 02>:** <What it means here.>

## <Domain> concept

<How Personal Ledger splits or models the domain, in a sentence.>

### <Area 1>

<What it is for.>

### <Area 2>

<What it is for.>

## Approach

<The typical workflow, in the order a user does it. Numbered steps for a sequence, bullets for options.>

## Related

<How this domain differs from and works with neighbours, one line each, linking their pages. Example: Payees vs Categories vs Tags.>

## Getting around

Press `g` then `<key>` to jump to <Domain> from anywhere. For the full set of keys, see [Getting around](navigation.md).

## Feature set and requirements

Intended features for <domain>. Ticked means built.

- [x] DOMAIN-001: <Feature or requirement>
- [ ] DOMAIN-002: <Feature or requirement>

## For developers

Curious how <domain> is structured in the codebase, or planning to change it? See the [<Domain> development documentation](development/<domain>.md).

Checklist before finishing

* Matching docs/development/<domain>.md exists (create a stub if not).
* Requirement IDs use the domain prefix, are unique, and ticks match reality—Cross-check with docs/product-requirements.md.
* Every key binding matches docs/navigation.md.
* Terms match CONTEXT.md; links resolve; page listed in docs/SUMMARY.md.
