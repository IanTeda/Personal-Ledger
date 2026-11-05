//! The Remove tag and Merge tags Dialogs' Views, the open one an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`TagRemovalDialog`] holds a `lib-tag-removal` Presenter and the
//! widget state that is gpui's to keep (the Merge Dialog's lists' scroll windows). Every Dialog
//! command and typed key reaches the Presenter as a `lib-viper` `DialogInput`, which it reads
//! itself, and every click becomes a Presenter event; each View draws the Presenter's view model
//! and nothing else.
//!
//! Each Presenter holds the removal port its confirm writes through. A confirmed outcome is handed
//! to the Tags page, and the host closes on [`DialogClosed`]. The Presenter closes its Dialog
//! through the host's [`HostRouter`]; a Toast it raises goes onto the bin's queue, which the
//! input's `send` drains.

mod merge_tags;
mod remove_tag;

use gpui::{App, Context, EventEmitter, IntoElement, Render, Window};
use lib_shell::{DialogCommand, Hint};
use lib_tag_removal::presenter::{
    MergeTagsEvent, MergeTagsPresenter, RemoveTagEvent, RemoveTagPresenter,
};
use lib_tags::TagsOutcome;
use lib_viper::dialog::DialogInput;
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::shell::view::dialog_entity::DialogReadback;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostRouter, HostedDialog};

/// The open Dialog's clicks.
type Clicks = DialogClicks<TagRemovalDialog>;

/// What a confirmed Dialog hands the Tags page that opened it.
pub(in crate::tag_removal) type Landed = Box<dyn Fn(TagsOutcome, &mut App)>;

/// The host's Router, as either Presenter closes through it.
type Router = HostRouter<TagsOutcome>;

/// The open Remove tag or Merge tags Dialog: its Presenter and its widget state.
pub struct TagRemovalDialog {
    view: View,
    landed: Landed,
}

pub(in crate::tag_removal) enum View {
    Remove(RemoveTagPresenter),
    /// Merge, with the top row of the source's and of the target's list window.
    Merge(MergeTagsPresenter, usize, usize),
}

impl View {
    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::Remove(_) => "RemoveTag",
            Self::Merge(..) => "MergeTags",
        }
    }
}

impl TagRemovalDialog {
    /// `view`'s Dialog, whose confirm hands its outcome to `landed`.
    pub(in crate::tag_removal) fn new(view: View, landed: Landed) -> Self {
        Self { view, landed }
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

impl EventEmitter<DialogClosed> for TagRemovalDialog {}

impl HostedDialog for TagRemovalDialog {
    /// Every input is the Presenter's to read; one that confirms writes through its removal port.
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) {
        let mut router = Router::default();
        match &mut self.view {
            View::Remove(presenter) => presenter.input(input, &mut router),
            View::Merge(presenter, ..) => presenter.input(input, &mut router),
        }
        self.closed(router, cx);
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::Remove(presenter) => presenter.view_model().frame.can_confirm,
            View::Merge(presenter, ..) => presenter.view_model().frame.can_confirm,
        }
    }

    /// Sends Confirm to the Presenter. A refusal stays on the Presenter's view model and keeps the
    /// Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = Router::default();
        match &mut self.view {
            View::Remove(presenter) => presenter.confirm(&mut router),
            View::Merge(presenter, ..) => presenter.confirm(&mut router),
        }
        self.closed(router, cx);
    }

    /// A cancelled Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = Router::default();
        match &mut self.view {
            View::Remove(presenter) => presenter.handle(RemoveTagEvent::Cancel, &mut router),
            View::Merge(presenter, ..) => presenter.handle(MergeTagsEvent::Cancel, &mut router),
        }
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        match &mut self.view {
            View::Merge(presenter, ..) => {
                let view = presenter.view_model();
                let open = view.source_list.is_open() || view.target_list.is_open();
                if open {
                    presenter.handle(MergeTagsEvent::CloseLists, &mut Router::default());
                }
                open
            }
            View::Remove(_) => false,
        }
    }

    /// Cancel and confirm, then on Merge the next field and the choose keys.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        let mut hints = vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
        ];
        if let View::Merge(..) = self.view {
            hints.push(Hint::new(
                key(NEXT_FIELD),
                crate::msg::desktop_hint_next_field(),
            ));
            hints.push(Hint::new(
                "\u{2191}/\u{2193}",
                crate::msg::desktop_hint_choose(),
            ));
        }
        Some(hints)
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let (frame, mut values) = match &self.view {
            View::Remove(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), remove_tag::readback(&view))
            }
            View::Merge(presenter, ..) => {
                let view = presenter.view_model();
                (view.frame.clone(), merge_tags::readback(&view))
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

impl Render for TagRemovalDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        match &mut self.view {
            View::Remove(presenter) => remove_tag::render(&presenter.view_model(), &actions, cx),
            View::Merge(presenter, source, target) => {
                merge_tags::render(&presenter.view_model(), (source, target), &actions, cx)
            }
        }
    }
}
