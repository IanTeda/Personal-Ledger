//! The Categories Dialogs' Views, each open one an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`CategoriesDialog`] holds a `lib-categories` Presenter and the
//! widget state that is gpui's to keep (whether the Parent category list is open and its
//! highlight). Every key and click becomes a Presenter event; each View draws the Presenter's
//! view model and nothing else.
//!
//! Confirm is lent the ports the Desktop answers from its Ledger ([`LedgerPorts`]), built around
//! the confirm's gpui context: the Monthly budget for the form, the reassign port for Delete. A
//! confirmed outcome is handed to the page, and the host closes on [`DialogClosed`]. A Toast the
//! Presenter raises goes onto the bin's queue, which the input's `send` drains.

mod category_form;
mod delete_category;

use chrono::NaiveDate;
use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window};
use lib_categories::presenter::{
    CategoryFormEvent, CategoryFormPresenter, DeleteCategoryEvent, DeleteCategoryPresenter,
};
use lib_categories::{CategoriesDialog as Which, CategoriesOutcome, SharedCategories};
use lib_shell::DialogCommand;
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::categories::ports::{self, LedgerPorts};
use crate::categories::router::DialogRouter;
use crate::form::select::SelectState;
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use lib_shell::Hint;

/// The open Categories Dialog's clicks.
type Clicks = DialogClicks<CategoriesDialog>;

/// What a confirmed Categories Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(CategoriesOutcome, &mut App)>;

/// The open Categories Dialog: its Presenter and its widget state.
pub struct CategoriesDialog {
    /// The Ledger the confirm's ports read and write, and whose read copy it brings in line.
    ledger: Entity<LedgerStore>,
    today: NaiveDate,
    view: View,
    landed: Landed,
}

enum View {
    /// Add or Edit, with the Parent category select's open state.
    Form(CategoryFormPresenter, SelectState),
    Delete(DeleteCategoryPresenter),
}

impl View {
    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::Form(presenter, _) if presenter.view_model().editing => "EditCategory",
            Self::Form(..) => "AddCategory",
            Self::Delete(_) => "DeleteCategory",
        }
    }
}

impl CategoriesDialog {
    /// `which`'s Presenter on `categories`, its usage and Monthly budget read from `ledger` as of
    /// `today`, or `None` when the Category it names is gone. A confirm hands its outcome to
    /// `landed`.
    pub(crate) fn open(
        which: Which,
        categories: &SharedCategories,
        ledger: &Entity<LedgerStore>,
        today: NaiveDate,
        landed: Landed,
        cx: &mut App,
    ) -> Option<Self> {
        let reads = LedgerPorts { ledger, today, cx };
        let shared = categories.clone();
        let view = match which {
            Which::AddCategory { parent } => View::Form(
                CategoryFormPresenter::add(shared, &reads, parent),
                SelectState::default(),
            ),
            Which::EditCategory(id) => View::Form(
                CategoryFormPresenter::edit(shared, &reads, &reads, id)?,
                SelectState::default(),
            ),
            Which::DeleteCategory(id) => {
                View::Delete(DeleteCategoryPresenter::new(&shared, &reads, id)?)
            }
        };
        Some(Self {
            ledger: ledger.clone(),
            today,
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

/// Sends `event` to `presenter`, returning what its Router was asked.
fn send<P, E>(
    presenter: &mut P,
    event: E,
    handle: fn(&mut P, E, &mut dyn lib_categories::CategoriesRouter),
) -> DialogRouter {
    let mut router = DialogRouter::default();
    handle(presenter, event, &mut router);
    router
}

impl EventEmitter<DialogClosed> for CategoriesDialog {}

impl HostedDialog for CategoriesDialog {
    /// Every key is the Categories Dialog's own: the Presenter holds every field.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        match &mut self.view {
            View::Form(presenter, select) => category_form::key(presenter, select, key),
            View::Delete(presenter) => delete_category::key(presenter, key),
        }
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::Form(presenter, _) => presenter.view_model().frame.can_confirm,
            View::Delete(presenter) => presenter.view_model().frame.can_confirm,
        }
    }

    /// Sends Confirm to the Presenter, lent the Ledger's ports, with the Ledger read copy in line
    /// either side. A refusal stays on the Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ledger, cx);
        let mut router = DialogRouter::default();
        let mut ports = LedgerPorts {
            ledger: &self.ledger,
            today: self.today,
            cx,
        };
        match &mut self.view {
            View::Form(presenter, _) => presenter.confirm(&mut ports, &mut router),
            View::Delete(presenter) => presenter.confirm(&mut ports, &mut router),
        }
        ports::refresh(&self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Categories Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = match &mut self.view {
            View::Form(presenter, _) => send(
                presenter,
                CategoryFormEvent::Cancel,
                CategoryFormPresenter::handle,
            ),
            View::Delete(presenter) => send(
                presenter,
                DeleteCategoryEvent::Cancel,
                DeleteCategoryPresenter::handle,
            ),
        };
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        match &mut self.view {
            View::Form(_, select) => {
                let open = select.is_open();
                select.cancel();
                open
            }
            View::Delete(_) => false,
        }
    }

    /// Cancel and confirm, and on the form, the key that moves field.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        let mut hints = vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
        ];
        if matches!(self.view, View::Form(..)) {
            hints.push(Hint::new(
                key(NEXT_FIELD),
                crate::msg::desktop_hint_next_field(),
            ));
        }
        Some(hints)
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let (frame, mut values) = match &self.view {
            View::Form(presenter, _) => {
                let view = presenter.view_model();
                (view.frame.clone(), category_form::readback(&view))
            }
            View::Delete(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), delete_category::readback(&view))
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

impl Render for CategoriesDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        match &self.view {
            View::Form(presenter, select) => {
                category_form::render(&presenter.view_model(), select, &actions, cx)
            }
            View::Delete(presenter) => {
                delete_category::render(&presenter.view_model(), &actions, cx)
            }
        }
    }
}
