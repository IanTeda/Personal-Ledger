# Personal Ledger TUI

Feasibility demos for the Personal Ledger TUI client (see the "TUI App feasibility" Wayfinder
map, GitHub issue #7, and `docs/product-requirements.md` FC-TUI-001–005). Currently six
screens: line, doughnut, candlestick, and divergent charts (dummy data), a table (dummy data),
and a live view against a real embedded SQLite store.

## Running in development

From the repo root:

```sh
cargo run --package bin_tui
```

Keybindings: `Tab` / `Shift+Tab` to switch screens, `q` or `Esc` to quit.

## Building and running the Linux AppImage

Packaging uses [`cargo-packager`](https://github.com/crabnebula-dev/cargo-packager), per
[ADR-0004](../../../docs/adr/0004-cargo-packager-plus-hand-rolled-windows-zip.md). AppImage is
the only Linux format produced — no `.deb` — because AppImage runs with no installation step
and no root at all (see the ADR for why).

1. Install `cargo-packager` once (not part of `mise.toml` — it's a build-time-only tool, not
   needed for normal development):

   ```sh
   cargo install cargo-packager --locked
   ```

2. Build and package, from the repo root:

   ```sh
   cargo packager --release --formats appimage --manifest-path crates/bins/bin_tui/Cargo.toml
   ```

   This produces `target/release/tui_0.1.0_x86_64.AppImage` (version number tracks the
   `version` field in `Cargo.toml`).

3. Run it — no `sudo`, no installer, no package manager involved:

   ```sh
   chmod +x target/release/tui_0.1.0_x86_64.AppImage
   ./target/release/tui_0.1.0_x86_64.AppImage
   ```

   If your system doesn't have FUSE available (AppImage's usual mount mechanism — some minimal
   or sandboxed environments don't), fall back to extract-and-run, which needs no FUSE at all:

   ```sh
   ./target/release/tui_0.1.0_x86_64.AppImage --appimage-extract-and-run
   ```

`crates/bins/bin_tui/icons/icon-256.png` is a placeholder (a plain coloured square, no branding) —
AppImage packaging requires a square icon to exist; swap it for real artwork whenever the app
gets any.
