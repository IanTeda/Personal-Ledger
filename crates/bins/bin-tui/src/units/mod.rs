//! The Units module's TUI half (`docs/development/viper-architecture.md`): the Units View, its
//! Dialogs and `impl UnitsRouter`, with the ports behind them. Every rule, every piece of text and
//! the keymap are `lib-units`'; this folder turns crossterm keys into Presenter events and draws
//! the view models, as the Desktop's `units/` does in gpui.
//!
//! - `view/units.rs` is the Units View, hosted by `Shell`, and by Settings › Units, which draws
//!   the same Presenter as TUI-033's tables (`view::settings::units`).
//! - `view/dialog.rs` hosts each Dialog's Presenter and draws it as a floating window.
//! - `router.rs` is the Router, which collects what a Presenter asked for.
//! - `ports.rs` holds the ports, with usage and holdings answered by the stores opened beside the
//!   Units (#887).
//!
//! Where the Units are kept, the SQLite adapter `main` opens or memory, is `lib-database`'s
//! `UnitsStore`.

pub mod ports;
pub mod router;
pub mod view;

use std::rc::Rc;

use lib_config::KeyBindingConfig;
use lib_database::{Commit, FakeStores, Stores, UnitsStore};
use lib_ledger::Ledger;
use lib_shell::DialogCommand;
use lib_units::keymap::{unit_prices_keymap, units_page_keymap};
use lib_units::presenter::{UnitPricesEvent, UnitsPageEvent};
use lib_viper::keymap::Keymap;

pub use ports::{LedgerBackedPorts, StoredUsage, TuiPorts};

/// What `Shell` builds each Units View from: the shared ports, and the page's, the prices view's
/// and the Dialogs' keymaps under `[keybindings]`.
#[derive(Clone)]
pub struct UnitsSetup {
    pub ports: TuiPorts,
    pub keymap: Keymap<UnitsPageEvent>,
    pub prices_keymap: Keymap<UnitPricesEvent>,
    pub dialog_keymap: Keymap<DialogCommand>,
}

impl UnitsSetup {
    pub fn new(
        store: Box<dyn UnitsStore>,
        usage: StoredUsage,
        commit: Rc<dyn Commit>,
        config: &KeyBindingConfig,
    ) -> Self {
        Self {
            ports: LedgerBackedPorts::shared(store, usage, commit, Ledger::stub()),
            keymap: units_page_keymap(config),
            prices_keymap: unit_prices_keymap(config),
            dialog_keymap: lib_shell::dialog_keymap(config),
        }
    }

    /// The stub seed in memory, for a `Shell` built without storage.
    pub fn stub(config: &KeyBindingConfig) -> Self {
        let (stores, _) = Stores::fake(FakeStores::stub(chrono::Local::now().date_naive()));
        Self::on(stores, config).0
    }

    /// The Units on `stores`, with what names each Unit read from the stores beside them, and the
    /// writer's failure reports, which the Units alone don't poll.
    pub fn on(
        stores: Stores,
        config: &KeyBindingConfig,
    ) -> (Self, std::sync::mpsc::Receiver<lib_database::WriteFailure>) {
        let Stores {
            units,
            accounts,
            bills,
            inventory,
            institutions,
            commit,
            failures,
            ..
        } = stores;
        let usage = StoredUsage {
            accounts,
            bills,
            inventory,
            institutions,
        };
        (Self::new(units, usage, commit, config), failures)
    }
}
