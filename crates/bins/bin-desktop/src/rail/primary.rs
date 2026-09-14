//! The primary rail (`docs/ux/desktop/Shell & Navigation/README.md`'s "1a" spec, "Primary
//! rail" component): grouped noun rows, jump-key column, Reconcile badge. Expanded state
//! only -- the collapsed state (option 1c) is a separate ticket (#152).
//!
//! Letter-spacing (`.11em` on the group headings) isn't rendered: `gpui` 0.2.2's `Styled`
//! trait has no letter-spacing property to set. Everything else in this component is exact.

use gpui::{App, Window, div, prelude::*, px};
use gpui_component::Sizable;

use crate::{icon::DesktopIcon, nav::Noun, theme::color};

/// Fixed column width: `docs/ux/desktop/Shell & Navigation/README.md`'s "Layout" table.
pub const WIDTH: gpui::Pixels = px(206.0);

/// How many Transactions are currently unreconciled -- the Reconcile row's own badge count.
/// Representative content: nothing computes a real count until Reconcile has real data
/// (out of scope for this map, per issue #144's "Out of scope").
const UNRECONCILED_COUNT: u32 = 14;

struct Row {
    noun: Noun,
    label: &'static str,
    /// `None` for Reconcile -- deliberately unbound, per the handoff ("it's a task, not a
    /// place").
    jump_key: Option<&'static str>,
}

const LEDGER_GROUP: &[Row] = &[
    Row {
        noun: Noun::Dashboard,
        label: "Dashboard",
        jump_key: Some("g d"),
    },
    Row {
        noun: Noun::Transactions,
        label: "Transactions",
        jump_key: Some("g t"),
    },
    Row {
        noun: Noun::Accounts,
        label: "Accounts",
        jump_key: Some("g a"),
    },
    Row {
        noun: Noun::Reconcile,
        label: "Reconcile",
        jump_key: None,
    },
];

const PLAN_GROUP: &[Row] = &[
    Row {
        noun: Noun::Budgets,
        label: "Budgets",
        jump_key: Some("g b"),
    },
    Row {
        noun: Noun::Reports,
        label: "Reports",
        jump_key: Some("g r"),
    },
];

const RECORDS_GROUP: &[Row] = &[
    Row {
        noun: Noun::Categories,
        label: "Categories",
        jump_key: Some("g c"),
    },
    Row {
        noun: Noun::Payees,
        label: "Payees",
        jump_key: Some("g p"),
    },
    Row {
        noun: Noun::Units,
        label: "Units",
        jump_key: Some("g u"),
    },
];

const SETTINGS_ROW: Row = Row {
    noun: Noun::Settings,
    label: "Settings",
    jump_key: Some("g s"),
};

#[derive(IntoElement)]
pub struct PrimaryRail {
    /// The row drawn selected -- `NavState::primary_highlight`, not `noun` directly, so
    /// browsing with `j`/`k`/`gg`/`G` updates this rail before `Enter` commits it (see
    /// `nav::NavState`'s "Primary rail highlight vs. selection" doc).
    active: Noun,
    /// Whether `FocusZone::PrimaryRail` is the shell's current focus -- draws the 2px ink
    /// inner edge on this rail's border-facing (right) side when `true`.
    focused: bool,
}

impl PrimaryRail {
    pub fn new(active: Noun, focused: bool) -> Self {
        Self { active, focused }
    }
}

impl RenderOnce for PrimaryRail {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .w(WIDTH)
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .pt(px(14.0))
            .pb(px(10.0))
            .bg(color::CHROME)
            .border_r(px(2.0))
            .border_color(if self.focused {
                color::INK
            } else {
                color::STRUCTURAL_RULE
            })
            .child(group_heading("LEDGER", true))
            .children(LEDGER_GROUP.iter().map(|row| self.render_row(row)))
            .child(group_heading("PLAN", false))
            .children(PLAN_GROUP.iter().map(|row| self.render_row(row)))
            .child(group_heading("RECORDS", false))
            .children(RECORDS_GROUP.iter().map(|row| self.render_row(row)))
            .child(div().flex_1())
            .child(div().h(px(2.0)).my(px(8.0)).bg(color::RAIL_DIVIDER))
            .child(self.render_row(&SETTINGS_ROW))
    }
}

impl PrimaryRail {
    fn render_row(&self, row: &Row) -> impl IntoElement {
        let selected = row.noun == self.active;
        let (icon_color, label_color, label_weight, jump_color) = if selected {
            (
                color::INK_ON_DARK,
                color::INK_ON_DARK,
                gpui::FontWeight::EXTRA_BOLD,
                color::INK_ON_DARK_SECONDARY,
            )
        } else {
            (
                color::INK,
                color::INK,
                gpui::FontWeight::NORMAL,
                color::INK_TERTIARY,
            )
        };

        let badge = (row.noun == Noun::Reconcile && UNRECONCILED_COUNT > 0).then(|| {
            div()
                .bg(color::ACCENT)
                .text_color(color::INK_ON_DARK)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .py(px(1.0))
                .px(px(5.0))
                .mr(px(4.0))
                .child(UNRECONCILED_COUNT.to_string())
        });

        div()
            .when(selected, |this| this.bg(color::INK))
            .flex()
            .items_center()
            .gap(px(9.0))
            .py(px(7.0))
            .px(px(14.0))
            .child(
                DesktopIcon::from(row.noun)
                    .icon()
                    .with_size(px(14.0))
                    .text_color(icon_color),
            )
            .child(
                div()
                    .flex_1()
                    .text_color(label_color)
                    .font_weight(label_weight)
                    .child(row.label),
            )
            .children(badge)
            .when_some(row.jump_key, |this, key| {
                this.child(div().text_size(px(11.0)).text_color(jump_color).child(key))
            })
    }
}

fn group_heading(label: &'static str, first: bool) -> impl IntoElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_TERTIARY)
        .pl(px(14.0))
        .pr(px(14.0))
        .pb(px(8.0))
        .when(first, |this| this.pt(px(0.0)))
        .when(!first, |this| this.pt(px(16.0)))
        .child(label)
}
