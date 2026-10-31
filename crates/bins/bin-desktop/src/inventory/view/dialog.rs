//! The Inventory Dialogs' Views, each open one an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`InventoryDialog`] holds a `lib-inventory` Presenter and the
//! widget state that is gpui's to keep (whether the Unit and destination lists are open and their
//! highlights). Every key and click becomes a Presenter event; each View draws the Presenter's
//! view model and nothing else.
//!
//! Remove location confirms through the reassign port, answered from the Ledger ([`LedgerPorts`])
//! and built around the confirm's gpui context. A confirmed outcome is handed to the page, and the
//! host closes on [`DialogClosed`]. A Toast the Presenter raises goes onto the bin's queue, which
//! the input's `send` drains.

mod area_blocked;
mod area_form;
mod location_form;
mod remove_area;
mod remove_location;

use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window, div, prelude::*, px};
use lib_inventory::presenter::{
    AreaBlockedEvent, AreaBlockedPresenter, AreaFormEvent, AreaFormPresenter, LocationFormEvent,
    LocationFormPresenter, RemoveAreaEvent, RemoveAreaPresenter, RemoveLocationEvent,
    RemoveLocationPresenter,
};
use lib_inventory::{InventoryDialog as Which, InventoryOutcome, SharedInventory};
use lib_shell::DialogCommand;
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::form::select::SelectState;
use crate::inventory::ports::{self, LedgerPorts};
use crate::inventory::router::DialogRouter;
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use crate::theme::color;
use lib_shell::Hint;

/// The open Inventory Dialog's clicks.
type Clicks = DialogClicks<InventoryDialog>;

/// What a confirmed Inventory Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(InventoryOutcome, &mut App)>;

/// The open Inventory Dialog: its Presenter and its widget state.
pub struct InventoryDialog {
    /// The Ledger the confirm's reassign port writes, and whose read copy it brings in line.
    ledger: Entity<LedgerStore>,
    view: View,
    landed: Landed,
}

enum View {
    /// Add or Edit location, with the Unit select's open state.
    Location(LocationFormPresenter, SelectState),
    RemoveLocation(RemoveLocationPresenter),
    /// Add or Edit area.
    Area(AreaFormPresenter),
    /// Remove area, with the destination select's open state.
    RemoveArea(RemoveAreaPresenter, SelectState),
    Blocked(AreaBlockedPresenter),
}

impl View {
    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::Location(presenter, _) if presenter.view_model().editing => "EditLocation",
            Self::Location(..) => "AddLocation",
            Self::RemoveLocation(_) => "RemoveLocation",
            Self::Area(presenter) if presenter.view_model().editing => "EditArea",
            Self::Area(_) => "AddArea",
            Self::RemoveArea(..) => "RemoveArea",
            Self::Blocked(_) => "AreaBlocked",
        }
    }
}

impl InventoryDialog {
    /// `which`'s Presenter on `inventory`, its Units and Documents read from `ledger`, or `None`
    /// when the row it names is gone. A confirm hands its outcome to `landed`.
    pub(crate) fn open(
        which: Which,
        inventory: &SharedInventory,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
        cx: &mut App,
    ) -> Option<Self> {
        let reads = LedgerPorts { ledger, cx };
        let view = match which {
            Which::AddLocation { unit } => View::Location(
                LocationFormPresenter::add(inventory, &reads, unit),
                SelectState::default(),
            ),
            Which::EditLocation(id) => View::Location(
                LocationFormPresenter::edit(inventory, &reads, id)?,
                SelectState::default(),
            ),
            Which::RemoveLocation(id) => {
                View::RemoveLocation(RemoveLocationPresenter::new(inventory, &reads, id)?)
            }
            Which::AddArea(location) => View::Area(AreaFormPresenter::add(inventory, location)?),
            Which::EditArea(id) => View::Area(AreaFormPresenter::edit(inventory, id)?),
            Which::RemoveArea(id) => View::RemoveArea(
                RemoveAreaPresenter::new(inventory, id)?,
                SelectState::default(),
            ),
            Which::AreaBlocked(id) => View::Blocked(AreaBlockedPresenter::new(inventory, id)?),
        };
        Some(Self {
            ledger: ledger.clone(),
            view,
            landed,
        })
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

impl EventEmitter<DialogClosed> for InventoryDialog {}

impl HostedDialog for InventoryDialog {
    /// Every key is the Inventory Dialog's own: the Presenter holds every field.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        match &mut self.view {
            View::Location(presenter, select) => location_form::key(presenter, select, key),
            View::RemoveLocation(presenter) => remove_location::key(presenter, key),
            View::Area(presenter) => area_form::key(presenter, key),
            View::RemoveArea(presenter, select) => remove_area::key(presenter, select, key),
            View::Blocked(_) => area_blocked::key(key),
        }
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::Location(presenter, _) => presenter.view_model().frame.can_confirm,
            View::RemoveLocation(presenter) => presenter.view_model().frame.can_confirm,
            View::Area(presenter) => presenter.view_model().frame.can_confirm,
            View::RemoveArea(presenter, _) => presenter.view_model().frame.can_confirm,
            View::Blocked(presenter) => presenter.view_model().frame.can_confirm,
        }
    }

    /// Sends Confirm to the Presenter, lent the reassign port, with the Ledger read copy in line
    /// either side. A refusal stays on the Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ledger, cx);
        let mut router = DialogRouter::default();
        let mut reassign = LedgerPorts {
            ledger: &self.ledger,
            cx,
        };
        match &mut self.view {
            View::Location(presenter, _) => presenter.confirm(&mut router),
            View::RemoveLocation(presenter) => presenter.confirm(&mut reassign, &mut router),
            View::Area(presenter) => presenter.confirm(&mut router),
            View::RemoveArea(presenter, _) => presenter.confirm(&mut router),
            View::Blocked(presenter) => presenter.confirm(&mut router),
        }
        ports::refresh(&self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Inventory Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = DialogRouter::default();
        match &mut self.view {
            View::Location(presenter, _) => {
                presenter.handle(LocationFormEvent::Cancel, &mut router)
            }
            View::RemoveLocation(presenter) => {
                presenter.handle(RemoveLocationEvent::Cancel, &mut router);
            }
            View::Area(presenter) => presenter.handle(AreaFormEvent::Cancel, &mut router),
            View::RemoveArea(presenter, _) => {
                presenter.handle(RemoveAreaEvent::Cancel, &mut router);
            }
            View::Blocked(presenter) => presenter.handle(AreaBlockedEvent::Cancel, &mut router),
        }
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        match &mut self.view {
            View::Location(_, select) | View::RemoveArea(_, select) => {
                let open = select.is_open();
                select.cancel();
                open
            }
            View::RemoveLocation(_) | View::Area(_) | View::Blocked(_) => false,
        }
    }

    /// Cancel and confirm, then the next field on Add location, the only form with two.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        let mut hints = vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
        ];
        if let View::Location(presenter, _) = &self.view
            && !presenter.view_model().editing
        {
            hints.push(Hint::new(
                key(NEXT_FIELD),
                crate::msg::desktop_hint_next_field(),
            ));
        }
        Some(hints)
    }

    /// The open Dialog's title, the name a form shows or the name typed back, and whether it would
    /// confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let (frame, mut values) = match &self.view {
            View::Location(presenter, _) => {
                let view = presenter.view_model();
                (view.frame.clone(), vec![("name", view.name.text)])
            }
            View::RemoveLocation(presenter) => {
                let view = presenter.view_model();
                let typed = view.confirm.map(|field| field.text).unwrap_or_default();
                (view.frame.clone(), vec![("confirm", typed)])
            }
            View::Area(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), vec![("name", view.name.text)])
            }
            View::RemoveArea(presenter, _) => (presenter.view_model().frame.clone(), Vec::new()),
            View::Blocked(presenter) => (presenter.view_model().frame.clone(), Vec::new()),
        };
        values.insert(0, ("title", frame.title));
        values.push(("can_confirm", frame.can_confirm.to_string()));
        DialogReadback {
            kind: self.view.kind(),
            values,
        }
    }
}

impl Render for InventoryDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        match &self.view {
            View::Location(presenter, select) => {
                location_form::render(&presenter.view_model(), select, &actions, cx)
            }
            View::RemoveLocation(presenter) => {
                remove_location::render(&presenter.view_model(), &actions, cx)
            }
            View::Area(presenter) => area_form::render(&presenter.view_model(), &actions, cx),
            View::RemoveArea(presenter, select) => {
                remove_area::render(&presenter.view_model(), select, &actions, cx)
            }
            View::Blocked(presenter) => area_blocked::render(&presenter.view_model(), &actions, cx),
        }
    }
}

/// A line of help or error text under a field: muted, or in the accent colour for an error.
fn note(text: String, error: bool, cx: &App) -> impl IntoElement {
    div()
        .mt(px(4.0))
        .text_size(px(11.5))
        .text_color(if error {
            color::accent_text(cx)
        } else {
            color::muted(cx)
        })
        .child(text)
}
