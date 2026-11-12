//! The Units module's TUI half (`docs/development/viper-architecture.md`): the Units View, its
//! Dialogs and `impl UnitsRouter`, with the ports behind them. Every rule, every piece of text and
//! the keymap are `lib-units`'; this folder turns crossterm keys into Presenter events and draws
//! the view models, as the Desktop's `units/` does in gpui.
//!
//! - `view/units.rs` is the Units View, hosted by `Shell`, and by Settings › Units, which draws
//!   the same Presenter as TUI-033's tables (`view::settings::units`).
//! - `view/dialog.rs` hosts each Dialog's Presenter and draws it as a floating window.
//! - `router.rs` is the Router, which collects what a Presenter asked for.
//!
//! The ports are `lib-database`'s `StoredUnitsPorts`, which `LedgerPorts` hands the Units with
//! usage and holdings read from the stores beside them (#887, #1084).

pub mod router;
pub mod view;

use std::rc::Rc;

use lib_config::KeyBindingConfig;
use lib_database::{Commit, FakeStores, LedgerPorts, SharedStores, Stores};
use lib_shell::DialogCommand;
use lib_units::keymap::{unit_prices_keymap, units_page_keymap};
use lib_units::presenter::{UnitPricesEvent, UnitsPageEvent};
use lib_viper::keymap::Keymap;

/// The Units ports the Units View and its Dialogs share.
pub type TuiPorts = lib_units::SharedPorts;

/// What `Shell` builds each Units View from: the shared ports, the stores beside them, the commit
/// each input ends in, and the page's, the prices view's and the Dialogs' keymaps under
/// `[keybindings]`.
#[derive(Clone)]
pub struct UnitsSetup {
    pub ports: TuiPorts,
    /// The stores beside the Units, whose rows Settings › Ledger and the base Unit guard count.
    pub stores: SharedStores,
    /// The commit that ends each input, held here while Units is the only module that writes.
    pub commit: Rc<dyn Commit>,
    /// The `[keybindings]` the page Presenter builds its keys from.
    pub config: KeyBindingConfig,
    pub keymap: Keymap<UnitsPageEvent>,
    pub prices_keymap: Keymap<UnitPricesEvent>,
    pub dialog_keymap: Keymap<DialogCommand>,
}

impl UnitsSetup {
    /// The Units `ledger` holds, under `config`'s key bindings.
    pub fn on(ledger: &LedgerPorts, config: &KeyBindingConfig) -> Self {
        Self {
            ports: ledger.units.clone(),
            stores: ledger.stores.clone(),
            commit: ledger.commit.clone(),
            config: config.clone(),
            keymap: units_page_keymap(config),
            prices_keymap: unit_prices_keymap(config),
            dialog_keymap: lib_shell::dialog_keymap(config),
        }
    }

    /// The sample seed in memory, for a `Shell` built without storage.
    pub fn sample(config: &KeyBindingConfig) -> Self {
        let today = chrono::Local::now().date_naive();
        let (stores, _) = Stores::fake(FakeStores::sample(today));
        Self::on(
            &LedgerPorts::new(stores, lib_core::Clock::fixed(today)),
            config,
        )
    }

    /// Ends the input just handled, if it wrote anything, as one transaction.
    pub fn commit(&self) {
        self.commit.commit();
    }
}
