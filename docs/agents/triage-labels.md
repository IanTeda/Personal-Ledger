# Triage Labels

This file is the label vocabulary: what each label means, and how the skills' canonical roles map to the label strings in this repo's issue tracker. When labels are set and changed is in [Ticket labels](ticket-labels.md). A label on GitHub that isn't defined here shouldn't exist: delete it, or define it here first.

## Triage roles

| Label in mattpocock/skills | Label in our tracker      | Meaning                                  |
| -------------------------- | ------------------------- | ---------------------------------------- |
| `needs-triage`             | `triage:needs-evaluation` | Maintainer needs to evaluate this issue  |
| `needs-info`               | `triage:needs-info`       | Waiting on reporter for more information |
| `ready-for-agent`          | `triage:ready-for-agent`  | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `triage:ready-for-human`  | Requires human implementation            |
| `wontfix`                  | `triage:wontfix`          | Will not be actioned                     |
| `bug`                      | `category:bug`            | Existing behaviour is wrong              |
| `enhancement`              | `category:enhancement`    | New or changed behaviour a user sees     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.

## Category Labels

Every ticket carries exactly one `category:` label naming the intent of the work: why the ticket exists. It is set when the ticket is created, usually in the backlog, and a ticket charted from a parent inherits the parent's category. The skills know only `bug` and `enhancement`; when an issue is neither, use `architecture`, `tooling` or `docs` rather than forcing it into `enhancement`.

| Label                   | Means                                                                                      |
| ----------------------- | ------------------------------------------------------------------------------------------ |
| `category:enhancement`  | New or changed behaviour a user sees                                                       |
| `category:bug`          | Existing behaviour is wrong, including a flaky test                                        |
| `category:architecture` | Restructuring with no behaviour change: refactors, seams, deepening modules                |
| `category:tooling`      | Build, CI, test harnesses, dev workflow, dependencies and agent skills                     |
| `category:docs`         | End-user or developer documentation only                                                   |

Performance work is a `bug` or `tooling`, writing tests is part of the change it tests, and security is a review tripwire rather than a category.

## Agent Labels

Each ticket that is ready for an agent also carries one `agent:<model>-<effort>` label naming the model and effort to run it with. Pick it by what the change adds, not by the ticket's type, and use the same split when picking a subagent's model. If Haiku medium hits ambiguity or a decision point, it stops and the ticket is relabelled up rather than guessed at.

| Label                 | Use for                                                                                                                                                                                                  |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `agent:haiku-medium`  | Adds no new type, enum variant or interim shim, touches about 5 files or fewer, and every decision is written in the ticket (tests, lint fixes, small refactors)                                       |
| `agent:sonnet-low`    | Repeats a pattern an earlier commit set, with that commit named in the ticket; end-user docs pages                                                                                                      |
| `agent:opus-medium`   | The first ticket of a series (it sets the pattern), any ticket with an open item under "Seams to decide", and research and decisions (Wayfinder charting and its non-`task` tickets, domain modelling, ADRs, design, code review) |
| `agent:opus-high`     | Hard bugs and cross-cutting design                                                                                                                                                                       |

The other `wayfinder:` types are `agent:opus-medium`; a `wayfinder:task` takes whichever row its change fits.

## Priority and State Labels

An open ticket has exactly one `priority:` rank and at most one `state:` beside it. Closing a ticket removes both and adds `state:90-done`, so a closed ticket keeps no `priority:` label. The numbers sort the labels in GitHub's label list. Choose and change them by [Ticket labels](ticket-labels.md).

| Rank                  | Means                                                                                                            |
| --------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `priority:10-now`     | Specified, and waiting costs: do these first                                                                     |
| `priority:20-next`    | Specified and ready to start, with no compounding cost or leverage                                               |
| `priority:99-backlog` | Not yet specified: an idea or parent ticket that needs grilling and Wayfinder charting before it can be ticketed |

| State                   | Means                                                                                                                    |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `state:10-in-progress`  | Assigned and being worked on                                                                                             |
| `state:20-waiting`      | Can't move until something happens: frozen, a decision, something outside the repo, or released unfinished               |
| `state:30-needs-review` | Done and committed, waiting on your review before it closes; see [Closing or review](ticket-labels.md#closing-or-review) |
| `state:90-done`         | Finished and closed. Replaces the rank and any other state on closing                                                    |

## Colours

Each family shares one colour, so a family reads at a glance in the issue list. `priority:` is the exception: its shades show urgency.

| Family       | Colour                                     |
| ------------ | ------------------------------------------ |
| `category:`  | `0E8A16`                                   |
| `wayfinder:` | `5319E7`                                   |
| `agent:`     | `1D76DB`                                   |
| `triage:`    | `D93F0B`                                   |
| `state:`     | `D4C5F9`                                   |
| `priority:`  | `B60205` now, `FBCA04` next, `C5DEF5` backlog |
