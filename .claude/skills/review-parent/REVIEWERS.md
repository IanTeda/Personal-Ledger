# Reviewer briefs

The orchestrator sends one of these, with the placeholders filled in, as a reviewer subagent's whole prompt. Both reviewers are read-only.

---

## Spec

Review the commits under parent ticket #<parent> in this repository against their spec. You are read-only: do not edit files, commit, stash or write to the issue tracker.

- **Diff**: `git diff <BASE>...<HEAD>`. Commits: <commit list>.
- **Spec**: #<parent> and its children <children>. Read each with `gh issue view <n> --json title,body`. The `## Decisions` and `## Acceptance criteria` sections are the spec. Read gate comments for the decisions they record, but judge the code yourself.

Report:

- **Missing**: what the spec asked for that is absent or partial.
- **Wrong**: what looks implemented but works differently from the spec.
- **Unasked**: behaviour or public API in the diff that the spec didn't ask for.

Quote the spec line and give `file:line` for each finding. Under 400 words.

## Standards and quality

Review the commits under parent ticket #<parent> in this repository for standards and quality. You are read-only: do not edit files, commit, stash or write to the issue tracker.

- **Diff**: `git diff <BASE>...<HEAD>`. Commits: <commit list>.
- **Standards**: `CLAUDE.md` "Conventions", the repo skills `.claude/skills/{rust-style,unit-tests,ui-tests,tracing}/SKILL.md`, `docs/development/viper-architecture.md`, `docs/localisation-design.md` and `docs/agents/markdown-style.md`.

Look through four lenses:

- **Standards**: a breach of a documented rule. Cite the file and the rule. Skip what `mise run lint` enforces.
- **Smells** (Fowler): Mysterious Name, Duplicated Code, Feature Envy, Data Clumps, Primitive Obsession, Repeated Switches, Shotgun Surgery, Divergent Change, Speculative Generality, Message Chains, Middle Man, Refused Bequest. Each is a judgement call, and a documented repo rule overrides it.
- **Reuse and efficiency**: new code that re-implements an existing helper (name the helper), and wasted work on a hot path (key dispatch, render, per-row loops).
- **Altitude**: a fix layered as a special case where a change to the underlying mechanism would be deeper and simpler. Say whether that change is small (within the diff) or large.

For each finding give the lens, `file:line`, a one-line summary and its cost. Mark each one hard (a documented rule) or judgement. Under 500 words.
