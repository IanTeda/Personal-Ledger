---
name: priorities-tickets
description: Walk every open ticket, retire the ones other work has superseded, and set the next five `priority:10-now` tickets by cost of delay.
disable-model-invocation: true
---

# Priorities Tickets

Walk the open tickets and leave the tracker with exactly five live, unblocked, specified tickets on `priority:10-now` and at most ten on `priority:20-next`, each with a `## Labels` line saying why it holds its slot. `docs/agents/ticket-labels.md` holds the caps and the base ranking order and wins where this skill disagrees; read only its **Ranking**, **Recording label decisions** and **Closing or review** sections. This skill adds the walk, the **supersession** sweep and the picking rules that turn that order into five tickets.

## Steps

1. **Snapshot.** Run `python3 .claude/skills/priorities-tickets/digest.py` once and work from its output: one block per open ticket, oldest first, with its labels, assignee, open blockers, parent, sub-issues, Seams to decide, latest comment, `## Labels` section and a `flags:` line, plus the current rank sets and the ADRs added since the oldest ticket. Read a full body (`gh issue view <n>`) only when a verdict or a seam collision needs more than the digest shows. Done when the digest has run.
2. **Supersession sweep.** Give every open ticket one verdict by [Supersession](#supersession): **live**, **done**, **superseded** or **drifted**. Done when every ticket has a verdict and each non-live verdict names its evidence (a commit, an ADR, a closed ticket, a missing file).
3. **Eligibility.** Keep only the tickets [Eligibility](#eligibility) admits. Done when each excluded ticket has its reason.
4. **Score.** Weigh each eligible ticket by [Picking rules](#picking-rules), writing one line per rule. Done when every eligible ticket has a line for blast radius, downstream impact, route and age.
5. **Pick five Now.** Choose the set by [Choosing the set](#choosing-the-set). Fewer than five eligible tickets is a valid result: fill what you can and report the gap, never promote a ticket [Eligibility](#eligibility) excludes to fill it.
6. **Pick ten Next.** Rank the remaining workable tickets by [The Next ten](#the-next-ten) and keep the top ten on `priority:20-next`; the rest overflow to `priority:99-backlog`. Done when Next holds at most ten and every overflow ticket has its reason.
7. **Propose.** Show the user one table: the five Now picks and the ten Next picks with their reasons, every ticket leaving Now or Next and why, and every non-live verdict with the action it takes. Ask with `AskUserQuestion` and wait for a yes; the user may swap picks or overrule a verdict. Hand-set labels (a `priority:` with no line in the ticket's `## Labels` section) are the user's: list any change to one as a question in this step, and leave it as it is unless they say yes.
8. **Apply.** Once approved, write in this order so neither cap is exceeded mid-run: retire the non-live tickets, move the Next overflow to `priority:99-backlog`, demote the tickets leaving Now to `priority:20-next`, promote the Now picks, then promote the Next picks. Each rank change updates its line in the ticket's `## Labels` section in place (`gh issue edit <n> --body-file`), and each `flow:` or `state:` change is a comment naming this skill as the step that caused it. Done when `gh issue list --state open --label priority:10-now` returns exactly the approved five and `--label priority:20-next` exactly the approved Next set.
9. **Report.** List the five Now and ten Next with one-line reasons, what was retired and on what evidence, what left Now or Next, and any gap or hand-set label the user kept.

## Supersession

A ticket is written against the codebase and decisions of its day; the work and the ADRs since can overtake it. The digest's `flags:` line runs the mechanical checks:

- **commits cite it:** commits since the ticket opened that name `#<n>`. One that did what the acceptance criteria ask is evidence of **done**.
- **cited ADRs changed since:** an ADR the body cites, changed after the ticket opened.
- **paths not in the tree:** backticked paths the body names that no tracked file matches: moved, deleted, or still to be created by the ticket.
- **names closed:** tickets the body names that have closed, such as a blocker, a parent, or the work it waited on.

A ticket with `flags: none` is **live** unless a judgement check below says otherwise. For a flagged ticket, and for these judgement checks across every ticket, decide the verdict:

- **ADRs added since it opened** (the digest's list): one that decides the ticket's question differently, or supersedes the ADR it builds on.
- **The glossary:** a term the ticket relies on renamed or retired in `GLOSSARY.md`.
- **Other tickets:** a newer open ticket that now owns the same work, or a closed parent or cleared Wayfinder map whose route no longer includes it.

Verdicts and the action each takes on approval:

- **Live:** the spec still matches the tree and the decisions. No action.
- **Done:** the work landed. Close or gate it by Closing or review in `docs/agents/ticket-labels.md`, citing the commit.
- **Superseded:** other work or a decision made it moot. Close as not planned with a comment naming what superseded it.
- **Drifted:** the goal still stands but the spec names code or decisions that have moved. Swap its `flow:` to `flow:grilling`, keep its rank, and comment with each point of drift. A drifted ticket is not eligible this run.

## Eligibility

A ticket can take a Now slot only if it is:

- **Workable:** either `flow:implementation` (specified, with an `agent:` label), or a `flow:grilling` or `flow:wayfinding` decision ticket whose question is written down, since a grilling session is the work. A Wayfinder map, a parent whose own work is its sub-issues, and a `flow:triage` ticket take no slot of their own.
- **Live** by the sweep.
- **Unblocked:** no open blocker in the digest's `graph:` line, or every open blocker is itself one of the five. Rank the blocker, not the blocked ticket.
- **Free:** no `state:waiting`, and no assignee other than the user.

A `state:needs-review` ticket already on Now keeps its rank and counts toward the cap, because the cap counts every state. List it first in the proposal so the user can clear it and free the slot.

## Picking rules

Cost of delay decides; these rules are how to read it for this codebase. The first three feed compounding and leverage in `docs/agents/ticket-labels.md`; the last two break ties.

- **Blast radius:** how much code the change touches and how much depends on that code. Find it from the crates and modules the body names, their dependents (`cargo tree -i <crate>`, `docs/diagrams/workspace-dependencies.html`) and `graphify query`. A wide change to a shared foundation (`lib-core`, `lib-database`, `lib-viper`, `lib-shell`, the protos) gets dearer with every ticket built on it, so wide blast radius on a seam ranks up. Wide blast radius on a leaf (one View, one Dialog) does not.
- **Downstream impact:** how many open tickets it blocks, natively or by being named in their bodies ("waits on #n", "after #n"), and how many features sit on the architecture it settles. A ticket that unblocks two others outranks one that unblocks none. A guardrail that stops a repeating mistake (a lint, a `viper_dependencies.rs` rule, a test harness gap) counts every future ticket as downstream.
- **Route:** on an open Wayfinder map's route, and the next ticket on it, ranks above an orphan of equal weight. A map's frontier ticket (first unblocked in map order) is the route's next step.
- **Age:** a live ticket that has sat specified for weeks while nearby code moved is paying drift: its spec gets dearer to keep true. If commits keep landing in the files it names, treat age as compounding; otherwise age only breaks ties, older first.
- **Effort:** last tie-break, smaller first. Size never ranks a ticket down.

## Choosing the set

The five are worked together, so pick a set, not five individual winners:

- **One per seam:** two picks that change the same module or crate at once collide. Keep the one with more downstream impact and leave the other on `20-next`, unless one blocks the other, in which case keep only the blocker.
- **Mixed agents:** where the scores are close, prefer a set that is not all `agent:human` or all `agent:opus-high`, so the Now list can be worked without one bottleneck. This never outranks cost of delay.
- **Keep what is moving:** a Now ticket that is assigned and in progress stays unless it went non-live in the sweep; demoting it mid-flight wastes the context already spent.

Each pick's `## Labels` line names the rule that won it, e.g. `` `priority:10-now`: blocks #981 and #983, and lib-database's adapters keep growing on the old seam. ``; each demotion names what outranked it, e.g. `` `priority:20-next`: outranked by #975 on the same seam; nothing builds on it yet. ``

## The Next ten

Next is the queue the Now five refill from, so rank it by the same [Picking rules](#picking-rules) over every workable (as [Eligibility](#eligibility) defines it), live ticket not picked for Now. A map that already holds `priority:20-next` keeps it and counts toward the ten, as the rank its route carries. It is looser than Now in two ways:

- **Blocked is fine** when the blocker is in Now or Next: the ticket queues behind it and rises when it closes. A ticket whose blocker sits outside both does not take a Next slot.
- **Seams may repeat:** two Next tickets may touch the same seam, since they won't be worked at once. Order them blocker first.

Every Next ticket counts toward the ten, `state:waiting` and assigned tickets included; a waiting ticket keeps its slot only if it outranks the eleventh.

Tickets ranked below ten **overflow** to `priority:99-backlog`. They keep their `flow:` and `agent:` labels, so the backlog shows they are workable and waiting for a slot, and their `## Labels` line says so, e.g. `` `priority:99-backlog`: specified; outside the Next ten, outranked by #962 and #970. `` Each run re-ranks them with the rest, so an overflow ticket rises once it outranks the tenth.
