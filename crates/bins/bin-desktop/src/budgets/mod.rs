//! The Budgets module on the Desktop: the page and every Dialog, running on `lib-budgets`'
//! Presenters (`view`, `router`), over `lib_budgets::SharedPorts`, which `LedgerPorts::budgets()`
//! hands them (#1083). The Budgets model lives in `lib_budgets`, re-exported here so callers keep
//! reaching it as `budgets::*`. The Budgets are kept in `lib-database`'s Budgets store (#952),
//! which the Desktop reads them from; the Ledger keeps no read copy (#1028).

pub mod router;
pub mod view;

pub use view::page::BudgetsPage;

use lib_budgets::{SharedPorts, presenter};

/// What the Budgets page's Router is built from for each call (its `PageView::Setup`): the ports
/// its Dialogs open on, the Clock whose day they open on and the Dialog keys their hints show.
pub struct BudgetsSetup {
    pub ports: SharedPorts,
    pub clock: lib_core::Clock,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}

impl BudgetsSetup {
    /// The page's Presenter, its keys from `config`, working its figures out on `clock`'s
    /// calendar day, and its Setup.
    pub fn new(
        ports: SharedPorts,
        clock: lib_core::Clock,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
        config: &lib_config::KeyBindingConfig,
    ) -> (presenter::BudgetsPagePresenter, Self) {
        let presenter = presenter::BudgetsPagePresenter::new(ports.clone(), clock.clone(), config);
        let setup = Self {
            ports,
            clock,
            dialog_keys,
        };
        (presenter, setup)
    }
}
