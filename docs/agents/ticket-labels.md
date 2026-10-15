# Ticket labels

Every GitHub issue carries a `category:` label naming its intent, every open issue a `priority:` rank and a `flow:` label naming the Flow it goes down next, and a `state:` label while something is happening to it, so the issue list shows what to work on next. This page is the one place the rules for choosing and changing them live; `CLAUDE.md`, the README and `docs/agents/triage-labels.md` point here. What each label means is defined in `docs/agents/triage-labels.md`, and the `flow:` and `wayfinder:` labels in `docs/agents/flow-labels.md`. Everything specific to this repo is in [This project](#this-project), so porting the system means rewriting that section only.

## Labels

On finishing, `triage:ready-for-agent` is removed and the `agent:*` label is set to the model and effort that actually actioned the ticket. Other labels are untouched.

- **Category from creation:** a ticket gets its `category:` when it is created, usually in the backlog, whatever its `wayfinder:` type. A ticket charted from a parent inherits the parent's category unless its intent clearly differs, such as a bug found while charting an enhancement.
- **The Now cap:** no more than 5 open tickets are `priority:10-now`, whatever their state. A sixth means moving one down to `priority:20-next`.
- **Blocked by a ticket:** a ticket blocked only by another ticket keeps its rank and gets no state. GitHub's native dependency shows the block, and the ticket rises once the blocker closes.
- **Backlog is for charting:** Claude takes a `priority:99-backlog` ticket only to grill or chart it, never to implement it.

## Recording label decisions

A label decision (the `category:`, the `priority:` rank and the `agent:` label) is recorded in a `## Labels` section at the end of the ticket body, one line per label with its reason, and updated in place when the decision changes. The body is the spec, so the reason travels with it. A `state:` or `flow:` change is an event rather than a decision: it is recorded as a comment naming the step that caused it, as [When labels change](#when-labels-change) says, and never as a `## Labels` line.

```markdown
## Labels

- `category:architecture`: moves writes behind Ledger commands, no behaviour change.
- `priority:20-next`: on the #588 route, nothing compounds on it yet.
- `agent:sonnet-low`: repeats the pattern set in <commit>.
```

A label you set by hand is final, except `agent:*`. Claude records every label it sets in the `## Labels` section, so a label missing from it is yours.

The `agent:*` label names the model and effort that action the ticket. Before a ticket is actioned with a model other than its label, Claude asks the user and waits for a yes. On closing, Claude sets the label to the model and effort that actually ran it and updates its `## Labels` line. If Claude cannot establish the effort, it asks the user rather than guessing.

## Ranking

Rank by the cost of delay, weighing these in order:

1. **Compounding:** does waiting make this ticket or others more expensive? Architecture other work builds on, a guardrail that stops a repeating mistake, build or test speed that every ticket pays for. Features built on bad architecture are rework. Check `category:architecture`, `category:tooling` and a flaky-test `category:bug` here first; the category flags a candidate, it doesn't set the rank.
2. **Leverage:** does finishing it unblock or speed up other tickets?
3. **Route:** is it on an open Wayfinder map's route?
4. **Effort:** only to break ties: the smaller ticket goes first (an easy win). Size never ranks a ticket down.

The result:

- High compounding or high leverage → `10-now`.
- Specified, with neither → `20-next`.
- Not yet specified → `99-backlog`.
- A feature that would sit on an open architecture ticket waits behind it. Say so in the feature's ticket.

## Closing or review

On finishing, Claude either closes the ticket itself or gates it for your review. The ticket's `agent:*` label sets the starting risk; the finished diff can escalate it, never downgrade it.

- **Close:** `agent:haiku-medium` or `agent:sonnet-low`, no tripwire fired, and you didn't ask to review first. The closing comment says why Claude closed it without review: the agent label, no tripwire, every acceptance criterion verified.
- **Gate:** `agent:opus-*`, any tripwire, or you asked to review first. Claude swaps the state for `state:30-needs-review`, comments with the commit and each tripwire that fired, and opens the run's final message with `Needs human review: #<n>` and those tripwires. The ticket stays open.

Tripwires, checked against the commit's diff and the run; the paths for each are in [This project](#this-project):

1. **Data:** schema migrations or cached query metadata.
2. **Shared foundations:** the crates or protocols every other part builds on.
3. **Security surface:** authentication, tokens or user credentials.
4. **Build and guardrails:** workspace manifests, dependency policy, toolchain pins, hooks or CI workflows.
5. **Decisions:** a new or changed ADR, or a glossary term.
6. **The run:** an acceptance criterion not verified, a test skipped, or a decision the ticket didn't make.

A gated ticket closes when you close it, or when you tell Claude it's approved.

## When labels change

| When                                   | Claude does                                                                                                       |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Creating a ticket                      | Gives it a `category:` (inherited from its parent if it has one) and a `priority:` rank, each with its reason in the `## Labels` section, and a `flow:` label: `flow:triage` for a raw issue someone else filed, `flow:grilling` for your own idea, or the Flow the creating step names below |
| Re-categorising or re-ranking a ticket | Changes the label and updates its line in the `## Labels` section                                                 |
| Claiming a ticket (assigning it)       | Adds `state:10-in-progress`                                                                                       |
| About to action a ticket with a model other than its `agent:*` label | Asks the user, and waits for a yes before starting                                                 |
| Finishing a ticket                     | In the same session as the commit: removes `triage:ready-for-agent`, sets its `agent:<model>-<effort>` label to the model and effort that actioned it and updates its `## Labels` line, then closes or gates it by [Closing or review](#closing-or-review). Closing comments with the commit and why no review was needed, removes its `priority:` rank, `flow:` and `state:` labels, adds `state:90-done`, and closes it. Other labels stay |
| You approve a `state:30-needs-review` ticket | Comments with your approval, removes its `priority:` rank and `flow:` label, swaps `state:30-needs-review` for `state:90-done`, and closes it |
| Triage finishes                        | Removes `triage:needs-evaluation` and swaps `flow:triage` for `flow:grilling` (it needs a spec) or `flow:implementation` (it's ready for an agent or a human), or closes it as `triage:wontfix`; comments with the outcome |
| Grilling finds the effort too big for one session | Swaps `flow:grilling` for `flow:wayfinding`; the ticket becomes the map (`wayfinder:map`) and comments why |
| `/to-tickets` charts a parent's tickets | Gives each new ticket `flow:implementation` and swaps the parent's `flow:` for `flow:implementation`; the parent closes by [Closing or review](#closing-or-review) once its tickets do |
| A Wayfinder map clears                 | Swaps the map's `flow:wayfinding` for `flow:grilling`, so `/to-spec` and `/to-tickets` turn its decisions into a buildable plan |
| Implementation hits a seam the ticket didn't decide | Swaps `flow:implementation` for `flow:grilling`, keeps the rank, and comments with the seam |
| Releasing a ticket unfinished          | Swaps `state:10-in-progress` for `state:20-waiting`, and comments with where it stopped                          |
| Anything else that blocks a ticket     | Adds `state:20-waiting`, and comments with what it's waiting on                                                  |
| A parent ticket closes, or a Wayfinder session starts | Re-ranks the affected tickets, and checks open tickets against `git log`: closes or gates a finished one by [Closing or review](#closing-or-review) |

## This project

Tripwire paths for Personal Ledger:

1. **Data:** anything under `migrations/` or `.sqlx/`.
2. **Shared foundations:** `lib-core`, `lib-database`, `lib-ledger`, or the `lib-rpc` protos.
3. **Security surface:** Sync Server auth, JWT handling, or `sync_users`.
4. **Build and guardrails:** the root `Cargo.toml`, `deny.toml`, `mise.toml`, `.githooks/` or `.github/workflows/`.
5. **Decisions:** `docs/adr/` or `GLOSSARY.md`.

Issue titles carry a bin-crate prefix (`TUI: `, `DESKTOP: `, `SYNC: `, or `CORE: `), set by [Issue tracker](issue-tracker.md), so area is never a label.
