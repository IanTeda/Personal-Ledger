//! The Add and Edit category Dialog's View, the open one an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`CategoriesDialog`] holds a `lib-categories` form Presenter and
//! the widget state that is gpui's to keep (the Parent category list's scroll window). Every Dialog
//! command and typed key reaches the Presenter as a `lib-viper` `DialogInput`, which it reads
//! itself, and every click becomes a Presenter event; the View draws the Presenter's view model
//! and nothing else. The Delete category Dialog is the Category Deletion module's
//! (`crate::category_deletion`, #1005).
//!
//! The Presenter closes its Dialog through the host's [`HostRouter`]. An input and a confirm are
//! lent the ports the Desktop answers from its stores ([`LedgerPorts`]), built around the
//! confirm's gpui context: the usage, the Monthly budget and the handover. A confirmed outcome is
//! handed to the page, and the host closes on [`DialogClosed`]. A Toast the Presenter raises goes
//! onto the bin's queue, which the input's `send` drains.

mod category_form;

use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window};
use lib_categories::presenter::{CategoryFormEvent, CategoryFormPresenter};
use lib_categories::{CategoriesDialog as Which, CategoriesOutcome, SharedCategories};
use lib_core::Clock;
use lib_shell::DialogCommand;
use lib_viper::dialog::DialogInput;
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::announcer::Announcer;
use crate::categories::ports::{self, CategoryStores, LedgerPorts};
use crate::shell::view::dialog_entity::DialogReadback;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostRouter, HostedDialog};
use lib_shell::Hint;

/// The open Categories Dialog's clicks.
type Clicks = DialogClicks<CategoriesDialog>;

/// What a confirmed Categories Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(CategoriesOutcome, &mut App)>;

/// What the Dialog's ports are built on for each call: the stores they read and write, the
/// Announcer they announce through, and the Shell's Clock, which the ports read the day from on each call.
pub(crate) struct PortsOn {
    pub stores: CategoryStores,
    pub announcer: Entity<Announcer>,
    pub clock: Clock,
}

impl PortsOn {
    /// The ports for one call around `cx`.
    fn ports<'a>(&'a self, cx: &'a mut App) -> LedgerPorts<'a> {
        LedgerPorts {
            stores: &self.stores,
            announcer: &self.announcer,
            clock: self.clock.clone(),
            cx,
        }
    }

    /// Announces any Categories write not yet announced.
    fn refresh(&self, cx: &mut App) {
        ports::refresh(&self.stores.writes, &self.announcer, cx);
    }
}

/// The open Add or Edit category Dialog: its Presenter and its widget state.
pub struct CategoriesDialog {
    /// What the confirm's ports are built on.
    on: PortsOn,
    presenter: CategoryFormPresenter,
    /// The top row of the Parent category list's window.
    scroll: usize,
    landed: Landed,
}

impl CategoriesDialog {
    /// `which`'s form Presenter on `categories`, its usage and Monthly budget read through `on`'s
    /// stores as of the day its Clock reads, or `None` when the Category it edits is gone, or `which` is a delete, which is
    /// not this Dialog's. A confirm hands its outcome to `landed`.
    pub(crate) fn open(
        which: Which,
        categories: &SharedCategories,
        on: PortsOn,
        landed: Landed,
        cx: &mut App,
    ) -> Option<Self> {
        let reads = on.ports(cx);
        let shared = categories.clone();
        let presenter = match which {
            Which::AddCategory { parent } => CategoryFormPresenter::add(shared, &reads, parent),
            Which::EditCategory(id) => CategoryFormPresenter::edit(shared, &reads, &reads, id)?,
            Which::DeleteCategory(_) => return None,
        };
        Some(Self {
            on,
            presenter,
            scroll: 0,
            landed,
        })
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

/// The host's Router, as a Categories Dialog's Presenter closes through it.
type Router = HostRouter<CategoriesOutcome>;

/// Sends `event` to `presenter`, returning what its Router was asked.
fn send<P, E>(
    presenter: &mut P,
    event: E,
    handle: fn(&mut P, E, &mut dyn lib_viper::dialog::DialogRouter<CategoriesOutcome>),
) -> Router {
    let mut router = Router::default();
    handle(presenter, event, &mut router);
    router
}

impl EventEmitter<DialogClosed> for CategoriesDialog {}

impl HostedDialog for CategoriesDialog {
    /// Every input is the Presenter's to read, lent the stores' ports. One that may confirm
    /// (`select`, `submit`) announces any Categories write before it and after it.
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) {
        let may_confirm = matches!(input, DialogInput::Select | DialogInput::Submit);
        if may_confirm {
            self.on.refresh(cx);
        }
        let mut router = Router::default();
        let mut ports = self.on.ports(cx);
        self.presenter.input(input, &mut ports, &mut router);
        if may_confirm {
            self.on.refresh(cx);
        }
        self.closed(router, cx);
    }

    fn can_confirm(&self) -> bool {
        self.presenter.view_model().frame.can_confirm
    }

    /// Sends Confirm to the Presenter, lent the stores' ports, announcing any Categories write
    /// either side. A refusal stays on the Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.on.refresh(cx);
        let mut router = Router::default();
        let mut ports = self.on.ports(cx);
        self.presenter.confirm(&mut ports, &mut router);
        self.on.refresh(cx);
        self.closed(router, cx);
    }

    /// A cancelled Categories Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = send(
            &mut self.presenter,
            CategoryFormEvent::Cancel,
            CategoryFormPresenter::handle,
        );
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        let open = self.presenter.view_model().parent_list.is_open();
        if open {
            send(
                &mut self.presenter,
                CategoryFormEvent::CloseParents,
                CategoryFormPresenter::handle,
            );
        }
        open
    }

    /// Cancel, confirm and the key that moves field.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        Some(vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
            Hint::new(key(NEXT_FIELD), crate::msg::desktop_hint_next_field()),
        ])
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.presenter.view_model();
        let mut values = category_form::readback(&view);
        values.insert(0, ("title", view.frame.title.clone()));
        values.push(("can_confirm", view.frame.can_confirm.to_string()));
        DialogReadback {
            kind: if view.editing {
                "EditCategory"
            } else {
                "AddCategory"
            },
            values,
        }
    }
}

impl Render for CategoriesDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        category_form::render(&self.presenter.view_model(), &mut self.scroll, &actions, cx)
    }
}
