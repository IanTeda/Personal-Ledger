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
//! The ports are `lib_units::SharedPorts`, which `LedgerPorts::units()` hands the Units with
//! usage and holdings read from the stores beside them (#887, #1084).

pub mod router;
pub mod view;

use std::rc::Rc;
use std::sync::mpsc;

use lib_config::KeyBindingConfig;
use lib_database::{Commit, FakeStores, LedgerPorts, SharedStores, Stores, WriteFailure};
use lib_shell::DialogCommand;
use lib_viper::keymap::Keymap;

/// The Units ports the Units View and its Dialogs share.
pub type TuiPorts = lib_units::SharedPorts;

/// What `Shell` builds each Units View from: the shared ports, the stores beside them, the commit
/// each input ends in, the `[keybindings]` the page Presenter builds its own keymaps from, and the
/// Dialogs' keymap.
#[derive(Clone)]
pub struct UnitsSetup {
    pub ports: TuiPorts,
    /// The stores beside the Units, whose rows Settings › Ledger and the base Unit guard count.
    pub stores: SharedStores,
    /// Settings › Ledger's reads and the Ledger's Preferences writes (`LedgerPorts::settings_ledger()`).
    pub settings: lib_settings::SettingsLedger,
    /// The commit that ends each input, held here while Units is the only module that writes.
    pub commit: Rc<dyn Commit>,
    /// The `[keybindings]` the page Presenter builds its keys from.
    pub config: KeyBindingConfig,
    pub dialog_keymap: Keymap<DialogCommand>,
    /// The Open Ledger's price fetching, `None` with no price feed (ADR-0062).
    pub fetching: Option<lib_units::SharedFetching>,
    /// The Open Ledger's Clock, which the Units View, its Router and its Dialogs read the day and
    /// the time from.
    pub clock: lib_core::Clock,
}

impl UnitsSetup {
    /// The Units of the Ledger `stores` hold, dated by `clock`, under `config`'s key bindings, their
    /// Price Source keys read with `api_key_secret`, with the writer's failed writes. The stores are shared out here so a clone of them stays
    /// beside the ports, which `LedgerPorts` keeps to itself (#1212).
    pub fn on(
        stores: Stores,
        clock: lib_core::Clock,
        config: &KeyBindingConfig,
        api_key_secret: lib_database::ApiKeySecret,
    ) -> (Self, mpsc::Receiver<WriteFailure>) {
        let shared = stores.share();
        let stores = shared.stores.clone();
        let (ledger, failures) = LedgerPorts::over(shared, clock.clone(), api_key_secret);
        let setup = Self {
            ports: ledger.units(),
            stores,
            settings: ledger.settings_ledger(),
            commit: ledger.commit(),
            config: config.clone(),
            dialog_keymap: lib_shell::dialog_keymap(config),
            fetching: None,
            clock,
        };
        (setup, failures)
    }

    /// The sample seed in memory, for a `Shell` built without storage.
    pub fn sample(config: &KeyBindingConfig) -> Self {
        let today = chrono::Local::now().date_naive();
        let (stores, _) = Stores::fake(FakeStores::sample(today));
        Self::on(
            stores,
            lib_core::Clock::fixed(today),
            config,
            lib_database::ApiKeySecret::in_memory(None),
        )
        .0
    }

    /// Ends the input just handled, if it wrote anything, as one transaction.
    pub fn commit(&self) {
        self.commit.commit();
    }
}
