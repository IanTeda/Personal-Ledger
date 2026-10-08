# Personal Ledger TUI — handoff index

Low-to-mid fidelity design references for the `bin-tui` Client, to be rebuilt in **Rust + ratatui**. Each package holds one destination: a design file (`*.dc.html`, open it in a browser from inside its folder, `support.js` beside it) and a `README.md` spec. They are references, not code to port.

## How to use
1. Build `01-chrome` first: the shell, command palette and help window that every other screen re-hosts inside.
2. Then the destinations in any order. Each README covers its frames, keyboard, state and ratatui notes.
3. [`navigation.md`](navigation.md) is the TUI's own living navigation spec; the cross-client keyboard grammar is in `docs/getting-around.md`.

## Packages

| Folder | Destination | Design file |
| --- | --- | --- |
| `01-chrome` | Chrome: shell at rest (1a no ledger, 1b dashboard), `:help`, command palette | `Ledger TUI Chrome.dc.html` |
| `02-accounts` | Accounts | `Ledger TUI Accounts.dc.html` |
| `03-categories` | Categories | `Ledger TUI Categories.dc.html` |
| `04-payees` | Payees | `Ledger TUI Payees.dc.html` |
| `05-settings` | Settings | `Ledger TUI Settings.dc.html` |
| `06-tags` | Tags | `Ledger TUI Tags.dc.html` |
| `07-units` | Units | `Ledger TUI Units.dc.html` |

The Desktop equivalents live in `docs/ux/desktop-mockups/`.
