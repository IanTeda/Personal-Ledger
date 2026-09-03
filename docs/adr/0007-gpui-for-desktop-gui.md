# `GPUI` for the Desktop GUI framework (supersedes ADR-0005)

After ADR-0005 chose `iced`, Ian directly requested `GPUI` — Zed's own GPU-accelerated UI
framework — instead. `GPUI` wasn't part of the original seven-candidate survey
(`docs/research/desktop-gui-frameworks.md` §1–6); a follow-up primary-source check (same
document, §7 addendum) confirmed it as a genuinely standalone, permissively-licensed,
extremely actively maintained crate, and this ADR records the framework decision changing to
it.

`GPUI` (crates.io: `gpui`, Apache-2.0, 246k downloads) is backed by Zed (89.6k GitHub stars,
pushed the same day as this research), the most active repository of any candidate considered
across either survey. Its own README's "macOS or Linux" line is most likely stale — its
`Cargo.toml` has genuine `target_os = "windows"` dependencies, `windows-manifest` is a default
feature, and Zed's own GitHub has active `gpui_windows`-tagged PR traffic and lists a Windows
download — so we're treating Windows support as real but not yet independently confirmed
end-to-end; this should be smoke-tested early during the Windows-installer ticket (issue #34)
rather than assumed.

Unlike `iced`, `GPUI` does **not** satisfy the ADR-0003 architecture-fit criterion that drove
ADR-0005's `iced`/`relm4` shortlist in the first place: it has no Model/Message/`Task`/
`Subscription` scaffolding, only an `Entity`/`Context`/`Render` system where `View`s rebuild
their element tree every frame. We're accepting that trade-off in exchange for `GPUI`'s
genuinely GPU-accelerated native rendering and the scale/momentum of the project backing it;
see ADR-0006 for why the previous architecture plan no longer applies, and issue #26's
resolution comment for the follow-on decision to design a `GPUI`-native architecture during the
first chart-demo ticket rather than up front.

`GPUI`'s chart/table ecosystem (`gpui-d3rs`, `gpui-px`, `sqlly-datatable` — all real,
permissively licensed, but single-maintainer and under 1,000 downloads each) is far less
mature than `plotters`, the shared charting layer behind the `iced`/`relm4` alternative. We're
accepting that risk too, on the same direct request; if it proves inadequate during the
chart-demo tickets (issues #28–#31), custom drawing on `GPUI`'s own low-level `Element` API is
the fallback, the same pattern used throughout this map and the TUI map before it.
