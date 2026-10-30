//! One Presenter per Transactions View and Dialog: the page's ([`TransactionsPagePresenter`]) and
//! the filter popover's it holds while the popover is open ([`FilterPresenter`]), with the chip
//! labels and clears the page's chip row reads. Each takes events through `handle` and builds a
//! plain-data view model through `view_model`, its text already localised from [`crate::msg`], so a
//! View only draws.

mod filter;
mod transactions_page;

pub use filter::{
    ChipField, ChipView, FilterEvent, FilterField, FilterOutcome, FilterPresenter, FilterViewModel,
    status_filter_label,
};
pub use transactions_page::{
    EMPTY_CELL, FooterTotal, FooterView, Hint, RowView, TagsCell, TransactionsPageEvent,
    TransactionsPagePresenter, TransactionsPageViewModel,
};

#[cfg(test)]
pub(crate) mod test_support {
    use chrono::NaiveDate;
    use lib_core::{AccountId, CategoryId, PayeeId, TagId, UnitId};

    use crate::fake::InMemoryTransactionReads;
    use crate::{TransactionAccount, TransactionCategory, TransactionPayee, TransactionTag};

    /// Composes this crate's layer into the process Locale, which every test that reads text
    /// needs: the first explicit `init` wins, so each test calls it before reading any.
    pub fn locale() {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
    }

    /// Runs `f` in `en-AU`, whose typed dates are day-first, so a test's dates read one way.
    pub fn au<T>(f: impl FnOnce() -> T) -> T {
        locale();
        lib_locale::with_locale(lib_locale::Locale::EnAu, f)
    }

    pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    /// The day the tests run on.
    pub fn today() -> NaiveDate {
        date(2026, 9, 19)
    }

    pub const EVERYDAY: AccountId = AccountId::seed(1);
    pub const WALLET: AccountId = AccountId::seed(2);
    pub const FOOD: CategoryId = CategoryId::seed(1);
    pub const GROCERIES: CategoryId = CategoryId::seed(2);
    pub const HOUSING: CategoryId = CategoryId::seed(3);
    pub const WOOLWORTHS: PayeeId = PayeeId::seed(1);
    pub const WORK_TRIP: TagId = TagId::seed(1);

    /// Two Accounts, a Category tree of Food › Groceries and Housing, one Payee and one Tag.
    pub fn reads() -> InMemoryTransactionReads {
        let account = |id, name: &str| TransactionAccount {
            id,
            name: name.to_string(),
            unit: UnitId::SEED_AUD,
        };
        let category = |id, name: &str, parent, path: &str| TransactionCategory {
            id,
            name: name.to_string(),
            parent,
            path: path.to_string(),
        };
        InMemoryTransactionReads {
            accounts: vec![account(EVERYDAY, "Everyday"), account(WALLET, "Wallet")],
            categories: vec![
                category(FOOD, "Food", None, "Food"),
                category(
                    GROCERIES,
                    "Groceries",
                    Some(FOOD),
                    "Food \u{203a} Groceries",
                ),
                category(HOUSING, "Housing", None, "Housing"),
            ],
            payees: vec![TransactionPayee {
                id: WOOLWORTHS,
                name: "Woolworths".to_string(),
            }],
            tags: vec![TransactionTag {
                id: WORK_TRIP,
                name: "work-trip".to_string(),
                color: None,
            }],
            units: Vec::new(),
        }
    }
}
