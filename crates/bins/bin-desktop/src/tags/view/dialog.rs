//! The Add tag and Edit tag Dialog's View, the open one an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`TagsDialog`] holds the `lib-tags` form Presenter. Every Dialog
//! command and typed key reaches the Presenter as a `lib-viper` `DialogInput`, which it reads
//! itself, and every click becomes a Presenter event; the View draws the Presenter's view model and
//! nothing else. The Remove tag and Merge tags Dialogs are `crate::tag_removal`'s (#1003).
//!
//! A confirmed outcome is handed to the page, and the host closes on [`DialogClosed`]. The
//! Presenter closes its Dialog through the host's [`HostRouter`]. A Toast the Presenter raises
//! goes onto the bin's queue, which the input's `send` drains.

mod tag_form;

use gpui::{App, Context, EventEmitter, IntoElement, Render, Window};
use lib_core::TagId;
use lib_shell::{DialogCommand, Hint};
use lib_tags::presenter::{TagFormEvent, TagFormPresenter};
use lib_tags::{SharedPorts, TagsOutcome};
use lib_viper::dialog::DialogInput;
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostRouter, HostedDialog};
use crate::shell::view::dialog_host::DialogReadback;

/// The open Tags Dialog's clicks.
type Clicks = DialogClicks<TagsDialog>;

/// What a confirmed Tags Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(TagsOutcome, &mut App)>;

/// The open Add tag or Edit tag Dialog: its Presenter.
pub struct TagsDialog {
    presenter: TagFormPresenter,
    landed: Landed,
}

impl TagsDialog {
    /// The Add tag Dialog on `ports`. A confirm hands its outcome to `landed`.
    pub(crate) fn add(ports: &SharedPorts, landed: Landed) -> Self {
        Self {
            presenter: TagFormPresenter::add(ports.clone()),
            landed,
        }
    }

    /// The Edit tag Dialog on Tag `id`, or `None` when it is gone.
    pub(crate) fn edit(ports: &SharedPorts, id: TagId, landed: Landed) -> Option<Self> {
        Some(Self {
            presenter: TagFormPresenter::edit(ports.clone(), id)?,
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

/// The host's Router, as a Tags Dialog's Presenter closes through it.
type Router = HostRouter<TagsOutcome>;

impl EventEmitter<DialogClosed> for TagsDialog {}

impl HostedDialog for TagsDialog {
    /// Every input is the Presenter's to read.
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) {
        let mut router = Router::default();
        self.presenter
            .handle(TagFormEvent::Input(input), &mut router);
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
        self.presenter.handle(TagFormEvent::Cancel, &mut router);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        false
    }

    /// Cancel and confirm, then the keys the form adds: the next field, the swatch row and, on
    /// Edit, Active.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        let mut hints = vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
            Hint::new(key(NEXT_FIELD), crate::msg::desktop_hint_next_field()),
            Hint::new("\u{2190}/\u{2192}", crate::msg::desktop_hint_colour()),
        ];
        if self.presenter.view_model().editing {
            hints.push(Hint::new("space", crate::msg::desktop_hint_toggle_active()));
        }
        Some(hints)
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.presenter.view_model();
        let mut values = tag_form::readback(&view);
        values.insert(0, ("title", view.frame.title.clone()));
        values.push(("can_confirm", view.frame.can_confirm.to_string()));
        DialogReadback {
            kind: if view.editing { "EditTag" } else { "AddTag" },
            values,
        }
    }
}

impl Render for TagsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        tag_form::render(&self.presenter.view_model(), &actions, cx)
    }
}
