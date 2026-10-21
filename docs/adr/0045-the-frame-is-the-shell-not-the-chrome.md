# The frame is the Shell, not the Chrome

GLOSSARY.md called the always-present frame around the current View the Chrome, and told readers to avoid "shell" because it meant the whole application. ADR-0032, ADR-0037 and ADR-0042 use Chrome in that sense, and the Desktop code has a `chrome/` module inside a `Shell` struct. Decided while grilling `docs/development/viper-architecture.md`, which makes the frame a VIPER module named `lib-shell`.

## Decision

- **The frame is the Shell.** The GLOSSARY entry is renamed from Chrome to Shell, with the same definition: the top bar, the rails, the status line, the command palette, Toasts and the Dialog host.
- **Chrome is an Avoid term.** It is a web-browser term and reads as jargon to anyone outside web design.
- **The whole application is the Client.** That term already exists, so "shell" no longer needs to mean it.
- **Older ADRs keep their wording.** In ADR-0032, ADR-0037 and ADR-0042, "Chrome" means what is now the Shell. Where they say `Shell`, they mean today's struct that owns it.
- **Code follows in the transition tickets.** `bin-desktop/src/chrome/` and `ChromeState` are renamed when the Shell becomes the `lib-shell` module.

## Considered Options

- **Keep Chrome and name the crate `lib-chrome`.** Rejected: Shell is the more familiar word for an application's frame, and the crate name should match the term.
