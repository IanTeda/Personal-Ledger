# Toasts off is a Client-scoped Preference, and Errors always toast

The user can turn Toasts off (see Toast and Toast Kind in `CONTEXT.md`). Whether Toasts are welcome depends on the device and its screen (a roomy Desktop window against a small TUI terminal), not on the Ledger, and every existing Preference is Ledger-scoped.

We're making **Toasts** an on/off **Client-scoped Preference**, default on: the first Client-scoped Preference, stored locally and never synced. Until the Settings screens read and write Preferences it is held in memory, like the Colour Theme Preferences. It is one switch, not one per Toast Kind, and it never silences an **Error**: with Toasts off, Info, Success and Warning go to a **status-line echo** instead, and Errors still toast. Turning Toasts off removes the showing Info, Success and Warning Toasts, and the newest moves to the echo for the rest of its lifetime.

The echo shows the Toast Kind's glyph in its mark colour and then the Message, only while Toasts are off (and in a TUI view under 40×8, where no Toast boxes are drawn), so the same text never shows twice. On the status line it sits below the command echo and the status-line message, and above a page's own legend or the shell hint strip, which it replaces on the left; the right-hand side is untouched. It lasts as long as the Toast would have, and an Error echo stays until dismissed. Every Toast enters the session Toast history whatever the setting.

The control is a "Toasts" On/Off row in each Client's Settings Display group, above the Colour Theme group, with a note that Errors still show, plus `toasts on` / `toasts off` palette commands.

## Considered options

A Ledger-scoped Preference was rejected because turning Toasts off on one device would turn them off on all of them. Configuration (`lib-config`) was rejected because the setting is changed from inside a running Client. A switch per Toast Kind was rejected as four settings for little gain, and allowing Errors off was rejected because a failed sync or refused write would then be lost silently. Echoing the latest Toast even while Toasts are on was rejected as showing the same text twice.
