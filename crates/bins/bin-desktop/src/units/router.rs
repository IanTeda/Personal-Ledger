//! `impl UnitsRouter` for the Desktop: where the Units page's Presenter, its prices view included,
//! asks to go. A Dialog opens as an Entity in the Dialog host through the page's gpui context,
//! which is why the Router is built for each `handle` call (`PageView::with_router`), and a Notice
//! goes onto the bin's queue (`crate::notices`), which the input's `send` drains.

use gpui::{AppContext as _, Context, Entity};
use lib_toast::{Notice, Notify};
use lib_units::presenter::{
    DeletePriceSourcePresenter, PriceSourceFormPresenter, SetBaseUnitPresenter, SetPricePresenter,
    UnitFormPresenter, UnitsPagePresenter,
};
use lib_units::{UnitId, UnitsDialog as Which, UnitsRouter};

use super::{UnitsPage, UnitsRoutes, UnitsSetup};
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::shell::view::module_page::ModulePage;

pub(super) struct PageRouter<'a, 'b> {
    pub routes: &'a UnitsRoutes,
    pub cx: &'a mut Context<'b, UnitsPage>,
}

impl UnitsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports, handed the Dialog keys, and hosts it as a
    /// `ModuleDialog`; its confirmed outcome comes back to the page through
    /// [`ModulePage::landing`], whether the tables or the prices view opened it. Delete unit is the
    /// Unit Deletion module's Dialog, which writes the Institutions that name the Unit too.
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let routes = self.routes;
        let landed = UnitsPage::landing(self.cx);
        let (ports, keys) = (routes.ports.clone(), routes.dialog_keys.get());
        match dialog {
            Which::AddUnit => {
                ModuleDialog::open(self.cx, UnitFormPresenter::add(ports, keys), landed);
            }
            Which::EditUnit(id) => {
                let Some(form) = UnitFormPresenter::edit(ports, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::DeleteUnit(id) => {
                return crate::unit_deletion::router::open_delete(
                    self.cx,
                    &ports,
                    &routes.deletion,
                    id,
                    keys,
                    landed,
                );
            }
            Which::SetBaseUnit => {
                ModuleDialog::open(self.cx, SetBaseUnitPresenter::new(ports, keys), landed);
            }
            Which::SetPrice(id) => {
                let now = super::view::page::now_of(&routes.clock);
                let Some(set) = SetPricePresenter::new(ports, now, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, set, landed);
            }
            Which::AddPriceSource => {
                ModuleDialog::open(self.cx, PriceSourceFormPresenter::add(ports, keys), landed);
            }
            Which::EditPriceSource(id) => {
                let Some(form) = PriceSourceFormPresenter::edit(ports, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::DeletePriceSource(id) => {
                let Some(delete) = DeletePriceSourcePresenter::new(ports, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, delete, landed);
            }
        }
        true
    }

    /// The page Presenter holds the prices view and draws it in its view model, so there is
    /// nothing to build.
    fn open_prices(&mut self, _unit: UnitId) {}

    fn close_prices(&mut self) {}
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Units module's entry point: its Settings page, built on `setup`'s ports and dated by
/// `clock`, which the Settings View hosts as any other page.
pub fn units_page(
    setup: UnitsSetup,
    announcer: Entity<Announcer>,
    clock: lib_core::Clock,
    cx: &mut gpui::App,
) -> Entity<UnitsPage> {
    let UnitsSetup {
        ports,
        deletion,
        dialog_keys,
        keymap,
        prices_keymap,
    } = setup;
    let presenter =
        UnitsPagePresenter::new(ports.clone(), super::view::page::now_of(&clock), keymap)
            .with_prices_keymap(prices_keymap);
    let routes = UnitsRoutes {
        ports,
        deletion,
        dialog_keys,
        clock,
    };
    cx.new(|cx| ModulePage::new(presenter, routes, &announcer, cx))
}
