# `cargo-packager` for macOS and Linux, a hand-rolled zip for Windows, no signing yet

The TUI feasibility cycle (`docs/product-requirements.md` FC-TUI-004) needs non-root packaging
on Windows, macOS, and Linux: a portable single `.exe` with no installer or elevation on
Windows, a real user-scope installer/bundle on macOS, and a real user-scope installable
package on Linux. A primary-source survey (see `docs/research/tui-packaging-tools.md`)
compared `cargo-dist`, `cargo-packager`, `cargo-bundle`, and hand-rolled CI scripting. No
single dedicated tool covers all three outputs cleanly: `cargo-dist` has by far the strongest
CI story but no native macOS or Linux packaging at all, not even on its public roadmap;
`cargo-packager` and `cargo-bundle` both natively cover all three categories, but their
Windows output is an installer, not a bare portable exe, which doesn't meet the Windows
requirement as specified.

We chose a **hybrid**: `cargo-packager` for macOS (`.dmg`/`.app`) and Linux (AppImage), where
it is the strongest-fitting tool — natively covering both formats, backed by a company
(CrabNebula) with a real release cadence — plus a hand-rolled zip step for Windows, since a
portable exe needs no packaging tool at all; it's just the raw `cargo build --release` output
compressed. This avoids `cargo-dist`'s missing macOS/Linux coverage and `cargo-bundle`'s
single-maintainer bus-factor and unstable config format, without paying for a fully hand-rolled
pipeline across all three platforms. `cargo-packager` has no official GitHub Action, so its
macOS/Linux steps will be hand-written into the CI matrix (ticket #13) alongside the trivial
Windows zip step.

For Linux, we chose **AppImage only**, not AppImage plus a secondary `.deb`. Debian Policy
requires installed package files to be owned `root:root` and installed under `/usr` via
`apt`/`dpkg` — in real tension with NFR.2's "never requires root." AppImage sidesteps this by
design (no installation step at all, just `chmod a+x` and run). Shipping a `.deb` even as a
clearly-labelled secondary option would mean shipping an artifact that structurally
contradicts NFR.2, which this feasibility cycle should be proving against, not alongside.
`cargo-packager` can add `.deb` support later without an architecture change if a future cycle
decides the tradeoff differently.

We're not investing in code signing or macOS notarization for this feasibility cycle.
Unsigned builds will trigger Gatekeeper's and Windows SmartScreen's override dialogs on first
run regardless of which tool builds them — a real UX cost, but an acceptable one for a
feasibility spike with no public release audience yet, given macOS signing requires an ongoing
Apple Developer Program enrollment and Windows signing requires a separate code-signing
certificate. Revisit when there's an actual release to sign.
