# Markdown Formatting

How to format any Markdown file you write or edit in this repo (`CONTEXT.md`, `docs/adr/`, `docs/*.md`, `README.md`, etc.).

## Headings

Use ATX headings (`#`, `##`, `###`, ...), never Setext (`===`/`---` underlines). Put exactly one blank line before a heading and one blank line after it — headings never sit flush against the paragraph above or below.

## Paragraphs: no hard-wrapping

Write each paragraph as a single unwrapped line, however long. Don't insert manual line breaks partway through a paragraph to keep lines under some column width — rely on the editor's or viewer's word wrap to display it. A hard-wrapped paragraph is harder to diff (an edit to one clause reflows every line after it) and harder to grep.

This applies to list items too: one list item is one line, no matter how long, unless it contains a nested block (a sub-list, a code fence) that genuinely needs its own lines.

`docs/product-requirements.md` and this file follow this convention. `CONTEXT.md` and `docs/adr/0001-single-entry-not-double-entry.md` predate it and are still hard-wrapped at ~85 columns — don't copy that style into new content, and feel free to unwrap a paragraph in those files if you're already editing it for another reason, but don't do a drive-by reformat of a file you're not otherwise touching.

## Blank lines between blocks

Exactly one blank line between block-level elements — paragraphs, headings, lists, code fences. Never zero (elements running together) and never two or more (looks like a rendering mistake, not intentional spacing).

## Lists

Use `-` for unordered list bullets, not `*` or `+`.

## File hygiene

End every file with exactly one trailing newline, no trailing whitespace on a line.
