---
name: viper-review
description: Review the code, recent or whole, against the VIPER architecture for five changes that make it more VIPER, deeper and simpler; rate them in a temp HTML report, recommend the top one, then chart your human's picks as a Wayfinder map of decision tickets and build slices.
argument-hint: "[<parent issue number>] [recent | all | <crate or path>]"
disable-model-invocation: true
---

# VIPER review

Find the five changes that would most move the code toward the VIPER architecture, rate them, and recommend one. This skill is **read-only on code**: it never edits, commits or stashes. Its writes are a report in the temp directory and, only after your human picks, a **map** on the tracker: a parent labelled `wayfinder:map` whose Notes carry execution, so its fog is worked through `/wayfinder` and its build slices through `/implement-parent`.

A **candidate** is one proposed change: what is wrong, where (`file:line`), what it becomes, and its **fog**, the decisions it still needs before it can be specified. A candidate with no fog is ready to build; fog is what a grilling session clears.

The sources this skill reads, each in one place:

- The VIPER rules a review checks against: `docs/development/viper-architecture.md` (Rules, Persistence, During the transition).
- The decisions a candidate must not re-litigate: the ADRs in `docs/adr/`, chiefly 0047, 0048, 0055, 0056 and 0062, and any ADR naming VIPER, a port, a Presenter or an Interactor.
- The domain names a candidate uses: `GLOSSARY.md`.
- The architecture vocabulary (**module**, **interface**, **depth**, **seam**, **adapter**, **leverage**, **locality**, the deletion test): the `codebase-design` skill.
- Creating a ticket, its labels and its `## Labels` section: `docs/agents/ticket-labels.md` ("When labels change"); a map's children's `wayfinder:` and `flow:` labels: `docs/agents/flow-labels.md` ("A map's children"); the `agent:` label: `docs/agents/triage-labels.md`; titles, sub-issues, dependencies and the map's body: `docs/agents/issue-tracker.md` ("Wayfinding operations"); what a self-contained body holds: `CLAUDE.md` "Tickets and model choice".

## 1. Scope

1. **Parent**: if an argument is an issue number, read it: `gh issue view <parent> --json number,title,body,labels,comments` and its sub-issues (`gh api repos/{owner}/{repo}/issues/<parent>/sub_issues --paginate`). With no parent, carry on; step 5 offers to create one.
2. **Range**, from the other argument:
   - `all`: every crate under `crates/`. `BASE` is none.
   - `<crate or path>`: that subtree. `BASE` is none.
   - `recent`, or nothing: `BASE` is the SHA in the latest `VIPER-reviewed through <sha>` comment on the parent, else the commit 50 back (`git rev-parse HEAD~50`). The scope is the files `git diff --stat BASE...HEAD` touches, plus the modules they call across a seam.
3. **Pin `HEAD`**: `git rev-parse HEAD`.
4. **Open work**: list open architecture tickets (`gh issue list --state open --label category:architecture --json number,title,labels`) and the parent's children. A change one of them already owns is not a candidate.

Done when the scope, `BASE`, `HEAD` and the list of owned work are written down. An empty `recent` diff means nothing to review: say so and stop.

## 2. Explore

Dispatch both explorers in one message, so they run in parallel. Each is a fresh, read-only subagent with model `opus` and effort `medium`. Each prompt is its brief in [`EXPLORERS.md`](EXPLORERS.md), with the scope, `BASE`, `HEAD` and the owned work filled in. Send nothing more.

- The **VIPER** explorer checks the code against the rules: the layer each piece sits in, the dependencies, the ports, what a Presenter, Interactor, Router and View may hold.
- The **depth** explorer reads it for shallow modules, leaking seams, duplication and needless complexity (`/codebase-design` and `/simplify`'s lenses).

Neither runs `/simplify` itself: `/simplify` applies fixes, and this skill writes no code.

## 3. Choose five

1. **Verify each premise**: open the `file:line` every surviving finding rests on and confirm the code is as it claims. Explorer output is a claim, not a fact; drop what doesn't hold.
2. **Merge** findings that share one change into one candidate, and drop any an ADR decided, unless the friction is real enough to reopen it (then the candidate says so: _"contradicts ADR-NNNN, worth reopening because …"_).
3. **Rate** each by [Rating](#rating), then keep the top five. Fewer than five that survive verification is a valid result: report the gap, never pad.

Done when there are five candidates (or the gap is stated), each with a verified `file:line`, a rating line per axis and its fog listed (or `None`).

## 4. Report

Write a self-contained HTML file to `${TMPDIR:-/tmp}/viper-review-<YYYYMMDD-HHMM>.html`, open it (`xdg-open <path>`), and give your human the absolute path. Follow the scaffold and diagram patterns in the `improve-codebase-architecture` skill's `HTML-REPORT.md` (`~/.claude/plugins/cache/mattpocock/mattpocock-skills/*/skills/engineering/improve-codebase-architecture/HTML-REPORT.md`), with these changes:

- **Header**: the scope, `BASE..HEAD`, and the five candidates as a ranked table: title, score, badge, layer (Entity, Interactor, Presenter, View, Router, port, adapter, cross-domain).
- **Each card**: Files, Problem (the VIPER rule or depth smell, quoted with `file:line`), Solution, Benefits (locality, leverage, tests), a before/after diagram showing the layers and the seam, the score with its three axis lines, the badge, and **Fog**: each open decision as a question, or `None`.
- **Top recommendation**: the first-ranked candidate and why it goes first: what it unblocks, what waiting on it costs, and why it beats the second.

Then give the ranked table and the top recommendation in your reply too, so the choice survives the browser tab.

## 5. Ask

Draft the **proposed tickets** for each candidate, splitting it along its fog, labelled by `flow-labels.md` "A map's children":

- Each part every decision covers becomes a **build slice**: `wayfinder:task`, `flow:implementation`, its `agent:` label by the `triage-labels.md` table.
- Each fog becomes a **decision ticket**: `wayfinder:grilling`, `flow:wayfinding`, holding the open questions. It **blocks** the build slices that wait on it, and its acceptance criterion is that the grilling session records the decisions and writes the remaining build slices as children of the same map (`/wayfinder <map>`, then `/to-tickets`).
- A candidate that is all fog is one decision ticket.

Use AskUserQuestion with `multiSelect`: "Which candidates should go on the map?", one option per candidate, its label the candidate's title and its description its score and the tickets it would create (how many decision tickets, how many build slices). With no parent, a second, single-select question asks whether to create the map (`CORE: VIPER review <date>`) or stop with the report.

## 6. Create

First the **map**, by `issue-tracker.md` "Wayfinding operations": the parent, labelled `wayfinder:map`, `flow:wayfinding`, `category:architecture` and `priority:99-backlog` (a map takes no rank of its own). Its body:

- **Destination**: every picked candidate has landed, each named in one line.
- **Notes**: "This map carries execution: build slices are `wayfinder:task` + `flow:implementation` children, picked up with `/implement-parent`", the rules a session reads (`docs/development/viper-architecture.md`, `GLOSSARY.md`), and every candidate with its score and badge.
- **Decisions so far**: empty.
- **Not yet specified**: fog the report hints at that you can't yet phrase as a question, or empty.
- **Out of scope**: each candidate your human passed on, its score, and why it sits past the Destination.
- **Labels**: one line per label, with its reason.

An existing parent becomes the map: rewrite its body into these sections (its old summary moves under Notes), swap its `flow:` for `flow:wayfinding` and add `wayfinder:map`, commenting why. A parent that is already a map grows instead: append the new candidates to its Destination and Notes and the passed ones to Out of scope.

Then each picked candidate's tickets, by `ticket-labels.md` "Creating a ticket":

1. **Body**: a self-contained spec. `## Problem` (each finding with `file:line`), `## Decisions`, `## Seams to decide` (`None`, or the fog), `## Acceptance criteria` (each naming a test or command that fails without the change; a rule across the codebase carries the grep that proves it), and `## Labels` (one line per label, with its reason).
2. **Labels**: `category:` inherited from the parent; `priority:` from the candidate's rank and the `ticket-labels.md` Ranking (only the top candidate may claim `priority:10-now`, and only within the Now cap); `flow:` and `agent:` as drafted.
3. **Link**: make each a sub-issue of the map, then, once every ticket has its number, wire each decision ticket as a blocker of the build slices waiting on it.
4. **Check**: run `mise run labels-check` and fix every issue it reports on the map or its children.

Then comment on the map: `VIPER-reviewed through <HEAD>` (the marker step 1 reads next time), the scope, the report's top recommendation, and each new ticket with its candidate.

## 7. Final message

- The ranked table and the top recommendation.
- **Created**: the map, then each ticket by title (its link inside), with its `wayfinder:`, `flow:`, `priority:` and `agent:` labels, grouped by candidate.
- **Out of scope**: each candidate your human passed on.
- **Next**: `/wayfinder <map>` works each decision ticket, one per session; once none is open, `/implement-parent <map>` builds the slices.

## Rating

Score each candidate out of 10 as the sum of three axes, each with one line of evidence:

- **VIPER gain (0–4)**: how far it moves code toward the rules. 4 breaks a rule a check would enforce (a dependency, I/O in a feature crate, a Presenter writing); 2 moves logic to the layer the rules name; 0 is cosmetic.
- **Compounding (0–4)**: by `ticket-labels.md` Ranking. 4 is a shared foundation (`lib-viper`, `lib-shell`, `lib-database`'s ports, `lib-core`) other tickets will copy or build on, or a pattern half the modules already repeat; 0 is a leaf only it touches.
- **Certainty (0–2)**: 2 with no fog, 1 with fog one decision ticket can clear, 0 if its fog needs several decision tickets or a map of its own.

The badge follows the score: **Strong** at 8 or more, **Worth exploring** at 5–7, **Speculative** below 5. Ties go to the smaller change.
