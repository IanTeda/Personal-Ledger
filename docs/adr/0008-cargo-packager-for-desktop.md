# `cargo-packager` for the Desktop packaging tool

`GPUI` (ADR-0007) ships no bundler of its own, unlike Tauri, so this decision was a free
choice rather than a framework-forced one. A primary-source survey
(`docs/research/desktop-packaging-tools.md`) compared `cargo-packager`, `cargo-dist`,
`cargo-bundle`, `tauri-bundler`, `cargo-wix`, direct `linuxdeploy`/`appimagetool` use, and
hand-rolled CI, and — after `GPUI` was chosen — added an addendum on how Zed itself (`GPUI`'s
own home project) packages its releases: a patched `cargo-bundle` fork for macOS, a hand-rolled
tarball (plus separate Flatpak/Snap channels) for Linux, and a hand-rolled Inno Setup installer
for Windows.

We chose **`cargo-packager`** for all three platforms — macOS `.dmg`, Linux AppImage, and a
Windows NSIS installer (`CurrentUser` non-elevated by default) — the same tool the TUI cycle
already uses (ADR-0004). This keeps one packaging tool and one CI dependency across
`bin-tui` and `bin-desktop`, rather than adopting Zed's own three-different-tools-per-platform
approach. We considered mirroring Zed's precedent directly, since it's the most battle-tested
real-world path for a `GPUI` app specifically, but its complexity (a Zed-patched `cargo-bundle`
fork, a custom Inno Setup script with code-signing and Windows Explorer shell-extension
packaging, multiple Linux distribution channels) exists to serve problems Personal Ledger's
feasibility-cycle demo doesn't have — an auto-updater, install-time telemetry, extensions, and
several release channels (stable/preview/nightly). Cross-platform consistency with the
already-shipped TUI packaging outweighed matching Zed's specific tooling.

`cargo-packager`'s Linux desktop-entry support is a bare on/off flag — thinner than
`cargo-bundle`'s per-locale metadata — which we're accepting as adequate for feasibility-cycle
scope, same as this map's tolerance for `iced`'s/`GPUI`'s thinner chart-bridge ecosystems
elsewhere. Its Windows NSIS installer defaults to a non-elevated `CurrentUser` install, which
satisfies NFR.2 without extra configuration — unlike `cargo-wix`'s MSI path, which defaults to
elevated unless explicitly reconfigured.
