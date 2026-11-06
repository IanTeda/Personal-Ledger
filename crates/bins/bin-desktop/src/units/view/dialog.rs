//! The Units Dialogs' Views, each open one an Entity in the Dialog host (`shell::view::dialog_entity`).
//! [`UnitsDialog`] holds a `lib-units` Presenter and the one piece of widget state that is gpui's
//! to keep, an open list's scroll window. Every Dialog command and typed key reaches the Presenter
//! as a `lib-viper` `DialogInput`, and every click as a Presenter event; each View draws the
//! Presenter's view model, open lists included, and nothing else.
//!
//! Delete unit is the Unit Deletion module's Dialog (`crate::unit_deletion`), which the page's
//! Router opens instead, since it writes the Institutions too.
//!
//! Confirm and Cancel go to the Presenter, which closes its Dialog through the host's
//! [`HostRouter`]; a confirmed outcome is handed to the page, and the host closes on
//! [`DialogClosed`].

mod delete_price_source;
mod price_source_form;
mod set_base_unit;
mod set_price;
mod unit_form;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, Render, Window, div, prelude::*, px};
use lib_units::UnitsDialog as Which;
use lib_units::presenter::{
    DeletePriceSourceEvent, DeletePriceSourcePresenter, PriceSourceFormEvent,
    PriceSourceFormPresenter, SetBaseUnitEvent, SetBaseUnitPresenter, SetPriceEvent,
    SetPricePresenter, UnitFormEvent, UnitFormPresenter, UnitsOutcome,
};
use lib_viper::dialog::{DialogInput, DialogRouter};

use self::unit_form::UnitSelects;
use crate::form::viper::{chosen_label, drawn, labels};
use crate::ledger_store::LedgerStore;
use crate::units::ports::{self, DesktopPorts};
use crate::{
    dialog,
    shell::view::dialog_entity::DialogReadback,
    shell::view::dialog_entity::{DialogClicks, DialogClosed, HostRouter, HostedDialog},
    theme::color,
};

/// The open Units Dialog's clicks.
type Clicks = DialogClicks<UnitsDialog>;

/// The Router a Units Dialog's Presenter closes through, which the Delete unit Dialog
/// (`crate::unit_deletion`) shares.
pub(crate) type Router = HostRouter<UnitsOutcome>;

/// What a confirmed Units Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(UnitsOutcome, &mut App)>;

/// The open Units Dialog: its Presenter and its widget state.
pub struct UnitsDialog {
    ports: DesktopPorts,
    /// The Ledger a confirm announces what it wrote through.
    ledger: Entity<LedgerStore>,
    view: View,
    landed: Landed,
}

enum View {
    /// Boxed: the form is the largest Presenter by far.
    UnitForm(Box<UnitFormPresenter>, UnitSelects),
    /// With the open list's scroll window.
    SetBaseUnit(SetBaseUnitPresenter, usize),
    SetPrice(SetPricePresenter),
    PriceSourceForm(PriceSourceFormPresenter),
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
            Self::PriceSourceForm(presenter) => {
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
            Which::SetBaseUnit => View::SetBaseUnit(SetBaseUnitPresenter::new(shared), 0),
            Which::SetPrice(id) => View::SetPrice(SetPricePresenter::new(
                shared,
                std::rc::Rc::new(chrono::Utc::now),
                id,
            )?),
            Which::AddPriceSource => View::PriceSourceForm(PriceSourceFormPresenter::add(shared)),
            Which::EditPriceSource(id) => {
                View::PriceSourceForm(PriceSourceFormPresenter::edit(shared, id)?)
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
    fn close_event(&mut self, confirm: bool) -> Router {
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
            View::PriceSourceForm(presenter) => {
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

    /// Sends the open Presenter `input`, returning what its Router was asked.
    fn send_input(&mut self, input: DialogInput) -> Router {
        match &mut self.view {
            View::UnitForm(presenter, _) => send(
                presenter.as_mut(),
                UnitFormEvent::Input(input),
                UnitFormPresenter::handle,
            ),
            View::SetBaseUnit(presenter, _) => send(
                presenter,
                SetBaseUnitEvent::Input(input),
                SetBaseUnitPresenter::handle,
            ),
            View::SetPrice(presenter) => send(
                presenter,
                SetPriceEvent::Input(input),
                SetPricePresenter::handle,
            ),
            View::PriceSourceForm(presenter) => send(
                presenter,
                PriceSourceFormEvent::Input(input),
                PriceSourceFormPresenter::handle,
            ),
            View::DeletePriceSource(presenter) => send(
                presenter,
                DeletePriceSourceEvent::Input(input),
                DeletePriceSourcePresenter::handle,
            ),
        }
    }

    /// Hands a confirmed outcome to the page and tells the host, once the Presenter has closed
    /// the Dialog.
    fn closed(&self, router: Router, cx: &mut Context<'_, Self>) {
        let Some(outcome) = router.closed() else {
            return;
        };
        if let Some(outcome) = outcome {
            (self.landed)(outcome, cx);
        }
        cx.emit(DialogClosed);
    }
}

/// Sends `event` to `presenter`, returning what its Router was asked.
fn send<P, E>(
    presenter: &mut P,
    event: E,
    handle: fn(&mut P, E, &mut dyn DialogRouter<UnitsOutcome>),
) -> Router {
    let mut router = Router::default();
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
    /// Every input is the Presenter's to read: it holds every field and every open list. One
    /// that may confirm (`select`, `submit`) announces the Units' writes either side, as
    /// [`Self::confirm`] does.
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) {
        let may_confirm = matches!(input, DialogInput::Select | DialogInput::Submit);
        if may_confirm {
            ports::announce(&self.ports, &self.ledger, cx);
        }
        let router = self.send_input(input);
        if may_confirm {
            ports::announce(&self.ports, &self.ledger, cx);
        }
        self.closed(router, cx);
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::UnitForm(presenter, _) => presenter.view_model().frame.can_confirm,
            View::SetBaseUnit(presenter, _) => presenter.view_model().frame.can_confirm,
            View::SetPrice(presenter) => presenter.view_model().frame.can_confirm,
            View::PriceSourceForm(presenter) => presenter.view_model().frame.can_confirm,
            View::DeletePriceSource(presenter) => presenter.view_model().frame.can_confirm,
        }
    }

    /// Sends Confirm to the Presenter, which runs its Interactor, announcing the Units' writes
    /// either side. A refusal stays on the Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        ports::announce(&self.ports, &self.ledger, cx);
        let router = self.close_event(true);
        ports::announce(&self.ports, &self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Units Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.close_event(false);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        match &mut self.view {
            View::UnitForm(presenter, _) => {
                let open = presenter.view_model().list.is_open();
                send(
                    presenter.as_mut(),
                    UnitFormEvent::CloseList,
                    UnitFormPresenter::handle,
                );
                open
            }
            View::SetBaseUnit(presenter, _) => {
                let open = presenter.view_model().list.is_open();
                send(
                    presenter,
                    SetBaseUnitEvent::CloseList,
                    SetBaseUnitPresenter::handle,
                );
                open
            }
            View::SetPrice(_) | View::PriceSourceForm(_) | View::DeletePriceSource(_) => false,
        }
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let (frame, mut values) = match &self.view {
            View::UnitForm(presenter, _) => {
                let view = presenter.view_model();
                (view.frame.clone(), unit_form::readback(&view))
            }
            View::SetBaseUnit(presenter, _) => {
                let view = presenter.view_model();
                (view.frame.clone(), set_base_unit::readback(&view))
            }
            View::SetPrice(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), set_price::readback(&view))
            }
            View::PriceSourceForm(presenter) => {
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
        match &mut self.view {
            View::UnitForm(presenter, selects) => {
                unit_form::render(&presenter.view_model(), selects, &actions, cx)
            }
            View::SetBaseUnit(presenter, scroll) => {
                set_base_unit::render(&presenter.view_model(), scroll, &actions, cx)
            }
            View::SetPrice(presenter) => set_price::render(&presenter.view_model(), &actions, cx),
            View::PriceSourceForm(presenter) => {
                price_source_form::render(&presenter.view_model(), &actions, cx)
            }
            View::DeletePriceSource(presenter) => {
                delete_price_source::render(&presenter.view_model(), &actions, cx)
            }
        }
    }
}
