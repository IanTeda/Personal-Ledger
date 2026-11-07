//! The Units module's Desktop half (`docs/development/viper-architecture.md`): the Views for the
//! Units Settings page and its Dialogs, and `impl UnitsRouter`. Every rule, every piece of text and
//! the keymap are `lib-units`'; this folder turns gpui keys and clicks into Presenter events and
//! draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds each Dialog's Presenter in an Entity in the Dialog host, one View per Dialog
//!   in `view/dialog/`.
//! - `router.rs` is the Router and the module's entry point ([`units_page`]).
//!
//! The ports are `lib-database`'s `StoredUnitsPorts`, which `LedgerPorts` hands the page and its
//! Dialogs with usage and holdings read from the other domains' stores (#1052, #1084).

mod router;
mod view;

use gpui::App;
use lib_units::SharedPorts;
pub use router::units_page;
pub(crate) use view::dialog::{Landed, Router, help, refusal};
pub use view::page::UnitsPage;

use crate::settings::snapshot::PageSnapshot;

/// The Units page as the UI tests read it: its view model.
#[doc(hidden)]
pub type SettingsUnitsSnapshot = lib_units::presenter::UnitsPageViewModel;

/// The Unit prices view as the UI tests read it: its view model, while it shows.
#[doc(hidden)]
pub type UnitPricesSnapshot = lib_units::presenter::UnitPricesViewModel;

impl PageSnapshot for SettingsUnitsSnapshot {
    type Page = UnitsPage;

    fn read(page: &UnitsPage, cx: &App) -> Self {
        page.view_model(cx)
    }
}

/// `None` while the Unit prices view is closed.
impl PageSnapshot for Option<UnitPricesSnapshot> {
    type Page = UnitsPage;

    fn read(page: &UnitsPage, _cx: &App) -> Self {
        page.prices_view_model()
    }
}

/// Prices every Unit in itself, so none is quoted in another and any can be deleted; for a UI test
/// that deletes a seeded Unit.
#[doc(hidden)]
pub fn unquote_for_test(ports: &SharedPorts) {
    let mut ports = ports.borrow_mut();
    let units = ports.units().units();
    for mut unit in units {
        if unit.link.quote != unit.id {
            unit.link.quote = unit.id;
            ports.split_mut().units.upsert_unit(unit);
        }
    }
}

/// What `shell::router::build` hands the Units module: its ports, and the page's and the Unit
/// prices view's keymaps under `[keybindings]`.
pub struct UnitsSetup {
    pub ports: SharedPorts,
    /// The Unit Deletion module's port over the Units and Institutions stores, which Delete unit
    /// writes through (#903).
    pub deletion: lib_unit_deletion::SharedUnitDeletion,
    pub keymap: lib_viper::keymap::Keymap<lib_units::presenter::UnitsPageEvent>,
    pub prices_keymap: lib_viper::keymap::Keymap<lib_units::presenter::UnitPricesEvent>,
}

impl UnitsSetup {
    pub fn new(
        ports: SharedPorts,
        deletion: lib_unit_deletion::SharedUnitDeletion,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            ports,
            deletion,
            keymap: lib_units::keymap::units_page_keymap(config),
            prices_keymap: lib_units::keymap::unit_prices_keymap(config),
        }
    }

    /// `storage`'s Units ports under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        Self::new(
            storage.ports.units.clone(),
            storage.ports.unit_deletion.clone(),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
