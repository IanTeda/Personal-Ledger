# Implementer brief

The orchestrator sends this text, with `<n>` and `<parent>` filled in, as the implementer subagent's whole prompt.

---

Implement ticket #<n>, a child of #<parent>, on the current branch (`concept`). The run is unattended: no human will answer a question, so where you would ask, stop and report instead.

1. **Read the spec**: `gh issue view <n> --json number,title,body,labels,comments`. State its title. The body is the spec; read the files, commits and tickets it points to. Follow `CLAUDE.md`.
2. **Build it**: call the Skill tool with `mattpocock-skills:tdd`, at the seams the ticket names. Call `mattpocock-skills:diagnosing-bugs` when a failure resists a first look. Call the repo skills where the change reaches them: `rust-style` for all Rust, `tracing` for spans and logs, `unit-tests` for unit tests, `ui-tests` for any Desktop View or Settings page, `end-user-docs` for a domain page.
3. **Keep it green**: run each test file you touch as you go, then every acceptance criterion's test, then `mise run end-to-end-ui <view>` for each Desktop View you changed, then `mise run lint`. The full workspace suites run once at the parent's close, not here.
4. **Stop on a seam**: if the ticket leaves a decision open (a seam under "Seams to decide", a type or event it doesn't name, an acceptance criterion you can't meet as written), leave your changes uncommitted and report `seam`.
5. **Commit**: run CLAUDE.md's signing key check; if it fails, leave your changes uncommitted and report `signing`. Otherwise commit once, `<area>: <short description> (#<n>)`. Leave the issue's labels, comments and state alone; the orchestrator owns them.

Report back in exactly this shape and nothing else:

```text
outcome: done | seam | failed | signing
commit: <sha or none>
criteria:
- <each acceptance criterion>: <the test or check that verified it, or "not verified: why">
tests: <the commands you ran and their result>
tripwire paths: <each path you changed that docs/agents/ticket-labels.md "This project" lists, or none>
decisions: <any decision you made that the ticket didn't, or none>
stopped at: <for seam or failed: where and why>
```
