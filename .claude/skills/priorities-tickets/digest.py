#!/usr/bin/env python3
"""One compact block per open ticket, for /priorities-tickets.

Prints what ranking reads (labels, the `## Labels` section, Seams to decide, the issue graph, the
latest comment) and the mechanical half of the supersession sweep as a `flags:` line, so the agent
reads a full body only for a flagged ticket. Stdlib only; needs `gh` and `git` on PATH.
"""

import json
import re
import subprocess
import sys
from datetime import datetime, timezone

REPO_OWNER, REPO_NAME = "IanTeda", "Personal-Ledger"
SNIPPET = 300

QUERY = """
query($owner: String!, $name: String!, $cursor: String) {
  repository(owner: $owner, name: $name) {
    issues(states: OPEN, first: 50, after: $cursor) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number title body createdAt
        labels(first: 30) { nodes { name } }
        assignees(first: 5) { nodes { login } }
        parent { number state }
        subIssuesSummary { total completed }
        blockedBy(first: 20) { nodes { number state } }
        blocking(first: 20) { nodes { number state } }
        comments(last: 1) { totalCount nodes { body createdAt } }
      }
    }
  }
}
"""


def run(*args: str) -> str:
    return subprocess.run(args, check=True, capture_output=True, text=True).stdout


def open_issues() -> list[dict]:
    issues, cursor = [], None
    while True:
        args = ["gh", "api", "graphql", "-f", f"query={QUERY}", "-f", f"owner={REPO_OWNER}", "-f", f"name={REPO_NAME}"]
        if cursor:
            args += ["-f", f"cursor={cursor}"]
        page = json.loads(run(*args))["data"]["repository"]["issues"]
        issues += page["nodes"]
        if not page["pageInfo"]["hasNextPage"]:
            return issues
        cursor = page["pageInfo"]["endCursor"]


def closed_numbers() -> set[int]:
    out = run("gh", "issue", "list", "--state", "closed", "--limit", "5000", "--json", "number", "--jq", ".[].number")
    return {int(n) for n in out.split()}


def commits_since(since: str) -> list[tuple[str, str, str]]:
    """(short hash, ISO date, subject + body) for every commit since `since`."""
    out = run("git", "log", f"--since={since}", "--format=%h%x1f%cI%x1f%s%n%b%x1e")
    commits = []
    for record in out.split("\x1e"):
        parts = record.strip("\n").split("\x1f")
        if len(parts) == 3:
            commits.append((parts[0], parts[1], parts[2]))
    return commits


def adr_changes(since: str) -> tuple[list[tuple[str, str]], dict[str, str]]:
    """ADRs added since `since` (date, path), and each ADR number's latest change date."""
    out = run("git", "log", f"--since={since}", "--name-status", "--format=%x1e%cI", "--", "docs/adr/")
    added, changed = [], {}
    for record in out.split("\x1e"):
        lines = [line for line in record.strip().splitlines() if line]
        if not lines:
            continue
        date = lines[0][:10]
        for line in lines[1:]:
            status, _, path = line.partition("\t")
            match = re.search(r"(\d{4})-", path.rsplit("/", 1)[-1])
            if not match:
                continue
            changed.setdefault(match.group(1), date)  # log is newest first
            if status.startswith("A"):
                added.append((date, path))
    return added, changed


def section(body: str, heading: str) -> str:
    match = re.search(rf"^##+ {heading}\s*$(.*?)(?=^##+ |\Z)", body, re.M | re.S | re.I)
    return match.group(1).strip() if match else ""


def one_line(text: str, limit: int = SNIPPET) -> str:
    text = " ".join(text.split())
    return text if len(text) <= limit else text[: limit - 1] + "…"


def absent_paths(body: str, tracked: list[str]) -> list[str]:
    """Backticked file paths the body names that no tracked file ends with."""
    missing = []
    for token in set(re.findall(r"`([^`\s]+)`", body)):
        path = re.sub(r":\d+(-\d+)?$", "", token).rstrip("/")
        if "/" not in path or re.search(r"[<>*{}$\"']|^/|^https?:|::", path):
            continue
        last = path.rsplit("/", 1)[-1]
        if "." in last and not re.search(r"\.(rs|md|toml|sql|ftl|html|json|ya?ml|sh|proto)$", last):
            continue
        if not any(f == path or f.endswith("/" + path) or f.startswith(path + "/") or ("/" + path + "/") in f for f in tracked):
            missing.append(token)
    return sorted(missing)


def main() -> None:
    issues = sorted(open_issues(), key=lambda i: i["createdAt"])
    if not issues:
        print("No open tickets.")
        return
    closed = closed_numbers()
    oldest = issues[0]["createdAt"]
    commits = commits_since(oldest)
    adrs_added, adr_changed = adr_changes(oldest)
    tracked = run("git", "ls-files").splitlines()
    today = datetime.now(timezone.utc)

    def labels_of(issue: dict) -> list[str]:
        return sorted(node["name"] for node in issue["labels"]["nodes"])

    def numbers(nodes: list[dict], state: str) -> str:
        return " ".join(f"#{n['number']}" for n in nodes if n["state"] == state) or "-"

    ranks = {}
    for issue in issues:
        for name in labels_of(issue):
            if name.startswith("priority:"):
                ranks.setdefault(name, []).append(f"#{issue['number']}")
    print(f"# Open tickets: {len(issues)}, oldest first. Repo HEAD {run('git', 'rev-parse', '--short', 'HEAD').strip()}")
    for rank in sorted(ranks):
        print(f"{rank} ({len(ranks[rank])}): {' '.join(ranks[rank])}")
    print("ADRs added since the oldest ticket (newest first):")
    for date, path in adrs_added:
        print(f"  {date} {path.rsplit('/', 1)[-1].removesuffix('.md')}")
    print()

    for issue in issues:
        n, body, created = issue["number"], issue["body"] or "", issue["createdAt"]
        age = (today - datetime.fromisoformat(created.replace("Z", "+00:00"))).days
        assignees = ",".join(a["login"] for a in issue["assignees"]["nodes"]) or "-"
        parent = issue["parent"]
        subs = issue["subIssuesSummary"]

        print(f"## #{n} [{created[:10]}, {age}d] {issue['title']}")
        print(f"labels: {' '.join(labels_of(issue))} | assignee: {assignees}")
        graph = [
            f"parent: #{parent['number']} ({parent['state'].lower()})" if parent else "parent: -",
            f"blocked by open: {numbers(issue['blockedBy']['nodes'], 'OPEN')}",
            f"blocking open: {numbers(issue['blocking']['nodes'], 'OPEN')}",
        ]
        if subs["total"]:
            graph.append(f"subs: {subs['completed']}/{subs['total']} done")
        print("graph: " + " · ".join(graph))

        seams = section(body, "Seams to decide")
        if seams:
            print(f"seams: {one_line(seams)}")
        comments = issue["comments"]
        if comments["nodes"]:
            last = comments["nodes"][0]
            print(f"last comment ({last['createdAt'][:10]}, {comments['totalCount']} total): {one_line(last['body'], 240)}")

        flags = []
        cite = re.compile(rf"#{n}(?!\d)")
        citing = [f"{h} {text.splitlines()[0]}" for h, date, text in commits if date >= created and cite.search(text)]
        if citing:
            flags.append("commits cite it: " + "; ".join(citing[:5]) + (f" (+{len(citing) - 5})" if len(citing) > 5 else ""))
        adrs = sorted({m for m in re.findall(r"ADR[- ]?(\d{4})", body) if adr_changed.get(m, "") >= created[:10]})
        if adrs:
            flags.append("cited ADRs changed since: " + ", ".join(f"ADR-{a} ({adr_changed[a]})" for a in adrs))
        missing = absent_paths(body, tracked)
        if missing:
            flags.append("paths not in the tree: " + ", ".join(f"`{p}`" for p in missing))
        named_closed = sorted({int(m) for m in re.findall(r"#(\d+)", body) if int(m) in closed and int(m) != n})
        if named_closed:
            flags.append("names closed: " + " ".join(f"#{m}" for m in named_closed))
        print("flags: " + (" | ".join(flags) if flags else "none"))

        labels_section = section(body, "Labels")
        if labels_section:
            print("Labels:")
            for line in labels_section.splitlines():
                if line.strip():
                    print("  " + line.strip())
        print()


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        sys.exit(f"{' '.join(error.cmd[:3])} failed: {error.stderr.strip()}")
