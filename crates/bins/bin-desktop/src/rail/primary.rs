//! The primary rail (`docs/ux/desktop/Shell & Navigation/README.md`'s "1a" spec, "Primary
//! rail" component): grouped noun rows, jump-key column, the Accounts row's own count badge --
//! and its collapsed "1c" state (52px, icon-only boxes, row-anchored hover tooltip), issue
//! #152.
//!
//! Letter-spacing (`.11em` on the group headings) isn't rendered: `gpui` 0.2.2's `Styled`
//! trait has no letter-spacing property to set. Everything else in this component is exact.

use std::rc::Rc;

use gpui::{App, BoxShadow, Window, div, point, prelude::*, px};
use gpui_component::Sizable;

use crate::{
    icon::DesktopIcon,
    nav::{Noun, RailMode},
    theme::color,
};

/// Fixed column width: `docs/ux/desktop/Shell & Navigation/README.md`'s "Layout" table.
pub const WIDTH: gpui::Pixels = px(206.0);

/// The collapsed ("1c") rail's own fixed width and per-row icon box, from the handoff's own
/// collapsed-rail markup (`docs/ux/desktop/Shell & Navigation/Ledger Desktop Shell.dc.html`,
/// card `1c`) -- the README's own "1a-1d" prose is silent on these pixels, so the canvas
/// itself is the literal source here.
pub const COLLAPSED_WIDTH: gpui::Pixels = px(52.0);
const ROW_BOX: gpui::Pixels = px(36.0);
const ROW_HEIGHT: gpui::Pixels = px(34.0);
/// Gap between a collapsed row's own right edge and its tooltip -- `left: calc(100% + 12px)`
/// in the handoff's markup.
const TOOLTIP_GAP: gpui::Pixels = px(12.0);

struct Row {
    noun: Noun,
    label: &'static str,
    jump_key: Option<&'static str>,
}

/// LEDGER now folds what used to be a separate RECORDS group (Categories, Payees) in with it,
/// plus the new `Tags` noun -- the handoff dropped the RECORDS heading entirely rather than
/// keep a group of two once Units moved out (see `Noun`'s own doc). `Transactions` moves off
/// `g t` to free it for `Tags`, onto `g l` instead.
const LEDGER_GROUP: &[Row] = &[
    Row {
        noun: Noun::Dashboard,
        label: "Dashboard",
        jump_key: Some("g d"),
    },
    Row {
        noun: Noun::Transactions,
        label: "Transactions",
        jump_key: Some("g l"),
    },
    Row {
        noun: Noun::Accounts,
        label: "Accounts",
        jump_key: Some("g a"),
    },
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
        noun: Noun::Tags,
        label: "Tags",
        jump_key: Some("g t"),
    },
];

const PLAN_GROUP: &[Row] = &[
    Row {
        noun: Noun::Bills,
        label: "Bills",
        jump_key: Some("g w"),
    },
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

const SETTINGS_ROW: Row = Row {
    noun: Noun::Settings,
    label: "Settings",
    jump_key: Some("g s"),
};

/// A row's hover-enter/leave, reported with the row's own `Noun` -- `Shell` owns the 500ms
/// reveal delay and which row (if any) has settled into "tooltip visible" (see
/// `Shell::handle_rail_hover`); this component only reports raw hover transitions. `Rc` since
/// one instance is cloned into every collapsed row's own `on_hover` closure.
pub type OnRowHover = Rc<dyn Fn(Noun, bool, &mut Window, &mut App)>;

/// A row's own click, reported with the row's own `Noun` -- `Shell::handle_rail_click` is what
/// actually navigates (a direct `NavState::set_noun`, the same call `g`-jump and the palette
/// both make, so all three land in the same state per acceptance criterion 1). `Rc` for the
/// same reason as `OnRowHover`: one instance, cloned into every row's own `on_click` closure.
pub type OnRowClick = Rc<dyn Fn(Noun, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct PrimaryRail {
    /// The row drawn selected -- `NavState::primary_highlight`, not `noun` directly, so
    /// browsing with `j`/`k`/`gg`/`G` updates this rail before `Enter` commits it (see
    /// `nav::NavState`'s "Primary rail highlight vs. selection" doc).
    active: Noun,
    /// Whether `FocusZone::PrimaryRail` is the shell's current focus -- draws the 2px ink
    /// inner edge on this rail's border-facing (right) side when `true`.
    focused: bool,
    mode: RailMode,
    /// The row (if any) whose hover has settled past the 500ms reveal delay -- only ever
    /// `Some` while `mode` is `Collapsed`, since the expanded rail draws no tooltip at all.
    tooltip_target: Option<Noun>,
    on_row_hover: OnRowHover,
    on_row_click: OnRowClick,
    /// The Accounts row's count badge -- the live number of accounts, not a fixed stub.
    account_count: usize,
}

impl PrimaryRail {
    pub fn new(
        active: Noun,
        focused: bool,
        mode: RailMode,
        tooltip_target: Option<Noun>,
        on_row_hover: OnRowHover,
        on_row_click: OnRowClick,
    ) -> Self {
        Self {
            active,
            focused,
            mode,
            tooltip_target,
            on_row_click,
            on_row_hover,
            account_count: crate::rail::context::account_count(),
        }
    }

    /// Overrides the Accounts row's badge with the real account count.
    pub fn account_count(mut self, count: usize) -> Self {
        self.account_count = count;
        self
    }
}

impl RenderOnce for PrimaryRail {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        match self.mode {
            RailMode::Expanded => self.render_expanded().into_any_element(),
            RailMode::Collapsed => self.render_collapsed().into_any_element(),
        }
    }
}

impl PrimaryRail {
    fn render_expanded(&self) -> impl IntoElement {
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
            .child(div().flex_1())
            .child(div().h(px(2.0)).my(px(8.0)).bg(color::RAIL_DIVIDER))
            .child(self.render_row(&SETTINGS_ROW))
    }

    /// The "1c" collapsed rail: 52px, centred icon-only boxes, no group headings or jump-key
    /// column (nothing left to letter-space or align once labels are gone). Every noun still
    /// renders, in the same row order as the expanded rail -- the handoff's own collapsed
    /// mockup happens to draw a shorter representative subset, but every other "representative
    /// content" screen in this handoff is explicitly partial, not a row count to match exactly.
    ///
    /// Dropping the group headings (LEDGER/PLAN) also drops the only thing separating one
    /// group from the next, so a thin rule (`collapsed_divider`) stands in their place between
    /// groups -- and, mirroring the expanded rail's own divider before `SETTINGS_ROW`, above
    /// Settings too, keeping the same vertical rhythm the headings established.
    fn render_collapsed(&self) -> impl IntoElement {
        div()
            .w(COLLAPSED_WIDTH)
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .py(px(12.0))
            .gap(px(2.0))
            .bg(color::CHROME)
            .border_r(px(2.0))
            .border_color(if self.focused {
                color::INK
            } else {
                color::STRUCTURAL_RULE
            })
            .children(
                LEDGER_GROUP
                    .iter()
                    .map(|row| self.render_collapsed_row(row)),
            )
            .child(collapsed_divider())
            .children(PLAN_GROUP.iter().map(|row| self.render_collapsed_row(row)))
            .child(div().flex_1())
            .child(collapsed_divider())
            .child(self.render_collapsed_row(&SETTINGS_ROW))
    }

    fn render_collapsed_row(&self, row: &Row) -> impl IntoElement {
        let selected = row.noun == self.active;
        let icon_color = if selected {
            color::INK_ON_DARK
        } else {
            color::INK
        };

        let badge = (row.noun == Noun::Accounts).then(|| {
            div()
                .absolute()
                .top(px(2.0))
                .right(px(1.0))
                .w(px(6.0))
                .h(px(6.0))
                .bg(color::ACCENT)
        });

        let noun = row.noun;
        let on_hover = self.on_row_hover.clone();
        let on_click = self.on_row_click.clone();
        let show_tooltip = self.tooltip_target == Some(noun);

        div()
            .id(gpui::SharedString::from(format!(
                "primary-rail-collapsed-{noun:?}"
            )))
            .w(ROW_BOX)
            .h(ROW_HEIGHT)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .when(selected, |this| this.bg(color::INK))
            .when(!selected, |this| {
                this.hover(|this| this.bg(color::HOVER_TINT))
            })
            .on_hover(move |hovered, window, cx| on_hover(noun, *hovered, window, cx))
            .on_click(move |_event, window, cx| on_click(noun, window, cx))
            .child(
                DesktopIcon::from(row.noun)
                    .icon()
                    .with_size(px(16.0))
                    .text_color(icon_color),
            )
            .children(badge)
            // `gpui::deferred`, not a plain child: the tooltip is nested inside the *first*
            // rail in the shell's row (`Shell::render`'s `PrimaryRail` -> `ContextRail` ->
            // `View` order), so painting it in normal tree order would let a later sibling
            // (the context rail) paint over it and clip it right back -- precisely the bug
            // issue #152 says the handoff calls out not to reintroduce. `deferred` delays its
            // paint until after every ancestor's other children, guaranteeing it renders on
            // top regardless of where it sits in the tree.
            .when(show_tooltip, |this| {
                this.child(gpui::deferred(collapsed_tooltip(row)))
            })
    }

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

        let badge = (row.noun == Noun::Accounts).then(|| {
            div()
                .bg(color::ACCENT)
                .text_color(color::INK_ON_DARK)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .py(px(1.0))
                .px(px(5.0))
                .mr(px(4.0))
                .child(self.account_count.to_string())
        });

        let noun = row.noun;
        let on_click = self.on_row_click.clone();

        div()
            .id(gpui::SharedString::from(format!("primary-rail-{noun:?}")))
            .cursor_pointer()
            .when(selected, |this| this.bg(color::INK))
            .when(!selected, |this| {
                this.hover(|this| this.bg(color::HOVER_TINT))
            })
            .on_click(move |_event, window, cx| on_click(noun, window, cx))
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

/// The collapsed rail's own hover tooltip: anchored to the row itself (`left: calc(100% +
/// 12px)`, vertically centred), not `gpui`'s cursor-anchored `.tooltip()` -- issue #152 calls
/// out that cursor-anchoring was a live clipping bug in an earlier mockup draft and the
/// handoff insists on anchoring "to the row itself" instead. Vertical centring is done by
/// stretching this wrapper to the row's own full height (`top_0()`/`bottom_0()`) and letting
/// flex `items_center` centre the pill inside it, rather than a fixed offset that would assume
/// a tooltip content height we don't know in advance.
fn collapsed_tooltip(row: &Row) -> impl IntoElement {
    div()
        .absolute()
        .left(ROW_BOX + TOOLTIP_GAP)
        .top(px(0.0))
        .bottom(px(0.0))
        .flex()
        .items_center()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .whitespace_nowrap()
                .bg(color::INK)
                .text_color(color::INK_ON_DARK)
                .text_size(px(12.0))
                .py(px(5.0))
                .px(px(10.0))
                .shadow(vec![BoxShadow {
                    color: color::PALETTE_SHADOW.into(),
                    offset: point(px(0.0), px(3.0)),
                    blur_radius: px(10.0),
                    spread_radius: px(0.0),
                }])
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child(row.label),
                )
                .when_some(row.jump_key, |this, key| {
                    this.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(color::INK_ON_DARK_SECONDARY)
                            .child(key),
                    )
                }),
        )
}

/// The collapsed rail's own stand-in for a group heading: a thin rule the width of a row's
/// own icon box, so it lines up with the rows above and below it rather than spanning the
/// full 52px column. The flex column's own `gap(px(2.0))` already gives it breathing room on
/// both sides -- no extra margin needed.
fn collapsed_divider() -> impl IntoElement {
    div().w(ROW_BOX).h(px(2.0)).bg(color::RAIL_DIVIDER)
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
