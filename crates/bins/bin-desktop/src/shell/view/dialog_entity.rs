//! The Dialog host for a module's Dialogs (#820): the open Dialog is a gpui Entity holding its
//! Presenter, so every key and click updates the Presenter with a context, and the Dialog draws
//! itself as a View. Units, Bills, Pay, Payees, Institutions, Accounts, Categories and Delete
//! account live here; the other Dialogs keep today's host (`shell::view::dialog_host`) until their
//! slices.
//!
//! Every key reaches the open Dialog through `lib-shell`'s Dialog keymap ([`DialogCommand`]), so
//! `Enter`, `Esc`, `Tab` and select stepping are the same in each Dialog and follow
//! `[keybindings]`. What a command means is the Dialog's call ([`HostedDialog::key`]), except
//! `back`, which the Shell decides: it closes an open select first, otherwise cancels the Dialog
//! and closes it. A key the keymap doesn't bind types into the focused field.
//!
//! A Dialog's Presenter closes it through its Router. The Dialog then hands a confirm's outcome to
//! the page that opened it and emits [`DialogClosed`], which the Shell closes the host on, so a
//! key and a click close it the same way.

use std::fmt;
use std::rc::Rc;

use gpui::{AnyEntity, AnyView, App, AppContext as _, Context, Entity, EventEmitter, Render};
use gpui::{EntityId, Subscription, WeakEntity, Window};
use lib_shell::DialogCommand;
use lib_viper::keymap::Keymap;

use crate::dialog::OnClick;
use crate::form::select_field::OnOptionClick;
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use crate::shell::view::statusline::Hint;
use crate::view::event::ViewEvent;

/// A Dialog's Presenter has closed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogClosed;

/// A module's Dialog as the host holds it: an Entity owning its Presenter and its widget state.
pub trait HostedDialog: Render + EventEmitter<DialogClosed> + Sized + 'static {
    /// A key as the Dialog's View reads it: `Enter` opens a closed select, picks from an open one,
    /// else answers [`DialogOutcome::Confirm`] only while the form is valid; `Tab` moves field;
    /// the arrows step a select; anything else types.
    fn key(&mut self, key: DialogKey, cx: &mut Context<'_, Self>) -> DialogOutcome;

    /// Whether the form would confirm now.
    fn can_confirm(&self) -> bool;

    /// Sends the Presenter its Confirm. One that lands hands its outcome to the page and emits
    /// [`DialogClosed`]; a refusal stays on the form.
    fn confirm(&mut self, cx: &mut Context<'_, Self>);

    /// Sends the Presenter its Cancel, which closes the Dialog having written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>);

    /// Closes an open select's list. Returns whether one was open.
    fn close_open_select(&mut self) -> bool;

    /// The status line's hints while the Dialog is open, keyed as `keymap` binds them; `None`
    /// leaves the View's.
    fn hints(&self, _keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        None
    }

    /// The open Dialog's state for the UI tests.
    fn readback(&self, cx: &App) -> DialogReadback;
}

/// The key a command stands for in the Dialog's View: `Back` never reaches the Dialog, and
/// `Submit` is the host's own.
fn command_key(command: DialogCommand) -> Option<DialogKey> {
    match command {
        DialogCommand::Select => Some(DialogKey::Enter),
        DialogCommand::NextField => Some(DialogKey::Tab),
        DialogCommand::PreviousField => Some(DialogKey::BackTab),
        DialogCommand::SelectNext => Some(DialogKey::Down),
        DialogCommand::SelectPrevious => Some(DialogKey::Up),
        DialogCommand::Back | DialogCommand::Submit => None,
    }
}

/// Runs `key` on `dialog`, confirming when its View says so.
fn press<D: HostedDialog>(dialog: &mut D, key: DialogKey, cx: &mut Context<'_, D>) {
    if dialog.key(key, cx) == DialogOutcome::Confirm {
        dialog.confirm(cx);
    }
}

/// What the Shell holds of the open Dialog, whatever its type.
trait AnyHosted {
    fn command(&self, command: DialogCommand, cx: &mut App);
    fn typed(&self, key: DialogKey, cx: &mut App);
    fn close_open_select(&self, cx: &mut App) -> bool;
    fn cancel(&self, cx: &mut App);
    fn hints(&self, keymap: &Keymap<DialogCommand>, cx: &App) -> Option<Vec<Hint>>;
    fn readback(&self, cx: &App) -> DialogReadback;
    fn view(&self) -> AnyView;
    fn entity(&self) -> AnyEntity;
    fn on_closed(&self, closed: Box<dyn Fn(&mut App)>, cx: &mut App) -> Subscription;
}

impl<D: HostedDialog> AnyHosted for Entity<D> {
    fn command(&self, command: DialogCommand, cx: &mut App) {
        self.update(cx, |dialog, cx| {
            match command_key(command) {
                Some(key) => press(dialog, key, cx),
                // `submit` confirms a valid form from any field, a select or a multi-line box.
                None if command == DialogCommand::Submit && dialog.can_confirm() => {
                    dialog.confirm(cx);
                }
                None => {}
            }
            cx.notify();
        });
    }

    fn typed(&self, key: DialogKey, cx: &mut App) {
        // A key the keymap left unbound (`tab` once `next_field` moves) is no command here.
        if matches!(
            key,
            DialogKey::Char(_)
                | DialogKey::Backspace
                | DialogKey::Left
                | DialogKey::Right
                | DialogKey::Ctrl(_)
        ) {
            self.update(cx, |dialog, cx| {
                press(dialog, key, cx);
                cx.notify();
            });
        }
    }

    fn close_open_select(&self, cx: &mut App) -> bool {
        self.update(cx, |dialog, cx| {
            cx.notify();
            dialog.close_open_select()
        })
    }

    fn cancel(&self, cx: &mut App) {
        self.update(cx, |dialog, cx| dialog.cancel(cx));
    }

    fn hints(&self, keymap: &Keymap<DialogCommand>, cx: &App) -> Option<Vec<Hint>> {
        self.read(cx).hints(keymap)
    }

    fn readback(&self, cx: &App) -> DialogReadback {
        self.read(cx).readback(cx)
    }

    fn view(&self) -> AnyView {
        self.clone().into()
    }

    fn entity(&self) -> AnyEntity {
        self.clone().into_any()
    }

    fn on_closed(&self, closed: Box<dyn Fn(&mut App)>, cx: &mut App) -> Subscription {
        cx.subscribe(self, move |_, _: &DialogClosed, cx| closed(cx))
    }
}

/// The open hosted Dialog, as the Shell holds it and a page hands it over.
#[derive(Clone)]
pub struct DialogHandle(Rc<dyn AnyHosted>);

impl fmt::Debug for DialogHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DialogHandle").field(&self.id()).finish()
    }
}

/// Two handles are the same when they hold the same Dialog Entity.
impl PartialEq for DialogHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for DialogHandle {}

impl DialogHandle {
    pub fn new<D: HostedDialog>(dialog: Entity<D>) -> Self {
        Self(Rc::new(dialog))
    }

    pub fn id(&self) -> EntityId {
        self.0.entity().entity_id()
    }

    /// A Dialog keymap command other than `back`.
    pub fn command(&self, command: DialogCommand, cx: &mut App) {
        self.0.command(command, cx);
    }

    /// A key the Dialog keymap doesn't bind.
    pub fn typed(&self, key: DialogKey, cx: &mut App) {
        self.0.typed(key, cx);
    }

    pub fn close_open_select(&self, cx: &mut App) -> bool {
        self.0.close_open_select(cx)
    }

    pub fn cancel(&self, cx: &mut App) {
        self.0.cancel(cx);
    }

    pub fn hints(&self, keymap: &Keymap<DialogCommand>, cx: &App) -> Option<Vec<Hint>> {
        self.0.hints(keymap, cx)
    }

    pub fn readback(&self, cx: &App) -> DialogReadback {
        self.0.readback(cx)
    }

    pub fn view(&self) -> AnyView {
        self.0.view()
    }

    /// The open Dialog as a `D`, `None` when another module's is open.
    pub fn downcast<D: HostedDialog>(&self) -> Option<Entity<D>> {
        self.0.entity().downcast::<D>().ok()
    }

    /// Runs `closed` once the Dialog's Presenter closes it, for as long as the subscription lives.
    pub fn on_closed(&self, closed: impl Fn(&mut App) + 'static, cx: &mut App) -> Subscription {
        self.0.on_closed(Box::new(closed), cx)
    }
}

/// Opens the Dialog `build` makes over page `P`, as an Entity in the host.
pub fn host_dialog<D, P>(cx: &mut Context<'_, P>, build: impl FnOnce(&mut Context<'_, D>) -> D)
where
    D: HostedDialog,
    P: EventEmitter<ViewEvent> + 'static,
{
    let dialog = cx.new(build);
    cx.emit(ViewEvent::HostDialog(DialogHandle::new(dialog)));
}

/// A click that picks a value of `T`.
pub type OnChoose<T> = Rc<dyn Fn(T, &mut Window, &mut App)>;

/// The open Dialog's clicks, each one input (`crate::input::send`): the cancel and confirm
/// buttons, and the changes its fields and lists make to it.
pub struct DialogClicks<D> {
    this: WeakEntity<D>,
    pub cancel: OnClick,
    pub confirm: OnClick,
}

impl<D> Clone for DialogClicks<D> {
    fn clone(&self) -> Self {
        Self {
            this: self.this.clone(),
            cancel: self.cancel.clone(),
            confirm: self.confirm.clone(),
        }
    }
}

impl<D: HostedDialog> DialogClicks<D> {
    pub fn new(cx: &Context<'_, D>) -> Self {
        let this = cx.weak_entity();
        Self {
            cancel: run(this.clone(), |dialog, cx| dialog.cancel(cx)),
            confirm: run(this.clone(), |dialog, cx| {
                if dialog.can_confirm() {
                    dialog.confirm(cx);
                }
            }),
            this,
        }
    }

    /// A click that runs `change` on the Dialog.
    pub fn click(&self, change: impl Fn(&mut D) + 'static) -> OnClick {
        run(self.this.clone(), move |dialog, _| change(dialog))
    }

    /// A click on row `index` of an open list, running `change` with it.
    pub fn option(&self, change: impl Fn(&mut D, usize) + 'static) -> OnOptionClick {
        self.choose(change)
    }

    /// A click that picks a value (a list row, a segment), running `change` with it.
    pub fn choose<T: 'static>(&self, change: impl Fn(&mut D, T) + 'static) -> OnChoose<T> {
        let this = self.this.clone();
        let change = Rc::new(change);
        Rc::new(move |value, _window: &mut Window, cx: &mut App| {
            let change = change.clone();
            crate::input::send(cx, |cx| {
                // A Dialog that has closed takes no click.
                let _ = this.update(cx, |dialog, cx| {
                    change(dialog, value);
                    cx.notify();
                });
            });
        })
    }
}

/// A click running `change` on the Dialog as one input, then redrawing it.
fn run<D: HostedDialog>(
    this: WeakEntity<D>,
    change: impl Fn(&mut D, &mut Context<'_, D>) + 'static,
) -> OnClick {
    Rc::new(move |_window: &mut Window, cx: &mut App| {
        crate::input::send(cx, |cx| {
            // A Dialog that has closed takes no click.
            let _ = this.update(cx, |dialog, cx| {
                change(dialog, cx);
                cx.notify();
            });
        });
    })
}
