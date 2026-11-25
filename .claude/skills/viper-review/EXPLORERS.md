# Explorer briefs

The orchestrator sends one of these, with the placeholders filled in, as an explorer subagent's whole prompt. Both explorers are read-only.

---

## VIPER

Read this repository's code for changes that would make it more VIPER. You are read-only: do not edit files, commit, stash or write to the issue tracker.

- **Scope**: <scope>. Range: <BASE>..<HEAD> (`git diff --stat <BASE>...<HEAD>` when a `BASE` is given; otherwise the whole scope).
- **Rules**: `docs/development/viper-architecture.md`, its Rules, Persistence and During the transition sections; the ADRs it cites; `GLOSSARY.md` for names. `graphify query "<question>"` finds callers and seams faster than grep.
- **Owned work**: <open tickets>. A change one of them owns is not a finding.

Hunt for **drift**: code that sits in the wrong layer or reaches across a seam the rules draw.

- **Layer**: domain logic in a View or a Desktop module that belongs in a feature crate's Interactor or Presenter; a Presenter that writes, logs or holds toolkit types; an Interactor reaching a store rather than a port; a Router decision made in a View; key handling that bypasses `input.rs`' `send` or the module keymap.
- **Dependencies**: a feature crate on another feature crate, a toolkit, `lib-database` or I/O; a shared type copied into a feature crate rather than living in `lib-core` or `lib-viper`.
- **Ports**: a read port answered by the wrong store, a multi-domain write outside a cross-domain module, a port with one adapter and no fake.
- **Transition**: a domain or View still on the old path that the During the transition section lists, and what migrating it would take.
- **Asymmetry**: one module doing a thing a different way from the rest (the next domain to follow `lib-units` and doesn't).

For each finding give the rule it breaks (quote it), `file:line`, the layer it should move to, and the decisions it would still need (its fog). Mark each one hard (a rule says so) or judgement. Under 600 words, most important first.

## Depth

Read this repository's code for modules to deepen and simplify. You are read-only: do not edit files, commit, stash or write to the issue tracker.

- **Scope**: <scope>. Range: <BASE>..<HEAD> (`git diff --stat <BASE>...<HEAD>` when a `BASE` is given; otherwise the whole scope).
- **Vocabulary**: load the `codebase-design` skill and use its terms exactly (**module**, **interface**, **depth**, **seam**, **adapter**, **leverage**, **locality**). `docs/development/viper-architecture.md` and the ADRs in `docs/adr/` record decisions; flag a conflict only when the friction is real. `GLOSSARY.md` names the domain. `graphify query "<question>"` finds callers and seams.
- **Owned work**: <open tickets>. A change one of them owns is not a finding.

Look through these lenses:

- **Depth**: shallow modules whose interface is nearly as large as their body; one concept spread across many small modules; tight coupling leaking across a seam. Apply the deletion test: would deleting it concentrate complexity, or just move it?
- **Testability**: logic only testable through the UI that a Presenter or Interactor would make testable through its interface; pure helpers extracted for tests while the bugs hide in their callers.
- **Reuse**: code that re-implements an existing helper (name it), or the same shape repeated across domains that `lib-viper`, `lib-list-cursor` or `lib-core` should hold once.
- **Simplification and efficiency**: needless indirection, dead parameters, special cases; wasted work on a hot path (key dispatch, render, per-row loops).
- **Altitude**: a special case layered on a mechanism, where changing the mechanism would be deeper and simpler.

For each finding give the lens, `file:line`, a one-line summary, what deepening it buys (locality, leverage, tests) and the decisions it would still need (its fog). Under 600 words, most important first.
