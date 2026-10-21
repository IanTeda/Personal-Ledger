//! The Units module's Desktop half (`docs/development/viper-architecture.md`): the Views for the
//! Units Settings page and its Dialogs, and `impl UnitsRouter`. Every rule, every piece of text and
//! the keymap are `lib-units`'; this folder turns gpui keys and clicks into Presenter events and
//! draws the view models.
//!
//! - `page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `dialogs.rs` hosts each Dialog's Presenter in today's Dialog host.
//! - `router.rs` is the Router and the module's entry point ([`units_page`]).
//! - `ports.rs` holds the halfway shims: usage and holdings answered from the Ledger, and the
//!   Ledger read copy refreshed after each write (#746).

mod dialogs;
mod page;
pub mod ports;
mod router;

pub use dialogs::UnitsDialog;
pub use page::UnitsPage;
pub use ports::{DesktopPorts, LedgerBackedPorts};
pub use router::units_page;

/// The Units page as the UI tests read it: its view model.
#[doc(hidden)]
pub type SettingsUnitsSnapshot = lib_units::presenter::UnitsPageViewModel;

/// What `build_shell` hands the Units module: its ports and its keymap under `[keybindings]`.
pub struct UnitsSetup {
    pub ports: DesktopPorts,
    pub keymap: lib_units::keymap::Keymap<lib_units::presenter::UnitsPageEvent>,
}

impl UnitsSetup {
    pub fn new(
        ports: Box<dyn lib_units::UnitsPorts>,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            ports: LedgerBackedPorts::shared(ports),
            keymap: lib_units::keymap::units_page_keymap(config),
        }
    }

    /// The stub seed in memory under the default keys, for a test that builds `Shell` directly.
    #[cfg(test)]
    pub fn stub() -> Self {
        Self::new(
            Box::new(lib_units::fake::InMemoryPorts::stub(chrono::Utc::now())),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
