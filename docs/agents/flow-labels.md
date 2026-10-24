# Flow labels

A **Flow** is one of the paths through the mattpocock skills. Every open issue carries one `flow:` label naming the Flow it goes down next, so the label says which skill to open. Maps and their tickets also carry a `wayfinder:` label naming what kind of Wayfinder ticket they are. This page defines both families. Where it disagrees with the `/wayfinder` skill, this page wins. The rest of the label vocabulary is in [Triage labels](triage-labels.md), and the rules for setting and changing labels are in [Ticket labels](ticket-labels.md).

## Flows

| Label                 | Skills                                                                         | Use when                                                                                                       |
| --------------------- | ------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| `flow:grilling`       | `/grill-with-docs`, `/to-spec`, `/to-tickets`                                  | An idea, or a ticket that isn't yet specified, that one session can sharpen into a spec and tickets            |
| `flow:wayfinding`     | `/wayfinder`, and its research, prototype, grilling and task tickets           | An effort too big and foggy for one session: a map and the decision tickets on it                              |
| `flow:implementation` | `/implement`, `/tdd`, `/code-review`, `/retro`; `/diagnosing-bugs` for hard bugs | Specified work, ready to build, including bug fixes                                                             |
| `flow:triage`         | `/triage`                                                                      | A raw issue someone else filed: a bug report or a feature request that hasn't been evaluated yet                |

The `flow:` label is the only label that names the next skill. `priority:` ranks the ticket, `state:` says what is happening to it now, and `agent:` says who actions it, and with `flow:implementation` marks work ready to take. Waiting and review are `state:`, because they pause a Flow rather than name one.

- **One per open issue:** every open issue carries exactly one `flow:` label. Closing an issue removes it, the same way it removes the `priority:` rank.
- **It follows the work:** a ticket's Flow is what happens to it next, not where it came from or where it sits. A Wayfinder map that carries execution in its Notes may chart build slices as child tickets. Those slices are `flow:implementation`, picked up with `/implement`, and they keep their `wayfinder:task` label while they are children of the map.
- **Triage is for issues you didn't create:** a ticket you write yourself starts in `flow:grilling`, never `flow:triage`. Tickets `/to-tickets` produces are already specified and go straight to `flow:implementation`.
- **Bugs are fixed in implementation:** `/triage` ends at a specified ticket, it doesn't fix it. The fix is `/implement`, with `/diagnosing-bugs` when the bug resists a first look.

When a ticket moves from one Flow to another is in [Ticket labels](ticket-labels.md#when-labels-change).

## Wayfinder labels

A Wayfinder map and every child ticket on it whose question is still a decision carry `flow:wayfinding` beside their `wayfinder:` label. The `wayfinder:` label says how the ticket is resolved, and the `flow:` label says which skill to open. Maps and their tickets also carry `category:` and `priority:` labels by [Ticket labels](ticket-labels.md), whatever the `/wayfinder` skill says about `wayfinder:` labels being the only ones.

| Label                 | Means                                                                                                                         |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `wayfinder:map`       | The map: a single issue holding the Destination, Notes, Decisions so far, Not yet specified and Out of scope, with tickets as its sub-issues |
| `wayfinder:research`  | AFK: reading documentation, third-party APIs or local resources to surface a fact a decision waits on, resolved by `/research` |
| `wayfinder:prototype` | HITL: a cheap, rough artifact to react to, made with `/prototype`, when "how should it look or behave" is the key question     |
| `wayfinder:grilling`  | HITL: a conversation, run with `/grilling` and `/domain-modeling`. The default type                                            |
| `wayfinder:task`      | HITL or AFK: manual work that must happen before a decision can be made, or a build slice on a map that carries execution      |

How a map and its tickets are created, blocked, claimed and resolved with `gh` is in [Issue tracker](issue-tracker.md#wayfinding-operations).
