# `iced` for the Desktop GUI framework

The Desktop App feasibility cycle (`docs/product-requirements.md` FC-DESKTOP-001) needs a
Rust desktop GUI framework that can render line, doughnut, candlestick, and divergent charts
plus tables, working across Linux, macOS, and Windows. A primary-source survey (see
`docs/research/desktop-gui-frameworks.md`) compared `egui`, `iced`, `Tauri`, `Slint`,
`gtk-rs`/`relm4`, `Dioxus`, and `Freya`. We chose `iced`.

Of all seven candidates, only `iced` and `relm4`/`gtk-rs` have an architecture that maps
directly onto ADR-0003's hybrid TEA/component pattern (see ADR-0006): `egui`, `Slint`,
`Dioxus`, and `Freya` have no framework-level Model/Message split, and `Tauri`'s Rust side has
no natural home for that loop at all, since it would have to live in whichever web frontend
framework is chosen instead of in Tauri's own code. Between `iced` and `relm4`, we picked
`iced` for its much bigger and more active community (31.4k vs. 2.0k GitHub stars, both
actively pushed) and because it needs no extra native-toolchain setup beyond the Rust
toolchain on any of the three target OSes. `relm4`/`gtk-rs` requires installing native GTK4
(`brew install gtk4 libadwaita meson desktop-file-utils` on macOS; a Windows MSVC/GNU
toolchain choice plus `gvsbuild` or pre-built packages) — a real added cost across the
three-OS CI matrix FC-DESKTOP-004 requires.

Neither `iced` nor `relm4` has a built-in doughnut or candlestick chart; both would bridge to
`plotters`, the only surveyed charting layer with native doughnut (`donut_hole()`) and
candlestick support — but `iced`'s own bridge crate, `plotters-iced`, has been stale for
roughly two years (last published 2024-09-18), while `relm4`'s `plotters-cairo` bridge is
actively maintained. We're accepting that gap rather than switching frameworks over it: like
the TUI cycle's own missing doughnut/divergent support (ADR-0002), we expect to resolve it
with hand-rolled custom drawing on `iced`'s own `Canvas` primitive rather than depending on the
stale bridge, deciding per chart type during each demo ticket (issues #28–#31) the same way the
TUI resolved its own gaps empirically rather than up front.

`relm4`/`gtk-rs`'s native `ColumnView` table (full click-to-sort via `Sorter`/`SortListModel`)
is more complete than anything confirmed for `iced_table`, but FC-DESKTOP-003 only requires
demonstrating a working table, not sorting — the same scope the TUI's own `ratatui::Table`
demo had — so this wasn't decisive.
