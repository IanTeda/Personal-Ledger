//! Where the Shell is: the [`Noun`] the primary rail selects and the [`ActiveView`] that owns the
//! keyboard, the page status and the main pane. Pure enums, moved from the Desktop's `navigation/`
//! (#820, Q5); the Clients keep what holds a toolkit's Entities and every display name, which is a
//! Message.

use serde::{Deserialize, Serialize};

/// The thirteen places the primary rail can select. `Noun::default()` is `Dashboard`, the app's
/// one home noun. Serializable, since the Desktop keeps the noun across a restart. Its display name is a
/// Message, so each Client names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Noun {
    #[default]
    Dashboard,
    Transactions,
    Documents,
    Notifications,
    Cash,
    Inventory,
    Loans,
    CreditCards,
    Investments,
    Bills,
    Budgets,
    Reports,
    Settings,
}

impl Noun {
    /// Every noun, in the primary rail's own row order (top to bottom, skipping group
    /// headings and the divider, which aren't selectable rows).
    pub const ALL: [Noun; 13] = [
        Noun::Dashboard,
        Noun::Transactions,
        Noun::Documents,
        Noun::Notifications,
        Noun::Cash,
        Noun::Inventory,
        Noun::Loans,
        Noun::CreditCards,
        Noun::Investments,
        Noun::Bills,
        Noun::Budgets,
        Noun::Reports,
        Noun::Settings,
    ];

    #[expect(
        clippy::expect_used,
        reason = "ALL is a fixed-length array of every Noun, and row_index_round_trips_every_noun checks each one is present exactly once"
    )]
    fn row_index(self) -> usize {
        Self::ALL
            .iter()
            .position(|noun| *noun == self)
            .expect("Noun::ALL must list every Noun")
    }

    /// The next row down. Clamps at the last row rather than wrapping -- `gg`/`G` are the
    /// handoff's own way to jump straight to the first/last row, which would be redundant if
    /// `j`/`k` also wrapped.
    pub fn next_row(self) -> Noun {
        Self::ALL[(self.row_index() + 1).min(Self::ALL.len() - 1)]
    }

    /// The next row up. See [`Self::next_row`] on why this clamps instead of wrapping.
    pub fn prev_row(self) -> Noun {
        Self::ALL[self.row_index().saturating_sub(1)]
    }

    pub fn first_row() -> Noun {
        Self::ALL[0]
    }

    pub fn last_row() -> Noun {
        Self::ALL[Self::ALL.len() - 1]
    }

    /// Whether this noun's context rail has any entities at all. Only `Dashboard`, `Bills`,
    /// `Budgets` and `Reports` keep one.
    ///
    /// The handoff's own state-machine section (rule 1's parenthetical) lists `Dashboard`
    /// alongside `Settings` as having none, but its "Context rail" component spec and its
    /// own accepted mockup both show Dashboard's context rail populated with the account
    /// roll-call ("On Dashboard it shows accounts") -- a real, visible, load-bearing context
    /// rail, not an absent one. Built against the concrete mockup, not the inconsistent
    /// prose: `Dashboard`'s "first entity" (rule 1) is the first account, same shape as any
    /// other noun with entities. See `docs/ux/desktop-mockups/README.md`'s "Where this differs from
    /// the handoff".
    ///
    /// `Transactions` has none (its table fills the pane, as its mockup shows), `Settings` has
    /// its own index rail instead, and the "not yet built" placeholder nouns have nothing to
    /// list. Accounts, Categories, Payees and Tags left the rail for Settings pages (#413), and
    /// took their context rails with them.
    pub fn has_context_entities(self) -> bool {
        matches!(
            self,
            Noun::Dashboard | Noun::Bills | Noun::Budgets | Noun::Reports
        )
    }

    /// Whether this noun is a "not yet built" placeholder view (#420): the rail row and its `g`
    /// binding are live, but the surface itself is a later map's work.
    pub fn is_placeholder(self) -> bool {
        matches!(
            self,
            Noun::Notifications
                | Noun::Cash
                | Noun::Inventory
                | Noun::Loans
                | Noun::CreditCards
                | Noun::Investments
        )
    }
}

/// The View that owns the keyboard, the page status and the main pane. Derived from the [`Noun`],
/// the selected Settings page `P` and whether a CSV import is open, so a new View fails to compile
/// at every exhaustive `match` until it is handled (ADR-0016). `P` is the Client's Settings page,
/// whose names are its Messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView<P> {
    Dashboard,
    Transactions,
    /// The CSV import step, which takes over the Transactions pane while it is open.
    Import,
    Documents,
    Bills,
    Budgets,
    /// Settings' index-versus-page focus stays in Settings domain code, not in this enum.
    Settings(P),
    /// A Noun whose surface is a later map's work (#420).
    Placeholder(Noun),
}

impl<P> ActiveView<P> {
    /// The Noun this View belongs to: the CSV import is part of Transactions, and every Settings
    /// page is part of Settings.
    pub fn noun(&self) -> Noun {
        match self {
            Self::Dashboard => Noun::Dashboard,
            Self::Transactions | Self::Import => Noun::Transactions,
            Self::Documents => Noun::Documents,
            Self::Bills => Noun::Bills,
            Self::Budgets => Noun::Budgets,
            Self::Settings(_) => Noun::Settings,
            Self::Placeholder(noun) => *noun,
        }
    }

    pub fn derive(noun: Noun, settings_page: P, import_open: bool) -> Self {
        match noun {
            Noun::Dashboard => Self::Dashboard,
            Noun::Transactions if import_open => Self::Import,
            Noun::Transactions => Self::Transactions,
            Noun::Documents => Self::Documents,
            Noun::Bills => Self::Bills,
            Noun::Budgets => Self::Budgets,
            Noun::Settings => Self::Settings(settings_page),
            Noun::Notifications
            | Noun::Cash
            | Noun::Inventory
            | Noun::Loans
            | Noun::CreditCards
            | Noun::Investments
            | Noun::Reports => Self::Placeholder(noun),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Client's Settings page, as far as these tests need one.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Page {
        General,
        Units,
    }

    #[test]
    fn row_index_round_trips_every_noun() {
        for (index, noun) in Noun::ALL.into_iter().enumerate() {
            assert_eq!(noun.row_index(), index, "{noun:?} is listed more than once");
        }
    }

    #[test]
    fn only_dashboard_bills_budgets_and_reports_have_context_entities() {
        for noun in Noun::ALL {
            let expected = matches!(
                noun,
                Noun::Dashboard | Noun::Bills | Noun::Budgets | Noun::Reports
            );
            assert_eq!(noun.has_context_entities(), expected, "{noun:?}");
        }
    }

    #[test]
    fn noun_row_order_clamps_at_both_ends() {
        assert_eq!(Noun::Dashboard.prev_row(), Noun::Dashboard);
        assert_eq!(Noun::Settings.next_row(), Noun::Settings);
        assert_eq!(Noun::Dashboard.next_row(), Noun::Transactions);
        assert_eq!(Noun::Settings.prev_row(), Noun::Reports);
    }

    #[test]
    fn noun_first_and_last_row() {
        assert_eq!(Noun::first_row(), Noun::Dashboard);
        assert_eq!(Noun::last_row(), Noun::Settings);
    }

    #[test]
    fn import_only_takes_over_transactions() {
        assert_eq!(
            ActiveView::derive(Noun::Transactions, Page::General, true),
            ActiveView::Import
        );
        assert_eq!(
            ActiveView::derive(Noun::Transactions, Page::General, false),
            ActiveView::Transactions
        );
        // A stale import never leaks into another Noun.
        assert_eq!(
            ActiveView::derive(Noun::Bills, Page::General, true),
            ActiveView::Bills
        );
    }

    #[test]
    fn settings_carries_its_page_and_each_view_belongs_to_its_noun() {
        assert_eq!(
            ActiveView::derive(Noun::Settings, Page::Units, false),
            ActiveView::Settings(Page::Units)
        );
        for noun in Noun::ALL {
            assert_eq!(
                ActiveView::derive(noun, Page::General, true).noun(),
                noun,
                "{noun:?}"
            );
        }
    }

    #[test]
    fn unbuilt_nouns_are_placeholders() {
        for noun in [
            Noun::Notifications,
            Noun::Cash,
            Noun::Inventory,
            Noun::Loans,
            Noun::CreditCards,
            Noun::Investments,
            Noun::Reports,
        ] {
            assert_eq!(
                ActiveView::derive(noun, Page::General, false),
                ActiveView::Placeholder(noun)
            );
        }
    }
}
