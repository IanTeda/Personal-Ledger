# Ticket labels

Every open GitHub issue carries a `priority:` rank, and a `state:` label while something is happening to it, so the issue list shows what to work on next. This page is the one place the rules for choosing and changing them live; `CLAUDE.md`, the README and `docs/agents/triage-labels.md` point here. What each label means is defined in `docs/agents/triage-labels.md`.

## Labels

On finishing, `triage:ready-for-agent` is removed and the `agent:*` label is set to the model and effort that actually actioned the ticket. Other labels are untouched.

- **The Now cap:** no more than 5 open tickets are `priority:10-now`, whatever their state. A sixth means moving one down to `priority:20-next`.
- **Blocked by a ticket:** a ticket blocked only by another ticket keeps its rank and gets no state. GitHub's native dependency shows the block, and the ticket rises once the blocker closes.
- **Backlog is for charting:** Claude takes a `priority:99-backlog` ticket only to grill or chart it, never to implement it.

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

- **Close:** `agent:haiku-medium` or `agent:sonnet-low`, no tripwire fired, and you didn't ask to review first. The closing comment says why Claude closed it without review: the agent label, no tripwire, every acceptance criterion verified.
- **Gate:** `agent:opus-*`, any tripwire, or you asked to review first. Claude swaps the state for `state:30-needs-review`, comments with the commit and each tripwire that fired, and opens the run's final message with `Needs human review: #<n>` and those tripwires. The ticket stays open.

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
| Creating a ticket                      | Gives it a `priority:` rank, and comments with the reason                                                         |
| Claiming a ticket (assigning it)       | Adds `state:10-in-progress`                                                                                       |
| Finishing a ticket                     | In the same session as the commit: removes `triage:ready-for-agent`, sets its `agent:<model>-<effort>` label to the model and effort that actioned it, then closes or gates it by [Closing or review](#closing-or-review). Closing comments with the commit and why no review was needed, removes its `priority:` rank and `state:` label, adds `state:90-done`, and closes it. Other labels stay |
| You approve a `state:30-needs-review` ticket | Comments with your approval, removes its `priority:` rank, swaps `state:30-needs-review` for `state:90-done`, and closes it |
| Releasing a ticket unfinished          | Swaps `state:10-in-progress` for `state:20-waiting`, and comments with where it stopped                          |
| Anything else that blocks a ticket     | Adds `state:20-waiting`, and comments with what it's waiting on                                                  |
| A parent ticket closes, or a Wayfinder session starts | Re-ranks the affected tickets, and checks open tickets against `git log`: closes or gates a finished one by [Closing or review](#closing-or-review) |

A label you set by hand is final. Claude's labels always come with a reason comment, so a label with no comment is yours.
