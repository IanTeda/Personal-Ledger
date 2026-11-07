//! The Add tag and Edit tag Dialog's View: the `lib-tags` form Presenter's [`DialogView`], which
//! the generic `ModuleDialog` hosts in the Dialog host (ADR-0056). Every Dialog command and typed
//! key reaches the Presenter as a `lib-viper` `DialogInput`, which it reads itself, and every click
//! is a Presenter event; the View draws the view model (`tag_form.rs`), hints included, and nothing
//! else. The Remove tag and Merge tags Dialogs are `crate::tag_removal`'s (#1003).

mod tag_form;

use gpui::{AnyElement, App};
use lib_shell::Hint;
use lib_tags::presenter::{TagFormPresenter, TagFormViewModel};

use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};

impl DialogView for TagFormPresenter {
    type Widgets = ();

    fn kind(view: &TagFormViewModel) -> &'static str {
        if view.editing { "EditTag" } else { "AddTag" }
    }

    fn render(
        view: &TagFormViewModel,
        _widgets: &mut (),
        clicks: &ModuleDialogClicks<Self>,
        cx: &App,
    ) -> AnyElement {
        tag_form::render(view, clicks, cx)
    }

    /// The fields the form draws, between its title and whether it would confirm.
    fn readback(view: &TagFormViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, tag_form::readback(view))
    }

    fn hints(view: &TagFormViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}
