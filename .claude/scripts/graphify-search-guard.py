#!/usr/bin/env python3
"""PreToolUse hook: points a repo search at graphify's knowledge graph, once per tool call.

Replaces three `graphify hook-guard search` hooks (`grep -r*`, `grep -R*`, `rg *`) that matched any
part of a compound Bash command, so a `cargo test | grep` or `ssh-add -l | grep` set them off,
often three at once. This fires only for the Grep and Glob tools and for a Bash segment that
starts with a recursive grep or `rg`, never for grep reading a pipe, and names the graph where
`GRAPHIFY_OUT` puts it.
"""

import json
import os
import re
import sys

# A segment ends at `&&`, `||`, `;` or a newline; within one, only the part before a pipe searches.
SEGMENT = re.compile(r"&&|\|\||;|\n")
RECURSIVE_GREP = re.compile(r"^(?:grep|egrep)\s+(?:-\S*\s+)*-[a-zA-Z]*[rR]")
RIPGREP = re.compile(r"^rg\s")


def searches_the_repo(command: str) -> bool:
    for segment in SEGMENT.split(command):
        head = segment.split("|", 1)[0].strip()
        if RECURSIVE_GREP.match(head) or RIPGREP.match(head):
            return True
    return False


def main() -> None:
    try:
        event = json.load(sys.stdin)
    except json.JSONDecodeError:
        return
    tool = event.get("tool_name", "")
    if tool == "Bash":
        if not searches_the_repo(event.get("tool_input", {}).get("command", "")):
            return
    elif tool not in ("Grep", "Glob"):
        return

    project = os.environ.get("CLAUDE_PROJECT_DIR", os.getcwd())
    graph = os.path.join(os.environ.get("GRAPHIFY_OUT", ".graphify-out"), "graph.json")
    if not os.path.exists(os.path.join(project, graph)):
        return

    message = (
        f"Search: {graph} is current, so `graphify query \"<question>\"`, `graphify path` or "
        "`graphify explain` usually orients faster than grepping the tree. Grep to read or "
        "change specific lines."
    )
    json.dump(
        {"hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": message}},
        sys.stdout,
    )


if __name__ == "__main__":
    main()
