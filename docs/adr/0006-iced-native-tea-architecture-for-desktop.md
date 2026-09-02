# Reuse the hybrid TEA/component architecture on `iced`'s native `Task`/`Subscription`

With `iced` chosen for the desktop GUI (ADR-0005), we're carrying ADR-0003's hybrid
TEA/component architecture over to `bin-desktop` rather than designing a new one: a top-level
`App` owns which screen is active and delegates `update`/`view` to a per-screen `Model`,
exactly as ADR-0003 describes for the TUI.

The plumbing changes, because `iced` provides that plumbing natively where `ratatui` didn't.
`iced`'s own `Application`/`Program` model is the Elm Architecture as-is, so instead of porting
ADR-0003's hand-rolled `tokio::select!` event loop verbatim, we express the same shape through
`iced`'s native `Task` (≈ ADR-0003's async work, built via `Task::perform()`) and
`Subscription` (≈ ADR-0003's `tokio::select!` multiplexing of tick/input/background-channel
events, as a declarative stream builder). `iced`'s own documentation treats composing multiple
screens behind one top-level `Message` enum as an idiomatic pattern, not a combination we have
to assemble from primitives the way ADR-0003 had to for `ratatui`.

This ADR supersedes ADR-0003 for the desktop client specifically. ADR-0003 remains the record
for the TUI, which keeps its own hand-rolled `tokio::select!` loop unchanged.
