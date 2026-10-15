# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all operations.

## Conventions

- **Title prefix**: every issue title starts with the bin crate it belongs to: `TUI: ` (`bin-tui`), `DESKTOP: ` (`bin-desktop`) or `SYNC: ` (`bin-sync-server`), e.g. `DESKTOP: remove Shell forwarders for Payees selection`. An issue for a lib crate takes the prefix of the bin being worked on when the issue was created, so a `lib-payees` change raised during Desktop work is `DESKTOP: `. An issue that belongs to no bin, or spans more than one (docs, CI, licence, shared design), is `CORE: `.
- **Create an issue**: `gh issue create --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: `gh issue view <number> --json number,title,body,labels,comments`.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Make an issue a sub-issue of a parent**: `gh issue create --parent <parent> ...`, or `gh issue edit <parent> --add-sub-issue <child>` afterwards (`gh` 2.94+). Older `gh`: `gh api --method POST repos/<owner>/<repo>/issues/<parent>/sub_issues -F sub_issue_id=<child-db-id>` (database id, as in **Blocking** below). Without sub-issues, put `Part of #<parent>` at the top of the child body.
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close <number> --comment "..."`

Infer the repo from `git remote -v`; `gh` does this automatically when run inside a clone.

## Diagrams

- A diagram in a spec ticket, Wayfinder map or Wayfinder ticket is optional. Add one when the ticket describes a flow across three or more components, or a migration's halfway state.
- Inside an issue body a diagram is Mermaid, which GitHub renders.
- Richer diagrams (reviews, ADRs, a map's destination) are Archify (`/archify`), committed as `docs/diagrams/<slug>.json` with the rendered `<slug>.html` beside it. The folder is flat and the kind (architecture, sequence, workflow, ...) lives only in the JSON, so a redraw as another kind doesn't break links. The slug names the subject; a diagram owned by an ADR or review is prefixed with it, e.g. `adr-0030-...`.
- Render from the repo root with the skill's `finalize` command (`node .claude/skills/archify/bin/archify.mjs finalize <type> docs/diagrams/<slug>.json docs/diagrams/<slug>.html --repo-root . --quality showcase --json`). The receipts it writes beside the HTML are gitignored. The skill is a copy of Archify's `v3.1.0` tag, not an `npx skills add` install, so upgrading means copying a newer tag over `.claude/skills/archify/`.
- An issue links to the diagram's repo path (`docs/diagrams/<slug>.html`). The inline Mermaid is what GitHub shows; the link is the full interactive view, opened locally or via `mise run docs-serve` (mdBook copies `docs/diagrams/` into the book; GitHub Pages only updates on release).

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents:

- **Read a PR**: `gh pr view <number> --comments` and `gh pr diff <number>` for the diff.
- **List external PRs for triage**: `gh api --paginate 'repos/{owner}/{repo}/pulls?state=open' --jq '.[] | select(.author_association | IN("OWNER","MEMBER","COLLABORATOR") | not) | {number, title, author: .user.login, author_association, labels: [.labels[].name]}'`.
- **Comment / label / close**: `gh pr comment`, `gh pr edit --add-label`/`--remove-label`, `gh pr close`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: resolve with `gh pr view 42` and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Read it as in **Read an issue** above.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets. What the `wayfinder:` and `flow:` labels mean is in [Flow labels](flow-labels.md).

- **Map**: a single issue labelled `wayfinder:map` and `flow:wayfinding`, holding the Notes / Decisions-so-far / Fog body. `gh issue create --label wayfinder:map --label flow:wayfinding`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (see **Make an issue a sub-issue of a parent**). Where sub-issues aren't enabled, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`) and a `flow:` label, by [Flow labels](flow-labels.md). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: GitHub's **native issue dependencies**, the canonical, UI-visible representation. Add an edge with `gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`, where `<blocker-db-id>` is the blocker's numeric **database id** (`gh api repos/<owner>/<repo>/issues/<n> --jq .id`, _not_ the `#number` or `node_id`). GitHub reports `issue_dependencies_summary.blocked_by` (open blockers only, the live gate). Where dependencies aren't available, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list the map's open children (`gh issue list --state open`, scoped to the map's sub-issues / task list), drop any with an open blocker (`issue_dependencies_summary.blocked_by > 0`, or an open issue in the `Blocked by` line) or an assignee; first in map order wins.
- **Claim**: `gh issue edit <n> --add-assignee @me --add-label state:10-in-progress`, the session's first write.
- **Resolve**: `gh issue comment <n> --body "<answer>"`, then `gh issue close <n>`, then append a context pointer (gist + link) to the map's Decisions-so-far.
