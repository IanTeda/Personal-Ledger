# Personal Ledger TUI — handoff index

Low-to-mid fidelity design references for the `bin-tui` Client, to be rebuilt in **Rust + ratatui**. Each package holds one destination: a design file (`*.dc.html`, `support.js` beside it) and a `README.md` spec. They are references, not code to port.

**Viewing:** run `mise run mockups` and open <http://localhost:8002>. The design files render through `support.js`, which fetches the page over HTTP, so opening a `.dc.html` straight from disk (`file://`) shows a blank page. `index.html` lists every page, with a switch between the Desktop and TUI sets.

**Card IDs:** every card carries a `TUI-NNN` ID (the label on the card and its `#` anchor). Say "TUI-001" rather than "1a" when referring to one. The old per-file IDs used in the prose below map to these in [`../mockup-ids.md`](../mockup-ids.md).

## How to use
1. Build `01-shell` first: the shell, command palette and help window that every other screen re-hosts inside.
2. Then the destinations in any order. Each README covers its frames, keyboard, state and ratatui notes.
3. [`navigation.md`](navigation.md) is the TUI's own living navigation spec; the cross-client keyboard grammar is in `docs/getting-around.md`.

## Packages

| Folder | Destination | Design file |
| --- | --- | --- |
| `01-shell` | Shell: at rest (1a no ledger, 1b dashboard), `:help`, command palette | `Ledger TUI Shell.dc.html` |
| `02-accounts` | Accounts | `Ledger TUI Accounts.dc.html` |
| `03-categories` | Categories | `Ledger TUI Categories.dc.html` |
| `04-payees` | Payees | `Ledger TUI Payees.dc.html` |
| `05-settings` | Settings | `Ledger TUI Settings.dc.html` |
| `06-tags` | Tags | `Ledger TUI Tags.dc.html` |
| `07-units` | Units | `Ledger TUI Units.dc.html` |

The Desktop equivalents live in `docs/ux/desktop-mockups/`.
