//! The **General** section (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state", first in
//! scroll order): ledger identity fields plus a "THIS LEDGER" summary panel. The identity fields
//! are still the mockup's own dummy values, with no `lib_database::Preferences` wiring; *Base unit*
//! is real, a select over the Ledger's Units that runs `lib-units`' `SetBaseUnit` Interactor on the
//! Units ports.

mod render;

use gpui::{Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_core::UnitId;
use lib_ledger::Domain;
use lib_units::UnitsPorts as _;
use lib_units::interactor::SetBaseUnit;

use crate::{
    ledger_store::{LedgerStore, notify_on},
    settings::{SettingsPage, hints::settings_general_hints, pages::Page},
    shell::view::statusline::{Hint, owned_hints},
    units::DesktopPorts,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for GeneralPage {}

/// The General page. It reads the Ledger for the *Base unit* select and holds only whether the
/// keyboard is on the page, which draws the select focused.
pub struct GeneralPage {
    ledger: Entity<LedgerStore>,
    ports: DesktopPorts,
    page_focused: bool,
    _units_changed: Subscription,
}

impl GeneralPage {
    pub fn new(
        ledger: Entity<LedgerStore>,
        ports: DesktopPorts,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let units_changed = notify_on(&ledger, Domain::Units, cx);
        Self {
            ledger,
            ports,
            page_focused: false,
            _units_changed: units_changed,
        }
    }

    /// The select's options in the Ledger's by-code order: `(id, code, name)`.
    fn unit_options(&self, cx: &gpui::App) -> Vec<(UnitId, String, String)> {
        self.ledger
            .read(cx)
            .ledger()
            .units_by_code()
            .into_iter()
            .filter(|unit| unit.is_active)
            .map(|unit| (unit.id, unit.code.clone(), unit.name.clone()))
            .collect()
    }

    fn base_unit(&self, cx: &gpui::App) -> UnitId {
        self.ledger.read(cx).ledger().base_unit()
    }

    /// Makes `id` the base Unit, one input committed once it ran (`crate::input::send`). A
    /// refusal means the Unit was removed under the select, which repaints without it, so there
    /// is nothing to report.
    fn set_base_unit(&mut self, id: UnitId, cx: &mut Context<'_, Self>) {
        crate::input::send(cx, |cx| {
            let _ = {
                let mut ports = self.ports.borrow_mut();
                let ports = ports.split_mut();
                SetBaseUnit(id).run(ports.units, ports.base_unit)
            };
            crate::units::ports::refresh(&self.ports, &self.ledger, cx);
        });
        cx.notify();
    }

    /// Clicking an option of the select.
    fn click_base_unit(&mut self, id: UnitId, cx: &mut Context<'_, Self>) {
        self.set_base_unit(id, cx);
    }

    /// `j`/`k` move the select to the next or previous Unit, stopping at the ends.
    fn step_base_unit(&mut self, delta: isize, cx: &mut Context<'_, Self>) {
        let ids: Vec<_> = self
            .unit_options(cx)
            .into_iter()
            .map(|(id, ..)| id)
            .collect();
        let next = crate::settings::step_choice(&ids, self.base_unit(cx), delta);
        self.set_base_unit(next, cx);
    }

    /// The select's state for a test: the base Unit's code and the options' codes in order.
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &gpui::App) -> SettingsGeneralSnapshot {
        SettingsGeneralSnapshot {
            base_unit: self
                .ledger
                .read(cx)
                .ledger()
                .base_unit_row()
                .map(|unit| unit.code.clone()),
            options: self
                .unit_options(cx)
                .into_iter()
                .map(|(_, code, _)| code)
                .collect(),
        }
    }
}

/// The General page for a test: the base Unit's code and the select's option codes in order.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsGeneralSnapshot {
    pub base_unit: Option<String>,
    pub options: Vec<String>,
}

impl Page for GeneralPage {
    const PAGE: SettingsPage = SettingsPage::General;

    fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if !self.page_focused || keystroke.modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "j" | "down" => self.step_base_unit(1, cx),
            "k" | "up" => self.step_base_unit(-1, cx),
            _ => return false,
        }
        true
    }

    fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    fn hints(&self, _cx: &gpui::App) -> Vec<Hint> {
        owned_hints(settings_general_hints())
    }
}
