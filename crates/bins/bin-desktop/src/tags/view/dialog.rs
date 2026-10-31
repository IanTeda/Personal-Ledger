//! The Tags Dialogs' Views, each open one an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`TagsDialog`] holds a `lib-tags` Presenter and the widget state
//! that is gpui's to keep (whether the Merge Dialog's lists are open and their highlights). Every
//! key and click becomes a Presenter event; each View draws the Presenter's view model and nothing
//! else.
//!
//! Remove and Merge confirm through the reassign port, answered from the Ledger ([`LedgerPorts`])
//! and built around the confirm's gpui context. A confirmed outcome is handed to the page, and the
//! host closes on [`DialogClosed`]. A Toast the Presenter raises goes onto the bin's queue, which
//! the input's `send` drains.

mod merge_tags;
mod remove_tag;
mod tag_form;

use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window};
use lib_shell::DialogCommand;
use lib_tags::presenter::{
    MergeTagsEvent, MergeTagsPresenter, RemoveTagEvent, RemoveTagPresenter, TagFormEvent,
    TagFormPresenter,
};
use lib_tags::{SharedPorts, TagsDialog as Which, TagsOutcome};
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::form::select::SelectState;
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use crate::tags::ports::{self, LedgerPorts};
use crate::tags::router::DialogRouter;
use lib_shell::Hint;

/// The open Tags Dialog's clicks.
type Clicks = DialogClicks<TagsDialog>;

/// What a confirmed Tags Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(TagsOutcome, &mut App)>;

/// The open Tags Dialog: its Presenter and its widget state.
pub struct TagsDialog {
    /// The Ledger the confirm's reassign port writes, and whose read copy it brings in line.
    ledger: Entity<LedgerStore>,
    view: View,
    landed: Landed,
}

enum View {
    /// Add or Edit.
    Form(TagFormPresenter),
    Remove(RemoveTagPresenter),
    /// Merge, with the source's and the target's open-list state.
    Merge(MergeTagsPresenter, SelectState, SelectState),
}

impl View {
    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::Form(presenter) if presenter.view_model().editing => "EditTag",
            Self::Form(_) => "AddTag",
            Self::Remove(_) => "RemoveTag",
            Self::Merge(..) => "MergeTags",
        }
    }
}

impl TagsDialog {
    /// `which`'s Presenter on `ports`, or `None` when the Tag it names is gone. A confirm hands its
    /// outcome to `landed`.
    pub(crate) fn open(
        which: Which,
        ports: &SharedPorts,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Option<Self> {
        let shared = ports.clone();
        let view = match which {
            Which::AddTag => View::Form(TagFormPresenter::add(shared)),
            Which::EditTag(id) => View::Form(TagFormPresenter::edit(shared, id)?),
            Which::RemoveTag(id) => View::Remove(RemoveTagPresenter::new(&shared, id)?),
            Which::MergeTags { source, target } => View::Merge(
                MergeTagsPresenter::new(&shared, source, target),
                SelectState::default(),
                SelectState::default(),
            ),
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

impl EventEmitter<DialogClosed> for TagsDialog {}

impl HostedDialog for TagsDialog {
    /// Every key is the Tags Dialog's own: the Presenter holds every field.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        match &mut self.view {
            View::Form(presenter) => tag_form::key(presenter, key),
            View::Remove(presenter) => remove_tag::key(presenter, key),
            View::Merge(presenter, source, target) => {
                merge_tags::key(presenter, source, target, key)
            }
        }
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::Form(presenter) => presenter.view_model().frame.can_confirm,
            View::Remove(presenter) => presenter.view_model().frame.can_confirm,
            View::Merge(presenter, ..) => presenter.view_model().frame.can_confirm,
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
            View::Form(presenter) => presenter.confirm(&mut router),
            View::Remove(presenter) => presenter.confirm(&mut reassign, &mut router),
            View::Merge(presenter, ..) => presenter.confirm(&mut reassign, &mut router),
        }
        ports::refresh(&self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Tags Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = DialogRouter::default();
        match &mut self.view {
            View::Form(presenter) => presenter.handle(TagFormEvent::Cancel, &mut router),
            View::Remove(presenter) => presenter.handle(RemoveTagEvent::Cancel, &mut router),
            View::Merge(presenter, ..) => presenter.handle(MergeTagsEvent::Cancel, &mut router),
        }
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        match &mut self.view {
            View::Merge(_, source, target) => {
                let open = source.is_open() || target.is_open();
                source.cancel();
                target.cancel();
                open
            }
            View::Form(_) | View::Remove(_) => false,
        }
    }

    /// Cancel and confirm, then the keys the Dialog adds: the next field, the swatch row and
    /// Active on the form, the choose keys on Merge.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        let mut hints = vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
        ];
        match &self.view {
            View::Form(presenter) => {
                hints.push(Hint::new(
                    key(NEXT_FIELD),
                    crate::msg::desktop_hint_next_field(),
                ));
                hints.push(Hint::new(
                    "\u{2190}/\u{2192}",
                    crate::msg::desktop_hint_colour(),
                ));
                if presenter.view_model().editing {
                    hints.push(Hint::new("space", crate::msg::desktop_hint_toggle_active()));
                }
            }
            View::Remove(_) => {}
            View::Merge(..) => {
                hints.push(Hint::new(
                    key(NEXT_FIELD),
                    crate::msg::desktop_hint_next_field(),
                ));
                hints.push(Hint::new(
                    "\u{2191}/\u{2193}",
                    crate::msg::desktop_hint_choose(),
                ));
            }
        }
        Some(hints)
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let (frame, mut values) = match &self.view {
            View::Form(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), tag_form::readback(&view))
            }
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

impl Render for TagsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        match &self.view {
            View::Form(presenter) => tag_form::render(&presenter.view_model(), &actions, cx),
            View::Remove(presenter) => remove_tag::render(&presenter.view_model(), &actions, cx),
            View::Merge(presenter, source, target) => {
                merge_tags::render(&presenter.view_model(), source, target, &actions, cx)
            }
        }
    }
}
