# Colour Theme, Colour Appearance and date style are Client-scoped Preferences

ADR-0014 made the display Preferences Ledger-scoped, kept in the Ledger file's `preferences` row and synced, and ADR-0023 kept Colour Theme and Colour Appearance there. A Client can now run with no Ledger open (#872): it starts that way under the On start Preference, and returns to it on `:close`. With no Ledger open there is no `preferences` row to read, yet the Shell still draws, and the Settings pages for this device, Display among them, must be shown and changed.

We're making **Colour Theme**, **Colour Appearance** and **date style** Client-scoped Preferences: kept locally by each Client, beside Toasts (ADR-0027) and On start, never synced, and applying whichever Ledger is open or none. They stay nullable with the same defaults (Modernist; System; the Locale's default), and ADR-0023's resolution order and `[theme]` overrides are unchanged. The base Unit stays Ledger-scoped, since it is Ledger data. Settings groups its pages by scope, This device and This Ledger, and Display is wholly a This device page.

## Considered options

Keeping them Ledger-scoped and drawing the defaults with no Ledger open was rejected: the Client would change colour as a Ledger opens and closes, and Display would have to be refused with no Ledger open or split across both groups. A Client-scoped value that a Ledger may override was rejected as two sources of truth for one colour. The cost accepted is that a theme no longer follows the user to their other Clients by sync; the Locale, also per Client, already works that way.
