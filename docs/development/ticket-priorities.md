# Ticket priorities

Every open GitHub issue carries `priority:` labels, so the issue list shows what to work on next. This page is the one place the rules live; `CLAUDE.md`, the README and `docs/agents/triage-labels.md` point here.

## Labels

A ticket has exactly one **rank** and at most one **state** beside it. A closed ticket has no rank or state: its `priority:` label is `90-done`. On finishing, `ready-for-agent` is removed and the `agent:*` label is set to the model and effort that actually actioned the ticket. Other labels are untouched. The numbers sort the labels in GitHub's label list.

| Label                     | Kind  | Means                                                                                                                  |
| ------------------------- | ----- | ---------------------------------------------------------------------------------------------------------------------- |
| `priority:10-now`         | Rank  | Specified, and waiting costs: do these first                                                                           |
| `priority:20-next`        | Rank  | Specified and ready to start, with no compounding cost or leverage                                                     |
| `priority:30-in-progress` | State | Assigned and being worked on                                                                                           |
| `priority:40-waiting`     | State | Can't move until something happens: frozen, a decision, something outside the repo, or released unfinished             |
| `priority:41-needs-review` | State | Done and committed, waiting on your review before it closes; see [Closing or review](#closing-or-review)             |
| `priority:99-backlog`     | Rank  | Not yet specified: an idea or parent ticket that needs grilling and Wayfinder charting before it can be ticketed       |
| `priority:90-done`        | Done  | Finished and closed. Replaces the rank and state on closing; a closed ticket keeps no other `priority:` label |

- **The Now cap:** no more than 5 open tickets are `10-now` or `30-in-progress`. A sixth means moving one down to `20-next`.
- **Blocked by a ticket:** a ticket blocked only by another ticket keeps its rank and gets no state. GitHub's native dependency shows the block, and the ticket rises once the blocker closes.
- **Backlog is for charting:** Claude takes a `99-backlog` ticket only to grill or chart it, never to implement it.

## Ranking

Rank by the cost of delay, weighing these in order:

1. **Compounding:** does waiting make this ticket or others more expensive? Architecture other work builds on, a guardrail that stops a repeating mistake, build or test speed that every ticket pays for. Features built on bad architecture are rework.
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

- **Close:** `agent:haiku-low` or `agent:sonnet-medium`, no tripwire fired, and you didn't ask to review first. The closing comment says why Claude closed it without review: the agent label, no tripwire, every acceptance criterion verified.
- **Gate:** `agent:opus-*`, any tripwire, or you asked to review first. Claude swaps the state for `41-needs-review`, comments with the commit and each tripwire that fired, and opens the run's final message with `Needs human review: #<n>` and those tripwires. The ticket stays open.

Tripwires, checked against the commit's diff and the run:

1. **Data:** anything under `migrations/` or `.sqlx/`.
2. **Shared foundations:** `lib-core`, `lib-database`, `lib-ledger`, or the `lib-rpc` protos.
3. **Security surface:** Sync Server auth, JWT handling, or `sync_users`.
4. **Build and guardrails:** the root `Cargo.toml`, `deny.toml`, `mise.toml`, `.githooks/` or `.github/workflows/`.
5. **Decisions:** a new or changed ADR, or a `GLOSSARY.md` term.
6. **The run:** an acceptance criterion not verified, a test skipped, or a decision the ticket didn't make.

A gated ticket closes when you close it, or when you tell Claude it's approved.

## When labels change

| When                                   | Claude does                                                                                                       |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Creating a ticket                      | Gives it a rank, and comments with the reason                                                                     |
| Claiming a ticket (assigning it)       | Adds `30-in-progress`                                                                                             |
| Finishing a ticket                     | In the same session as the commit: removes `ready-for-agent`, sets its `agent:<model>-<effort>` label to the model and effort that actioned it, then closes or gates it by [Closing or review](#closing-or-review). Closing comments with the commit and why no review was needed, swaps its `priority:` label for `90-done`, and closes it. Other labels stay |
| You approve a `41-needs-review` ticket | Comments with your approval, swaps `41-needs-review` for `90-done`, and closes it                              |
| Releasing a ticket unfinished          | Swaps `30-in-progress` for `40-waiting`, and comments with where it stopped                                      |
| Anything else that blocks a ticket     | Adds `40-waiting`, and comments with what it's waiting on                                                        |
| A parent ticket closes, or a Wayfinder session starts | Re-ranks the affected tickets, and checks open tickets against `git log`: closes or gates a finished one by [Closing or review](#closing-or-review) |

A label you set by hand is final. Claude's labels always come with a reason comment, so a label with no comment is yours.
