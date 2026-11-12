# Implementer brief

The orchestrator sends this text, with `<n>` and `<parent>` filled in, as the implementer subagent's whole prompt.

---

Implement ticket #<n>, a child of #<parent>, on the current branch (`concept`). The run is unattended: no human will answer a question, so where you would ask, stop and report instead.

1. **Read the spec**: `gh issue view <n> --json number,title,body,labels,comments`. State its title. The body is the spec; read the files, commits and tickets it points to. Follow `CLAUDE.md`.
2. **Build it**: call the Skill tool with `mattpocock-skills:tdd`, at the seams the ticket names. Call `mattpocock-skills:diagnosing-bugs` when a failure resists a first look. Call the repo skills where the change reaches them: `rust-style` for all Rust, `tracing` for spans and logs, `unit-tests` for unit tests, `ui-tests` for any Desktop View or Settings page, `end-user-docs` for a domain page.
3. **Keep it green**: run each test file you touch as you go, then every acceptance criterion's test, then `mise run end-to-end-ui <view>` for each Desktop View you changed, then `mise run lint`. The full workspace suites run once at the parent's close, not here.
4. **Stop on a seam**: if the ticket leaves a decision open (a seam under "Seams to decide", a type or event it doesn't name, an acceptance criterion you can't meet as written), leave your changes uncommitted and report `seam`.
5. **Commit**: run CLAUDE.md's signing key check; if it fails, leave your changes uncommitted and report `signing`. Otherwise stage the paths you changed by name and commit once, `<area>: <short description> (#<n>)`. Another session may be editing the same tree, so a change you didn't make stays unstaged. Leave the issue's labels, comments and state alone; the orchestrator owns them.

A **decision** is a choice that changes what the spec promised, and each one gates the ticket for review: a public type, variant, function or field the ticket doesn't name; a behaviour a user could notice changing; a file changed outside the ticket's scope. Everything else you chose (names, private helpers, test layout, the order you built in) is a **note**.

Report back in exactly this shape and nothing else:

```text
outcome: done | seam | failed | signing
commit: <sha or none>
criteria:
- <each acceptance criterion>: <the test or check that verified it, or "not verified: why">
tests: <the commands you ran and their result>
tripwire paths: <each path you changed that docs/agents/ticket-labels.md "This project" lists, or none>
changed paths: <every path you changed, committed or not>
decisions: <each decision, as defined above, or none>
notes: <choices that are not decisions, or none>
stopped at: <for seam or failed: where and why>
```
