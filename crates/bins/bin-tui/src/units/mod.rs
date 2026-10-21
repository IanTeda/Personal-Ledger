//! The Units module's TUI half (`docs/development/viper-architecture.md`): the Dialogs' View and
//! `impl UnitsRouter`, with the ports and store behind them. Every rule, every piece of text and
//! the keymap are `lib-units`'; the Units View (`view/units.rs`) and this folder turn crossterm
//! keys into Presenter events and draw the view models, as the Desktop's `units/` does in gpui.
//!
//! - `dialog.rs` hosts each Dialog's Presenter and draws it as a floating window.
//! - `router.rs` is the Router, which collects what a Presenter asked for.
//! - `ports.rs` holds the halfway shims: usage and holdings answered by the stub Ledger (#746).
//! - `store.rs` is where the Units are kept: the SQLite adapter `main` opens, or memory.

pub mod dialog;
pub mod ports;
pub mod router;
pub mod store;

use lib_config::KeyBindingConfig;
use lib_ledger::Ledger;
use lib_units::keymap::{Keymap, units_page_keymap};
use lib_units::presenter::UnitsPageEvent;

pub use ports::{LedgerBackedPorts, TuiPorts};
pub use store::UnitsStore;

/// What `Shell` builds each Units View from: the shared ports and the keymap under
/// `[keybindings]`.
#[derive(Clone)]
pub struct UnitsSetup {
    pub ports: TuiPorts,
    pub keymap: Keymap<UnitsPageEvent>,
}

impl UnitsSetup {
    pub fn new(store: Box<dyn UnitsStore>, config: &KeyBindingConfig) -> Self {
        Self {
            ports: LedgerBackedPorts::shared(store, Ledger::stub()),
            keymap: units_page_keymap(config),
        }
    }

    /// The stub seed in memory, for a `Shell` built without storage.
    pub fn stub(config: &KeyBindingConfig) -> Self {
        Self::new(
            Box::new(lib_units::fake::InMemoryPorts::stub(chrono::Utc::now())),
            config,
        )
    }
}
