---
name: implement-parent
description: Implement every sub-issue of a parent ticket in one unattended run, one fresh implementer subagent per child, then close the parent with a review, a simplify pass and the full suites.
argument-hint: "<parent issue number>"
disable-model-invocation: true
---

# Implement parent

Drive a parent ticket's children to done without a human in the loop. You are the **orchestrator**: you hold the task graph, dispatch one fresh **implementer** subagent per child, and own every label, comment and close. Implementers write code; you write to the issue tracker.

The run is unattended, so nothing pauses it but a **halt** (below). A ticket that needs your human's eyes is **gated** (`state:needs-review`) and the run moves on; your human reviews every gated ticket at the end. Talk to implementers through **context pointers** (issue numbers, commit SHAs, file paths), never by restating a ticket.

The rules this skill applies, each in one place:

- Labels, finishing, tripwires and close-or-gate: `docs/agents/ticket-labels.md` ("When labels change", "Closing or review", "This project").
- `agent:` label to model and effort: `docs/agents/triage-labels.md`.
- `gh` operations, sub-issues and native dependencies: `docs/agents/issue-tracker.md`.
- The signing key check, the parent-close suites and the commit format: `CLAUDE.md`.

## 1. Preflight

Every check passes before the first write. On any failure, stop and report which check failed; touch nothing.

1. **Auto mode**: the session's system context says auto mode is active. Otherwise tell your human to restart in auto mode.
2. **Clean tree on `concept`**: `git status --porcelain` is empty and the branch is `concept`.
3. **Signing key**: CLAUDE.md's signing key check passes.
4. **Read the parent and its children**: `gh issue view <parent>` and `gh api repos/{owner}/{repo}/issues/<parent>/sub_issues --paginate`, then `gh issue view <n> --json number,title,body,labels,state,assignees` for each open child.
5. **Every child is specified**: every open child carries `flow:implementation`. Any open child on `flow:grilling`, `flow:wayfinding` or `flow:triage` refuses the run: list each one with its label, so your human resolves them first.
6. **Base commit**: record `BASE`, the commit before the parent's work began. It is the parent of the oldest commit whose message names the parent or any child (`git log --reverse --format=%H --grep '#<n>'` over all of them), or `HEAD` when none exists yet. The parent-close review diffs from it, so a resumed run reviews the whole parent.

## 2. Build the task graph

- **Eligible**: an open child on `flow:implementation` with no `agent:human`, `state:waiting` or `state:needs-review` label and no assignee. Every other open child is **skipped**; record why for the report.
- **Edges**: each child's blockers from `gh api repos/{owner}/{repo}/issues/<n>/dependencies/blocked_by`. A child is ready once every blocker is closed or gated (a gated blocker's commit is on `concept`, so its dependants build on it).
- **Order**: the **frontier** is every ready eligible child; take them one at a time, first in sub-issue order.

## 3. Run each child

Loop until the frontier is empty or the run halts.

1. **Claim**: `gh issue edit <n> --add-assignee @me`.
2. **Dispatch**: one fresh implementer via the Agent tool, in the foreground, with `model` and `effort` read from the child's `agent:<model>-<effort>` label (`agent:sonnet-low` is model `sonnet`, effort `low`). This skill is the instruction that sets the effort. The prompt is the brief in [`IMPLEMENTER.md`](IMPLEMENTER.md) with `<n>` and `<parent>` filled in; send nothing more.
3. **Check the report** against the diff: `git show --stat <sha>` confirms the commit exists and the tripwire paths it lists are the ones it touched.
4. **Finish** by its outcome:
   - **Done**: follow ticket-labels.md "Finishing a ticket": set the `agent:` label and its `## Labels` line to the model and effort the implementer ran with, then close or gate by "Closing or review", checking the tripwires against the commit's diff and the implementer's report. Tripwire 6 fires on the report's `decisions:` line only; its `notes:` line goes in the closing comment and gates nothing. A gate does not stop the run.
   - **Seam**: the ticket needs a decision it doesn't make. `git stash push -u -m "implement-parent #<n>: <one-line reason>" -- <the report's changed paths>`, so another session's edits stay in the tree, swap `flow:implementation` for `flow:grilling` by ticket-labels.md, unassign, and comment with the seam and the stash name.
   - **Failed**: tests or lint stayed red, or the implementer gave up. Stash as above, unassign, add `state:waiting`, and comment with where it stopped and the stash name.
5. **Prune**: a seam or failure removes every child that transitively depends on it from this run; record them as skipped.
6. **Halt** on either:
   - **Two failures in a row** (seam or failed, back to back): something in the environment is likely wrong.
   - **Signing key gone**: the implementer reports the check failed. Leave its work uncommitted in place, unassigned, and comment on the child that the run halted there.

   On a halt, send a push notification (the `PushNotification` tool) naming the reason, then go straight to the report (step 5), skipping parent close.

## 4. Close the parent

Once the frontier is empty, run these in order. Run CLAUDE.md's signing key check before each commit; a failure is a halt.

1. **Review**: call the Skill tool with `mattpocock-skills:code-review`, fixed point `BASE`, spec source the parent and its children.
2. **Fix**: one fresh fixer subagent (model `opus`, effort `medium`) takes every finding, applies them, runs the targeted tests and `mise run lint`, and commits once as `<area>: address the #<parent> review`. Findings it cannot resolve without a decision go to the report unapplied. Close-or-gate the fix commit with the parent (step 4.5).
3. **Simplify**: call the Skill tool with `simplify` over `git diff BASE...HEAD`, then `mise run lint` and the targeted tests for the files it touched, and commit as `<area>: simplify the #<parent> changes`. If lint or a test goes red, `git revert --no-edit` the commit (signed) and record it in the report.
4. **Full suites**: bare `mise run end-to-end-ui`, `mise run end-to-end-ui smoke` and `cargo test --workspace`, so they test the final tree. Each suite's exit status is its verdict; read its output only for the names of failing tests. On a red suite with a simplify commit in place, revert it and re-run the failing suite: green means the simplify caused it (keep the revert, record it); still red means the failure is the parent's, and the parent is gated.
5. **Close or gate the parent** by ticket-labels.md "Closing or review", over `git diff BASE...HEAD`. It is gated if any child was gated, seamed, failed or skipped, if a suite stayed red, or if any review finding went unapplied; the gate comment lists each reason. Then do ticket-labels.md's "A parent ticket closes" re-rank.

## 5. Report

The run's final message, first line `Needs human review: #<n>, …` when anything was gated (ticket-labels.md "Closing or review"). Then:

- **Done**: each closed child with its commit.
- **Gated**: each gated ticket with its tripwires.
- **Seams and failures**: each with its stash name and where it stopped.
- **Skipped**: each with why (label, assignee, or a blocker that seamed or failed).
- **Parent close**: review findings applied and unapplied, whether the simplify commit stayed, and each suite's result.
- **Halt**: the reason, and the resume line: fix the cause, `git stash pop` if needed, then `/implement-parent <parent>`. A re-run resumes by itself, since closed, gated and waiting children drop out of the frontier.
- **Pushed**: nothing. Commits stay local on `concept`.
- Close with: run `/retro` on this session.
