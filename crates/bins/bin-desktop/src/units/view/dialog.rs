//! The Units Dialogs' Views, hosted by today's Dialog host. [`UnitsDialog`] is the value the host
//! holds while one is open (`OpenDialog::Units`): a `lib-units` Presenter and the widget state
//! that is gpui's to keep (which select's list is open and its highlight). Every key and click
//! becomes a Presenter event; each View draws the Presenter's view model and nothing else.
//!
//! The Presenter sits in an `Rc<RefCell<_>>`, not an Entity: the host's `Dialog` methods take no
//! gpui context to update one through, and the host repaints after every key and click.
//!
//! Confirm sends the Presenter its Confirm event ([`ConfirmDialog::confirm`]); a Dialog that
//! closed through its Router hands the outcome to the page. `Esc` and Cancel close through the
//! host, which tells the Presenter nothing, since a cancelled Units Dialog wrote nothing.

mod delete_price_source;
mod delete_unit;
mod price_source_form;
mod set_base_unit;
mod unit_form;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_toast::{Notice, Notify};
use lib_units::presenter::{
    DeletePriceSourcePresenter, DeleteUnitPresenter, PriceSourceFormPresenter,
    SetBaseUnitPresenter, UnitFormPresenter, UnitsOutcome,
};
use lib_units::{UnitsDialog as Which, UnitsRouter};

use self::price_source_form::ApiKeyInput;
use self::unit_form::UnitSelects;
use crate::form::viper::{
    choose_option, chosen_label, enter, labels, select_key, synced, toggle_select, typed,
};
use crate::units::ports::{self, DesktopPorts};
use crate::{
    chrome::dialog_host::{
        ConfirmDialog, Dialog, DialogActions, DialogKey, DialogOutcome, DialogReadback,
        DialogScope, Landed,
    },
    dialog,
    form::select::SelectState,
    theme::color,
};

type Shared<P> = Rc<RefCell<P>>;

/// The open Units Dialog: its Presenter and its widget state.
#[derive(Clone)]
pub struct UnitsDialog {
    ports: DesktopPorts,
    view: View,
}

#[derive(Clone)]
enum View {
    UnitForm(Shared<UnitFormPresenter>, UnitSelects),
    DeleteUnit(Shared<DeleteUnitPresenter>, SelectState),
    SetBaseUnit(Shared<SetBaseUnitPresenter>, SelectState),
    PriceSourceForm(Shared<PriceSourceFormPresenter>, ApiKeyInput),
    DeletePriceSource(Shared<DeletePriceSourcePresenter>),
}

impl View {
    /// The Presenter's address, which tells two open Dialogs apart.
    fn presenter(&self) -> *const () {
        match self {
            Self::UnitForm(presenter, _) => Rc::as_ptr(presenter).cast(),
            Self::DeleteUnit(presenter, _) => Rc::as_ptr(presenter).cast(),
            Self::SetBaseUnit(presenter, _) => Rc::as_ptr(presenter).cast(),
            Self::PriceSourceForm(presenter, _) => Rc::as_ptr(presenter).cast(),
            Self::DeletePriceSource(presenter) => Rc::as_ptr(presenter).cast(),
        }
    }

    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::UnitForm(_, selects) if selects.editing() => "EditUnit",
            Self::UnitForm(..) => "AddUnit",
            Self::DeleteUnit(..) => "DeleteUnit",
            Self::SetBaseUnit(..) => "SetBaseUnit",
            Self::PriceSourceForm(presenter, _) => {
                let view = presenter.borrow().view_model();
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

impl fmt::Debug for UnitsDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UnitsDialog")
            .field(&self.view.kind())
            .finish()
    }
}

/// Two values are the same Dialog when they hold the same Presenter.
impl PartialEq for UnitsDialog {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.view.presenter(), other.view.presenter())
    }
}

impl Eq for UnitsDialog {}

impl UnitsDialog {
    /// `which`'s Presenter on `ports`, or `None` when the row it names is gone.
    pub fn open(which: Which, ports: &DesktopPorts) -> Option<Self> {
        let shared = ports.clone();
        let view = match which {
            Which::AddUnit => unit_form::open(UnitFormPresenter::add(shared), false),
            Which::EditUnit(id) => unit_form::open(UnitFormPresenter::edit(shared, id)?, true),
            Which::DeleteUnit(id) => View::DeleteUnit(
                Rc::new(RefCell::new(DeleteUnitPresenter::new(shared, id)?)),
                SelectState::default(),
            ),
            Which::SetBaseUnit => View::SetBaseUnit(
                Rc::new(RefCell::new(SetBaseUnitPresenter::new(shared))),
                SelectState::default(),
            ),
            Which::AddPriceSource => price_source_form::open(PriceSourceFormPresenter::add(shared)),
            Which::EditPriceSource(id) => {
                price_source_form::open(PriceSourceFormPresenter::edit(shared, id)?)
            }
            Which::DeletePriceSource(id) => View::DeletePriceSource(Rc::new(RefCell::new(
                DeletePriceSourcePresenter::new(shared, id)?,
            ))),
        };
        Some(Self {
            ports: ports.clone(),
            view,
        })
    }

    /// A click's change, sent through the host's edit callback so the Dialog repaints.
    fn on_click(actions: &DialogActions, change: impl Fn(&mut Self) + 'static) -> dialog::OnClick {
        let actions = actions.clone();
        let change = Rc::new(change);
        Rc::new(move |_window: &mut Window, cx: &mut App| {
            let change = change.clone();
            actions.edit(cx, move |dialog: &mut Self, _| {
                change(dialog);
                None
            });
        })
    }

    /// As [`Self::on_click`], for a click on row `index` of an open list.
    fn on_option(
        actions: &DialogActions,
        change: impl Fn(&mut Self, usize) + 'static,
    ) -> crate::form::select_field::OnOptionClick {
        let actions = actions.clone();
        let change = Rc::new(change);
        Rc::new(move |index, _window: &mut Window, cx: &mut App| {
            let change = change.clone();
            actions.edit(cx, move |dialog: &mut Self, _| {
                change(dialog, index);
                None
            });
        })
    }
}

/// Where a Units Dialog's Presenter asks to go: closing, and Notices, which go onto the bin's
/// queue for `Shell` to drain once the key or click returns. The page opens Dialogs.
#[derive(Default)]
struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what its confirm did.
    closed: Option<Option<UnitsOutcome>>,
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
    presenter: &Shared<P>,
    event: E,
    handle: fn(&mut P, E, &mut dyn UnitsRouter),
) -> DialogRouter {
    let mut router = DialogRouter::default();
    handle(&mut presenter.borrow_mut(), event, &mut router);
    router
}

/// Why the last confirm was refused, in the danger colour under the fields.
fn refusal(text: Option<String>, cx: &App) -> Option<AnyElement> {
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
fn help(text: String, cx: &App) -> AnyElement {
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

impl Dialog for UnitsDialog {
    /// Every key is the Units Dialog's own: the host's typing and `Tab` would edit a `TextField`
    /// the Presenter doesn't hold.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        Some(match &mut self.view {
            View::UnitForm(presenter, selects) => unit_form::key(presenter, selects, key),
            View::DeleteUnit(presenter, select) => delete_unit::key(presenter, select, key),
            View::SetBaseUnit(presenter, select) => set_base_unit::key(presenter, select, key),
            View::PriceSourceForm(presenter, api_key) => {
                price_source_form::key(presenter, api_key, key)
            }
            View::DeletePriceSource(presenter) => delete_price_source::key(presenter, key),
        })
    }

    fn is_valid(&self) -> bool {
        match &self.view {
            View::UnitForm(presenter, _) => presenter.borrow().view_model().frame.can_confirm,
            View::DeleteUnit(presenter, _) => presenter.borrow().view_model().frame.can_confirm,
            View::SetBaseUnit(presenter, _) => presenter.borrow().view_model().frame.can_confirm,
            View::PriceSourceForm(presenter, _) => {
                presenter.borrow().view_model().frame.can_confirm
            }
            View::DeletePriceSource(presenter) => presenter.borrow().view_model().frame.can_confirm,
        }
    }

    fn close_open_select(&mut self) -> bool {
        let close = |state: &mut SelectState| {
            let open = state.is_open();
            state.cancel();
            open
        };
        match &mut self.view {
            View::UnitForm(_, selects) => selects.close(),
            View::DeleteUnit(_, select) | View::SetBaseUnit(_, select) => close(select),
            View::PriceSourceForm(..) | View::DeletePriceSource(_) => false,
        }
    }

    fn render(
        &self,
        _scope: &DialogScope,
        actions: &DialogActions,
        cx: &App,
    ) -> Option<AnyElement> {
        Some(match &self.view {
            View::UnitForm(presenter, selects) => {
                unit_form::render(&presenter.borrow().view_model(), selects, actions, cx)
            }
            View::DeleteUnit(presenter, select) => {
                delete_unit::render(&presenter.borrow().view_model(), select, actions, cx)
            }
            View::SetBaseUnit(presenter, select) => {
                set_base_unit::render(&presenter.borrow().view_model(), select, actions, cx)
            }
            View::PriceSourceForm(presenter, _) => {
                price_source_form::render(&presenter.borrow().view_model(), actions, cx)
            }
            View::DeletePriceSource(presenter) => {
                delete_price_source::render(&presenter.borrow().view_model(), actions, cx)
            }
        })
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _scope: &DialogScope, _cx: &App) -> Option<DialogReadback> {
        let (frame, mut values) = match &self.view {
            View::UnitForm(presenter, selects) => {
                let view = presenter.borrow().view_model();
                (view.frame.clone(), unit_form::readback(&view, selects))
            }
            View::DeleteUnit(presenter, select) => {
                let view = presenter.borrow().view_model();
                (view.frame.clone(), delete_unit::readback(&view, select))
            }
            View::SetBaseUnit(presenter, _) => {
                let view = presenter.borrow().view_model();
                (view.frame.clone(), set_base_unit::readback(&view))
            }
            View::PriceSourceForm(presenter, _) => {
                let view = presenter.borrow().view_model();
                (view.frame.clone(), price_source_form::readback(&view))
            }
            View::DeletePriceSource(presenter) => {
                let view = presenter.borrow().view_model();
                (view.frame.clone(), delete_price_source::readback(&view))
            }
        };
        values.insert(0, ("title", frame.title));
        values.push(("can_confirm", frame.can_confirm.to_string()));
        Some(DialogReadback {
            kind: self.view.kind(),
            values,
        })
    }
}

impl ConfirmDialog for UnitsDialog {
    type Outcome = UnitsOutcome;

    /// Sends Confirm to the Presenter, which runs its Interactor. A refusal stays on the
    /// Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<UnitsOutcome>> {
        ports::refresh(&self.ports, &scope.ledger, cx);
        let router = match &self.view {
            View::UnitForm(presenter, _) => send(
                presenter,
                lib_units::presenter::UnitFormEvent::Confirm,
                UnitFormPresenter::handle,
            ),
            View::DeleteUnit(presenter, _) => send(
                presenter,
                lib_units::presenter::DeleteUnitEvent::Confirm,
                DeleteUnitPresenter::handle,
            ),
            View::SetBaseUnit(presenter, _) => send(
                presenter,
                lib_units::presenter::SetBaseUnitEvent::Confirm,
                SetBaseUnitPresenter::handle,
            ),
            View::PriceSourceForm(presenter, _) => send(
                presenter,
                lib_units::presenter::PriceSourceFormEvent::Confirm,
                PriceSourceFormPresenter::handle,
            ),
            View::DeletePriceSource(presenter) => send(
                presenter,
                lib_units::presenter::DeletePriceSourceEvent::Confirm,
                DeletePriceSourcePresenter::handle,
            ),
        };
        ports::refresh(&self.ports, &scope.ledger, cx);
        let outcome = router.closed.flatten()?;
        Some(Landed {
            toast: None,
            outcome,
        })
    }
}
