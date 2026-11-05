//! The Delete category Dialog's View, an Entity in the Dialog host (`shell::view::dialog_entity`).
//! [`CategoryDeletionDialog`] holds `lib-category-deletion`'s Presenter. Every Dialog command and
//! typed key reaches it as a `lib-viper` `DialogInput`, which it reads itself, and every click
//! becomes a Presenter event; the View draws the Presenter's view model and nothing else.
//!
//! The Presenter holds the deletion port its confirm writes through. A confirmed outcome is handed
//! to the Categories page, and the host closes on [`DialogClosed`]. The Presenter closes its Dialog
//! through the host's [`HostRouter`]; the Toast it raises goes onto the bin's queue, which the
//! input's `send` drains.

mod delete_category;

use gpui::{App, Context, EventEmitter, IntoElement, Render, Window};
use lib_categories::CategoriesOutcome;
use lib_category_deletion::presenter::{DeleteCategoryEvent, DeleteCategoryPresenter};
use lib_shell::{DialogCommand, Hint};
use lib_viper::dialog::DialogInput;
use lib_viper::keymap::{BACK, Keymap, SELECT};

use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostRouter, HostedDialog};
use crate::shell::view::dialog_host::DialogReadback;

/// The open Dialog's clicks.
type Clicks = DialogClicks<CategoryDeletionDialog>;

/// What a confirmed Dialog hands the Categories page that opened it.
pub(in crate::category_deletion) type Landed = Box<dyn Fn(CategoriesOutcome, &mut App)>;

/// The host's Router, as the Presenter closes through it.
type Router = HostRouter<CategoriesOutcome>;

/// The open Delete category Dialog.
pub struct CategoryDeletionDialog {
    presenter: DeleteCategoryPresenter,
    landed: Landed,
}

impl CategoryDeletionDialog {
    /// `presenter`'s Dialog, whose confirm hands its outcome to `landed`.
    pub(in crate::category_deletion) fn new(
        presenter: DeleteCategoryPresenter,
        landed: Landed,
    ) -> Self {
        Self { presenter, landed }
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

impl EventEmitter<DialogClosed> for CategoryDeletionDialog {}

impl HostedDialog for CategoryDeletionDialog {
    /// Every input is the Presenter's to read; one that confirms writes through its deletion port.
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) {
        let mut router = Router::default();
        self.presenter.input(input, &mut router);
        self.closed(router, cx);
    }

    fn can_confirm(&self) -> bool {
        self.presenter.view_model().frame.can_confirm
    }

    /// Sends Confirm to the Presenter. A refusal stays on the Presenter's view model and keeps the
    /// Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = Router::default();
        self.presenter.confirm(&mut router);
        self.closed(router, cx);
    }

    /// A cancelled Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = Router::default();
        self.presenter
            .handle(DeleteCategoryEvent::Cancel, &mut router);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        false
    }

    /// Cancel and confirm.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        Some(vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
        ])
    }

    /// The Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.presenter.view_model();
        let mut values = delete_category::readback(&view);
        values.insert(0, ("title", view.frame.title.clone()));
        values.push(("can_confirm", view.frame.can_confirm.to_string()));
        DialogReadback {
            kind: "DeleteCategory",
            values,
        }
    }
}

impl Render for CategoryDeletionDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        delete_category::render(&self.presenter.view_model(), &actions, cx)
    }
}
