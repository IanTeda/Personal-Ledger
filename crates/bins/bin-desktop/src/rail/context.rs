//! The context rail (`docs/ux/desktop/Shell & Navigation/README.md`'s "1a" spec, "Context
//! rail" component): scoped to the active noun. Only Dashboard's own content (the account
//! roll-call) is real -- every other noun with entities (rule 4: everything except Dashboard
//! and Settings) gets a placeholder frame until its own view lands (issue #153). A noun with
//! no entities renders no context rail at all; `Shell` is what skips calling this, not this
//! module (rule 4 lives in `NavState::noun`, not here).

use gpui::{App, Window, div, prelude::*, px};

use crate::{nav::Noun, theme::color};

/// Fixed column width: `docs/ux/desktop/Shell & Navigation/README.md`'s "Layout" table.
pub const WIDTH: gpui::Pixels = px(238.0);

struct Account {
    name: &'static str,
    /// Pre-formatted, tabular-nums-ready, sign already resolved to U+2212 where negative --
    /// representative content, not a real `lib_core::Money`.
    balance: &'static str,
    negative: bool,
    meta: &'static str,
}

/// The Dashboard's own account roll-call. Representative content, matching the handoff's own
/// mockup -- real `lib_database::Accounts` data is out of scope for this map (issue #144's
/// "Out of scope").
const ACCOUNTS: &[Account] = &[
    Account {
        name: "ANZ Everyday",
        balance: "4,182.55",
        negative: false,
        meta: "bank · aud",
    },
    Account {
        name: "ANZ Offset",
        balance: "61,204.10",
        negative: false,
        meta: "bank · aud",
    },
    Account {
        name: "Amex Platinum",
        balance: "\u{2212}2,318.44",
        negative: true,
        meta: "credit card · aud",
    },
    Account {
        name: "Home Loan",
        balance: "\u{2212}381,311.34",
        negative: true,
        meta: "loan · aud",
    },
    Account {
        name: "Vanguard VAS",
        balance: "1,240 u",
        negative: false,
        meta: "investment · vas",
    },
    Account {
        name: "Cash wallet",
        balance: "240.00",
        negative: false,
        meta: "cash · aud",
    },
    Account {
        name: "Bitcoin",
        balance: "0.4120 u",
        negative: false,
        meta: "investment · btc",
    },
];

/// How many context-rail entities the given noun actually has today. Only `Dashboard`'s
/// account roll-call is real; every other noun with entities (`Noun::has_context_entities`)
/// still renders a placeholder frame with nothing to move through, so callers driving
/// keyboard movement (`Shell`) should treat `0` as "movement is a no-op here", not an error.
pub fn entity_count(noun: Noun) -> usize {
    match noun {
        Noun::Dashboard => account_count(),
        _ => 0,
    }
}

/// Total accounts -- today just `ACCOUNTS.len()`, the same figure Dashboard's own roll-call
/// shows. Exposed separately from `entity_count` so callers that want "how many accounts
/// exist" (the primary rail's own Accounts-row count badge) don't have to ask
/// `entity_count(Noun::Dashboard)` for a number that's conceptually about accounts, not
/// Dashboard.
pub fn account_count() -> usize {
    ACCOUNTS.len()
}

#[derive(IntoElement)]
pub struct ContextRail {
    noun: Noun,
    /// The row keyboard movement is currently on (`NavState::context`) -- `None` renders no
    /// row highlighted.
    context: Option<usize>,
    /// Whether `FocusZone::ContextRail` is the shell's current focus -- draws the 2px ink
    /// inner edge on this rail's border-facing (right) side, and only then tints the current
    /// row (the handoff defines no persistent "selected" row style for the context rail,
    /// only a hover tint -- reused here as the closest existing token for "this is where
    /// keyboard movement is," rather than inventing a new one).
    focused: bool,
}

impl ContextRail {
    pub fn new(noun: Noun, context: Option<usize>, focused: bool) -> Self {
        Self {
            noun,
            context,
            focused,
        }
    }
}

impl RenderOnce for ContextRail {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let frame = div()
            .w(WIDTH)
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(color::GROUND)
            .border_r(px(2.0))
            .border_color(if self.focused {
                color::INK
            } else {
                color::STRUCTURAL_RULE
            });

        match self.noun {
            Noun::Dashboard => frame
                .child(header("ACCOUNTS", &format!("{} active", ACCOUNTS.len())))
                .children(ACCOUNTS.iter().enumerate().map(|(index, account)| {
                    let current = self.focused && self.context == Some(index);
                    account_row(account, index == ACCOUNTS.len() - 1, current)
                }))
                .child(div().flex_1())
                .child(footer("+ new account", ":account new")),
            noun => frame
                .child(header(&noun_label(noun), "not yet built"))
                .child(div().flex_1()),
        }
    }
}

fn header(label: &str, count: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .pt(px(14.0))
                .pl(px(14.0))
                .pr(px(14.0))
                .pb(px(8.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(10.0))
                        .text_color(color::INK)
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(color::INK_TERTIARY)
                        .child(count.to_string()),
                ),
        )
        .child(div().h(px(2.0)).bg(color::STRUCTURAL_RULE))
}

fn account_row(account: &Account, last: bool, current: bool) -> impl IntoElement {
    div()
        .py(px(9.0))
        .px(px(14.0))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .when(current, |this| this.bg(color::HOVER_TINT))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::HAIRLINE)
        })
        .child(
            div().flex().justify_between().child(account.name).child(
                div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .when(account.negative, |this| this.text_color(color::ACCENT_TEXT))
                    .child(account.balance),
            ),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(color::INK_TERTIARY)
                .child(account.meta),
        )
}

fn footer(affordance: &'static str, command: &'static str) -> impl IntoElement {
    div()
        .py(px(10.0))
        .px(px(14.0))
        .border_t(px(1.0))
        .border_color(color::HAIRLINE)
        .text_size(px(11.5))
        .text_color(color::INK_SECONDARY)
        .flex()
        .child(format!("{affordance} · "))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::INK)
                .child(command),
        )
}

fn noun_label(noun: Noun) -> String {
    format!("{noun:?}").to_uppercase()
}
