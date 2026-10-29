//! The Units Dialogs' Views, each open one an Entity in the Dialog host (`shell::view::dialog_entity`).
//! [`UnitsDialog`] holds a `lib-units` Presenter and the widget state that is gpui's to keep (which
//! select's list is open and its highlight). Every key and click becomes a Presenter event; each
//! View draws the Presenter's view model and nothing else.
//!
//! Delete unit is the Unit Deletion module's Dialog (`crate::unit_deletion`), which the page's
//! Router opens instead, since it writes the Institutions too.
//!
//! Confirm and Cancel go to the Presenter, which closes its Dialog through its Router; a confirmed
//! outcome is handed to the page, and the host closes on [`DialogClosed`].

mod delete_price_source;
mod price_source_form;
mod set_base_unit;
mod set_price;
mod unit_form;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, Render, Window, div, prelude::*, px};
use lib_toast::{Notice, Notify};
use lib_units::presenter::{
    DeletePriceSourcePresenter, PriceSourceFormPresenter, SetBaseUnitPresenter, SetPricePresenter,
    UnitFormPresenter, UnitsOutcome,
};
use lib_units::{UnitsDialog as Which, UnitsRouter};

use self::price_source_form::ApiKeyInput;
use self::unit_form::UnitSelects;
use crate::form::viper::{
    choose_option, chosen_label, enter, labels, select_key, synced, toggle_select, typed,
};
use crate::ledger_store::LedgerStore;
use crate::units::ports::{self, DesktopPorts};
use crate::{
    dialog,
    form::select::SelectState,
    shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog},
    shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback},
    theme::color,
};

/// The open Units Dialog's clicks.
type Clicks = DialogClicks<UnitsDialog>;

/// What a confirmed Units Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(UnitsOutcome, &mut App)>;

/// The open Units Dialog: its Presenter and its widget state.
pub struct UnitsDialog {
    ports: DesktopPorts,
    /// The Ledger read copy a confirm brings in line with what it wrote.
    ledger: Entity<LedgerStore>,
    view: View,
    landed: Landed,
}

enum View {
    /// Boxed: the form is the largest Presenter by far.
    UnitForm(Box<UnitFormPresenter>, UnitSelects),
    SetBaseUnit(SetBaseUnitPresenter, SelectState),
    SetPrice(SetPricePresenter),
    PriceSourceForm(PriceSourceFormPresenter, ApiKeyInput),
    DeletePriceSource(DeletePriceSourcePresenter),
}

impl View {
    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::UnitForm(_, selects) if selects.editing() => "EditUnit",
            Self::UnitForm(..) => "AddUnit",
            Self::SetBaseUnit(..) => "SetBaseUnit",
            Self::SetPrice(_) => "SetPrice",
            Self::PriceSourceForm(presenter, _) => {
                let view = presenter.view_model();
                match (&view.endpoint, &view.usage) {
                    (None, _) => "EditBuiltInPriceSource",
                    (Some(_), None) => "AddPriceSource",
                    (Some(_), Some(_)) => "EditPriceSource",
                }
            }
            Self::DeletePriceSource(_) => "DeletePriceSource",
        }
    }
}

impl UnitsDialog {
    /// `which`'s Presenter on `ports`, or `None` when the row it names is gone. A confirm brings
    /// `ledger` in line and hands its outcome to `landed`.
    pub(crate) fn open(
        which: Which,
        ports: &DesktopPorts,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Option<Self> {
        let shared = ports.clone();
        let view = match which {
            Which::AddUnit => unit_form::open(UnitFormPresenter::add(shared), false),
            Which::EditUnit(id) => unit_form::open(UnitFormPresenter::edit(shared, id)?, true),
            // The page's Router opens the Unit Deletion module's Dialog instead.
            Which::DeleteUnit(_) => return None,
            Which::SetBaseUnit => {
                View::SetBaseUnit(SetBaseUnitPresenter::new(shared), SelectState::default())
            }
            Which::SetPrice(id) => View::SetPrice(SetPricePresenter::new(
                shared,
                std::rc::Rc::new(chrono::Utc::now),
                id,
            )?),
            Which::AddPriceSource => price_source_form::open(PriceSourceFormPresenter::add(shared)),
            Which::EditPriceSource(id) => {
                price_source_form::open(PriceSourceFormPresenter::edit(shared, id)?)
            }
            Which::DeletePriceSource(id) => {
                View::DeletePriceSource(DeletePriceSourcePresenter::new(shared, id)?)
            }
        };
        Some(Self {
            ports: ports.clone(),
            ledger: ledger.clone(),
            view,
            landed,
        })
    }

    /// A click's change to the Dialog, as one input.
    fn on_click(actions: &Clicks, change: impl Fn(&mut Self) + 'static) -> dialog::OnClick {
        actions.click(change)
    }

    /// As [`Self::on_click`], for a click on row `index` of an open list.
    fn on_option(
        actions: &Clicks,
        change: impl Fn(&mut Self, usize) + 'static,
    ) -> crate::form::select_field::OnOptionClick {
        actions.option(change)
    }

    /// Sends the open Presenter its Confirm or its Cancel, returning what its Router was asked.
    fn close_event(&mut self, confirm: bool) -> DialogRouter {
        use lib_units::presenter::{
            DeletePriceSourceEvent, PriceSourceFormEvent, SetBaseUnitEvent, SetPriceEvent,
            UnitFormEvent,
        };
        match &mut self.view {
            View::UnitForm(presenter, _) => {
                let event = if confirm {
                    UnitFormEvent::Confirm
                } else {
                    UnitFormEvent::Cancel
                };
                send(presenter.as_mut(), event, UnitFormPresenter::handle)
            }
            View::SetBaseUnit(presenter, _) => {
                let event = if confirm {
                    SetBaseUnitEvent::Confirm
                } else {
                    SetBaseUnitEvent::Cancel
                };
                send(presenter, event, SetBaseUnitPresenter::handle)
            }
            View::SetPrice(presenter) => {
                let event = if confirm {
                    SetPriceEvent::Confirm
                } else {
                    SetPriceEvent::Cancel
                };
                send(presenter, event, SetPricePresenter::handle)
            }
            View::PriceSourceForm(presenter, _) => {
                let event = if confirm {
                    PriceSourceFormEvent::Confirm
                } else {
                    PriceSourceFormEvent::Cancel
                };
                send(presenter, event, PriceSourceFormPresenter::handle)
            }
            View::DeletePriceSource(presenter) => {
                let event = if confirm {
                    DeletePriceSourceEvent::Confirm
                } else {
                    DeletePriceSourceEvent::Cancel
                };
                send(presenter, event, DeletePriceSourcePresenter::handle)
            }
        }
    }

    /// Hands a confirmed outcome to the page and tells the host, once the Presenter has closed
    /// the Dialog.
    fn closed(&self, router: DialogRouter, cx: &mut Context<'_, Self>) {
        let Some(outcome) = router.closed else {
            return;
        };
        if let Some(outcome) = outcome {
            (self.landed)(outcome, cx);
        }
        cx.emit(DialogClosed);
    }
}

/// Where a Units Dialog's Presenter asks to go: closing, and Notices, which go onto the bin's
/// queue for `Root` to drain once the key or click returns. The page opens Dialogs. The Delete unit
/// Dialog (`crate::unit_deletion`) shares it.
#[derive(Default)]
pub(crate) struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what its confirm did.
    pub(crate) closed: Option<Option<UnitsOutcome>>,
}

impl UnitsRouter for DialogRouter {
    fn open_dialog(&mut self, _dialog: Which) -> bool {
        false
    }

    fn close_dialog(&mut self, outcome: Option<UnitsOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

/// Sends `event` to `presenter`, returning what its Router was asked.
fn send<P, E>(
    presenter: &mut P,
    event: E,
    handle: fn(&mut P, E, &mut dyn UnitsRouter),
) -> DialogRouter {
    let mut router = DialogRouter::default();
    handle(presenter, event, &mut router);
    router
}

/// Why the last confirm was refused, in the danger colour under the fields.
pub(crate) fn refusal(text: Option<String>, cx: &App) -> Option<AnyElement> {
    text.map(|text| {
        div()
            .debug_selector(|| "units-dialog-refusal".to_string())
            .text_size(px(12.0))
            .text_color(color::accent_text(cx))
            .child(text)
            .into_any_element()
    })
}

/// A field's 11.5px muted help line under it.
pub(crate) fn help(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}

/// A test button that has nothing behind it yet: drawn disabled beside its note.
fn test_not_built(id: &'static str, label: String, note: String, cx: &App) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(
            div()
                .debug_selector(move || id.to_string())
                .py(px(7.0))
                .px(px(12.0))
                .border_1()
                .border_color(color::border(cx))
                .opacity(0.45)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(label),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child(note),
        )
        .into_any_element()
}

impl EventEmitter<DialogClosed> for UnitsDialog {}

impl HostedDialog for UnitsDialog {
    /// Every key is the Units Dialog's own: the Presenter holds every field.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        match &mut self.view {
            View::UnitForm(presenter, selects) => unit_form::key(presenter, selects, key),
            View::SetBaseUnit(presenter, select) => set_base_unit::key(presenter, select, key),
            View::SetPrice(presenter) => set_price::key(presenter, key),
            View::PriceSourceForm(presenter, api_key) => {
                price_source_form::key(presenter, api_key, key)
            }
            View::DeletePriceSource(presenter) => delete_price_source::key(presenter, key),
        }
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::UnitForm(presenter, _) => presenter.view_model().frame.can_confirm,
            View::SetBaseUnit(presenter, _) => presenter.view_model().frame.can_confirm,
            View::SetPrice(presenter) => presenter.view_model().frame.can_confirm,
            View::PriceSourceForm(presenter, _) => presenter.view_model().frame.can_confirm,
            View::DeletePriceSource(presenter) => presenter.view_model().frame.can_confirm,
        }
    }

    /// Sends Confirm to the Presenter, which runs its Interactor, with the Ledger read copy in
    /// line either side. A refusal stays on the Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ports, &self.ledger, cx);
        let router = self.close_event(true);
        ports::refresh(&self.ports, &self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Units Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.close_event(false);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        let close = |state: &mut SelectState| {
            let open = state.is_open();
            state.cancel();
            open
        };
        match &mut self.view {
            View::UnitForm(_, selects) => selects.close(),
            View::SetBaseUnit(_, select) => close(select),
            View::SetPrice(_) | View::PriceSourceForm(..) | View::DeletePriceSource(_) => false,
        }
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let (frame, mut values) = match &self.view {
            View::UnitForm(presenter, selects) => {
                let view = presenter.view_model();
                (view.frame.clone(), unit_form::readback(&view, selects))
            }
            View::SetBaseUnit(presenter, _) => {
                let view = presenter.view_model();
                (view.frame.clone(), set_base_unit::readback(&view))
            }
            View::SetPrice(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), set_price::readback(&view))
            }
            View::PriceSourceForm(presenter, _) => {
                let view = presenter.view_model();
                (view.frame.clone(), price_source_form::readback(&view))
            }
            View::DeletePriceSource(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), delete_price_source::readback(&view))
            }
        };
        values.insert(0, ("title", frame.title));
        values.push(("can_confirm", frame.can_confirm.to_string()));
        DialogReadback {
            kind: self.view.kind(),
            values,
        }
    }
}

impl Render for UnitsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        match &self.view {
            View::UnitForm(presenter, selects) => {
                unit_form::render(&presenter.view_model(), selects, &actions, cx)
            }
            View::SetBaseUnit(presenter, select) => {
                set_base_unit::render(&presenter.view_model(), select, &actions, cx)
            }
            View::SetPrice(presenter) => set_price::render(&presenter.view_model(), &actions, cx),
            View::PriceSourceForm(presenter, _) => {
                price_source_form::render(&presenter.view_model(), &actions, cx)
            }
            View::DeletePriceSource(presenter) => {
                delete_price_source::render(&presenter.view_model(), &actions, cx)
            }
        }
    }
}
