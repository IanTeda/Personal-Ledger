---
name: review-parent
description: Review a parent ticket's commits read-only on two axes (spec, and standards and quality), report the findings, and turn the ones your human picks into new child tickets of the parent.
argument-hint: "<parent issue number>"
disable-model-invocation: true
---

# Review parent

Review the work done under a parent ticket and turn what your human chooses into **follow-up tickets**. This skill is **read-only on code**: it never edits, commits or stashes. Its only writes are to the issue tracker, and only after your human picks. Fixes happen later, as ordinary children, through `/implement-parent`.

The rules this skill applies, each in one place:

- Creating a ticket, its `## Labels` section, and approving a gated ticket: `docs/agents/ticket-labels.md` ("When labels change").
- The `agent:` label for each new ticket: `docs/agents/triage-labels.md` (the `agent:` table).
- Issue titles, sub-issues and native dependencies: `docs/agents/issue-tracker.md`.
- What a self-contained ticket body holds (spec, "Seams to decide", acceptance criteria): `CLAUDE.md` "Tickets and model choice".

## 1. Scope

1. **Read the parent and its children**: `gh issue view <parent> --json number,title,body,labels,comments` and `gh api repos/{owner}/{repo}/issues/<parent>/sub_issues --paginate`.
2. **Pin `BASE`**:
   - If an earlier `/review-parent` comment on the parent says `Reviewed through <sha>`, `BASE` is that SHA, so the review covers only the work since then.
   - Otherwise, `BASE` is the parent of the oldest commit whose message names the parent or any child (`git log --reverse --format=%H --grep '#<n>'` over all of them).
3. **Pin `HEAD`**: record `git rev-parse HEAD` as the reviewed head.
4. Confirm the scope before going further. `git diff BASE...HEAD` must be non-empty. If it is empty, report that there is nothing to review and stop.

## 2. Review

Dispatch both reviewers in one message, so they run in parallel. Each is a fresh, read-only subagent with model `opus` and effort `medium` (`triage-labels.md`: code review). Each prompt is its brief in [`REVIEWERS.md`](REVIEWERS.md), with `BASE`, `HEAD`, the commit list (`git log --oneline BASE..HEAD`), the parent and the children filled in. Send nothing more.

Both axes run here, once. Do not also run `/simplify` or `/code-review`, which would repeat the same findings.

## 3. Report

Merge the two reports into one, and keep it in your own reply rather than on the tracker.

1. Put the **Spec** findings and the **Standards and quality** findings under separate headings. Don't rerank across the two axes.
2. Drop any finding that restates a decision recorded in the parent's or a child's `## Decisions`, and note that you dropped it.
3. Draft **proposed tickets**. Group findings that share one fix into one ticket. For each ticket give:
   - its title, with the bin-crate prefix
   - the findings it covers
   - its `flow:` label: `flow:implementation` when every decision is made, `flow:grilling` when it needs one
   - its `agent:` label, by the `triage-labels.md` table (a `flow:grilling` ticket carries none)
   - its `priority:` rank and the reason for it

   Findings worth no ticket are listed as **leave as is**, each with why.

## 4. Ask

Use AskUserQuestion with `multiSelect`. Ask "Which of these should become child tickets of #<parent>?", with one option per proposed ticket. Each option's label is the ticket title, and its description is its findings and its `agent:` label.

AskUserQuestion takes 4 options per question and 4 questions per call. Spread up to 16 proposed tickets across questions. Past 16, ask in plain text for the numbers to create.

Then ask a second, single-select question about the parent:

- When no ticket is picked: close the parent (your human's approval of its gate), or leave it open.
- When some are picked: leave the parent open for them (recommended), or close it anyway.

## 5. Create

Create each picked ticket by `ticket-labels.md` "Creating a ticket":

1. **Body**: a self-contained spec. Include `## Problem` (each finding, quoted with `file:line`), `## Decisions`, `## Seams to decide` (`None`, or the open seam), `## Acceptance criteria` (naming the tests that prove it), and `## Labels` (one line per label, with its reason).
2. **Labels**:
   - `category:`: inherited from the parent.
   - `priority:`: as drafted.
   - `flow:`: as drafted.
   - `agent:`: as drafted, by the `triage-labels.md` table. When the label leans on an earlier commit's pattern, name that commit in the body.
3. **Link**: make it a sub-issue of the parent, and wire any blocking edges in a second pass, once every new ticket has its number.

Then comment on the parent: `Reviewed through <HEAD>` (the marker step 1 reads next time), the commit range, and each new ticket with its findings and why. If the parent carries `state:needs-review` and your human chose to close it, close it by `ticket-labels.md` "You approve a `state:needs-review` ticket".

## 6. Final message

- The report from step 3.
- **Created**: each new ticket by name, with its labels.
- **Left as is**: each finding not ticketed.
- **Parent**: closed or left open, and the next step: `/implement-parent <parent>` runs the new children.
