# AI Connections are per-Client Configuration, and Assistant Access starts read only

Settings › AI Connections (Desktop 16v/16w–16y, TUI-034) covers two features that move Ledger data across the Client boundary, in opposite directions:

- **Model Providers** are language-model services a Client sends a request *to*, for example Claude, OpenAI, or a local Ollama.
- **Assistant Access** is a local MCP server that external **Assistants**, such as Claude Desktop and Claude Code, connect *to* in order to read the Ledger.

We decided that both are **Configuration on each Client, never synced**, and that Assistant Access ships **read only**. Read & write is deferred to its own ADR.

Per-Client scope follows from what these settings are. API keys belong in each machine's OS keychain, and a Model Provider such as a local Ollama at `localhost:11434` only means something on the machine that runs it. The MCP server listens on one machine's loopback address. A connected Assistant's token is a credential for that one listener. Syncing the provider list without its keys would put entries on other Clients that can't work there, and syncing keys would spread secrets through the Sync Server's Change Set store. Each Client therefore sets up its own connections, as it does for the Sync Server's URL and token.

Read only first, because writes from an Assistant raise questions the first version doesn't need to answer. A write needs an approval step in a Client that may not be in front of the user. It has to be attributed in History, and it has to go through the normal Change Set path so `u` undoes it on every Client. Reading the Ledger delivers most of the value (questions, reports, finding transactions) with none of that risk.

## Considered Options

- **Sync the Model Provider list as a Ledger-scoped Preference, keys local.** Rejected for the reason above: entries without working keys or reachable endpoints on other Clients.
- **Read & write with per-change approval, as the mockup draws it.** Deferred, not rejected. The Access control stays in the design, but only Read only is offered until a follow-up ADR settles approval, attribution and undo.
- **Read & write with no approval, relying on undo.** Rejected. Silent bulk edits from an external agent are a poor default for financial records, even when each one can be undone.

## Consequences

- **No Ledger data leaves the Client by default** (NFR.2). A Model Provider receives only what the user explicitly selects for an AI action. Assistant Access is off until the user turns it on and adds an Assistant.
- **Secrets live in the OS keychain.** Model Provider API keys and Assistant tokens are stored there, and other Configuration holds only references to them. An Assistant's token is shown once, when it is created. Losing it means Revoke, then Add again. Revoking ends that Assistant's session immediately.
- **The MCP server binds to `127.0.0.1` only**, on a configurable port (default 7421), and runs only while its Client is running. If another Client on the same machine already holds the port, the page says so instead of failing silently. The server belongs in `lib-mcp`, today an empty scaffold.
- **Test** checks a Model Provider's key and model before Save is enabled, the same pattern as the Sync Server dialog (16r/16s).
- **Still undecided:** which features call a Model Provider (for example Category suggestions, or reading a Document's Extracted Facts), and what exactly the read-only MCP surface exposes. Each needs its own design before it's built.
