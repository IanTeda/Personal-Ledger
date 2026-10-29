//! The Units module's TUI half (`docs/development/viper-architecture.md`): the Units View, its
//! Dialogs and `impl UnitsRouter`, with the ports behind them. Every rule, every piece of text and
//! the keymap are `lib-units`'; this folder turns crossterm keys into Presenter events and draws
//! the view models, as the Desktop's `units/` does in gpui.
//!
//! - `view/units.rs` is the Units View, hosted by `Shell`, and by Settings › Units, which draws
//!   the same Presenter as TUI-033's tables (`view::settings::units`).
//! - `view/dialog.rs` hosts each Dialog's Presenter and draws it as a floating window.
//! - `router.rs` is the Router, which collects what a Presenter asked for.
//! - `ports.rs` holds the halfway shims: usage and holdings answered by the stub Ledger (#746).
//!
//! Where the Units are kept, the SQLite adapter `main` opens or memory, is `lib-database`'s
//! `UnitsStore`.

pub mod ports;
pub mod router;
pub mod view;

use std::rc::Rc;

use lib_config::KeyBindingConfig;
use lib_database::{Commit, InMemoryCommit, UnitsStore};
use lib_ledger::Ledger;
use lib_shell::DialogCommand;
use lib_units::keymap::units_page_keymap;
use lib_units::presenter::UnitsPageEvent;
use lib_viper::keymap::Keymap;

pub use ports::{LedgerBackedPorts, TuiPorts};

/// What `Shell` builds each Units View from: the shared ports, and the page's and the Dialogs'
/// keymaps under `[keybindings]`.
#[derive(Clone)]
pub struct UnitsSetup {
    pub ports: TuiPorts,
    pub keymap: Keymap<UnitsPageEvent>,
    pub dialog_keymap: Keymap<DialogCommand>,
}

impl UnitsSetup {
    pub fn new(
        store: Box<dyn UnitsStore>,
        commit: Rc<dyn Commit>,
        config: &KeyBindingConfig,
    ) -> Self {
        Self {
            ports: LedgerBackedPorts::shared(store, commit, Ledger::stub()),
            keymap: units_page_keymap(config),
            dialog_keymap: lib_shell::dialog_keymap(config),
        }
    }

    /// The stub seed in memory, for a `Shell` built without storage.
    pub fn stub(config: &KeyBindingConfig) -> Self {
        Self::new(
            Box::new(lib_units::fake::InMemoryPorts::stub(chrono::Utc::now())),
            Rc::new(InMemoryCommit),
            config,
        )
    }
}
