# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker      | Meaning                                  |
| -------------------------- | ------------------------- | ---------------------------------------- |
| `needs-triage`             | `triage:needs-evaluation` | Maintainer needs to evaluate this issue  |
| `needs-info`               | `triage:needs-info`       | Waiting on reporter for more information |
| `ready-for-agent`          | `triage:ready-for-agent`  | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `triage:ready-for-human`  | Requires human implementation            |
| `wontfix`                  | `triage:wontfix`          | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.

## Agent Labels

Each ticket that is ready for an agent also carries one `agent:<model>-<effort>` label naming the model and effort to run it with, following "Tickets and model choice" in `CLAUDE.md`. If Haiku medium hits ambiguity or a decision point, it stops and the ticket is relabelled up rather than guessed at.

| Label                 | Use for                                                                                                   |
| --------------------- | --------------------------------------------------------------------------------------------------------- |
| `agent:opus-medium`   | The first ticket of a series, any ticket with an open "Seams to decide" item, and research and decisions  |
| `agent:opus-high`     | Hard bugs and cross-cutting design                                                                        |
| `agent:sonnet-low`    | Repeating a pattern an earlier commit set (named in the ticket); `/end-user-docs` pages                   |
| `agent:haiku-medium`  | No new type, enum variant or interim shim, about 5 files or fewer, every decision written in the ticket   |

The other `wayfinder:` types are `agent:opus-medium`; a `wayfinder:task` takes whichever row its change fits.

## Priority and State Labels

Each open ticket has one rank (`priority:10-now`, `priority:20-next`, `priority:99-backlog`) and at most one state (`state:10-in-progress`, `state:20-waiting`, `state:30-needs-review`); a closed ticket has only `state:90-done`. Choose and change them by [Ticket labels](../development/ticket-labels.md).
