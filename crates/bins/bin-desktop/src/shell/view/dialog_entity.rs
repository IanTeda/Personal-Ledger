//! The Dialog host for a module's Dialogs (#820): the open Dialog is a gpui Entity holding its
//! Presenter, so every key and click updates the Presenter with a context, and the Dialog draws
//! itself as a View. Every module's Dialog lives here (#991); only the Dialogs not yet on a
//! module keep the legacy host (`shell::view::dialog_host`).
//!
//! Every key reaches the open Dialog through `lib-shell`'s Dialog keymap ([`DialogCommand`]), so
//! `Enter`, `Esc`, `Tab` and select stepping are the same in each Dialog and follow
//! `[keybindings]`. What a command means is the Dialog's call, except `back`, which the Shell
//! decides: it closes an open select first, otherwise cancels the Dialog and closes it. A key the
//! keymap doesn't bind types into the focused field.
//!
//! Every Dialog's Presenter takes `lib-viper`'s [`DialogInput`]s: each command and typed key is one
//! ([`HostedDialog::input`]), and the Presenter closes through the one [`HostRouter`].
//!
//! A Dialog's Presenter closes it through its Router. The Dialog then hands a confirm's outcome to
//! the page that opened it and emits [`DialogClosed`], which the Shell closes the host on, so a
//! key and a click close it the same way.

use std::fmt;
use std::rc::Rc;

use gpui::{AnyEntity, AnyView, App, AppContext as _, Context, Entity, EventEmitter, Render};
use gpui::{EntityId, Subscription, WeakEntity, Window};
use lib_shell::DialogCommand;
use lib_toast::{Notice, Notify};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::keymap::Keymap;

use crate::dialog::OnClick;
use crate::form::select_field::OnOptionClick;
use crate::shell::view::dialog_host::{DialogKey, DialogReadback};
use crate::view::event::ViewEvent;
use lib_shell::Hint;

/// A Dialog's Presenter has closed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogClosed;

/// A module's Dialog as the host holds it: an Entity owning its Presenter and its widget state.
pub trait HostedDialog: Render + EventEmitter<DialogClosed> + Sized + 'static {
    /// A Dialog input: the Presenter decides what it means, and confirms or closes through a
    /// [`HostRouter`].
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>);

    /// Whether the form would confirm now.
    fn can_confirm(&self) -> bool;

    /// Sends the Presenter its Confirm. One that lands hands its outcome to the page and emits
    /// [`DialogClosed`]; a refusal stays on the form.
    fn confirm(&mut self, cx: &mut Context<'_, Self>);

    /// Sends the Presenter its Cancel, which closes the Dialog having written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>);

    /// Closes an open select's list. Returns whether one was open.
    fn close_open_select(&mut self) -> bool;

    /// Whether the Dialog steps a list right now, so the Dialog keymap's list step (`j`/`k` by
    /// default) reaches it as a select step. Any other Dialog takes those keys as typing.
    fn steps_list(&self) -> bool {
        false
    }

    /// The status line's hints while the Dialog is open, keyed as `keymap` binds them; `None`
    /// leaves the View's.
    fn hints(&self, _keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        None
    }

    /// The open Dialog's state for the UI tests.
    fn readback(&self, cx: &App) -> DialogReadback;
}

/// A typed key as a Presenter's input: a character, `Backspace` or an arrow along a row. The
/// others type nothing.
fn typed_input(key: DialogKey) -> Option<DialogInput> {
    match key {
        DialogKey::Char(ch) => Some(DialogInput::Char(ch)),
        DialogKey::Backspace => Some(DialogInput::Backspace),
        DialogKey::Left => Some(DialogInput::Left),
        DialogKey::Right => Some(DialogInput::Right),
        _ => None,
    }
}

/// The one Router every module Dialog's Presenter closes through (`lib_viper::dialog`): it
/// records the close for the Dialog to act on once its Presenter returns, and queues each Notice
/// on the bin's queue for the input's `send` to drain. A Dialog builds one for each Presenter call.
pub struct HostRouter<O> {
    closed: Option<Option<O>>,
}

impl<O> Default for HostRouter<O> {
    fn default() -> Self {
        Self { closed: None }
    }
}

impl<O> HostRouter<O> {
    /// `Some` once the Presenter closed its Dialog, holding what its confirm did (`None` for a
    /// cancel).
    pub fn closed(self) -> Option<Option<O>> {
        self.closed
    }
}

impl<O> DialogRouter<O> for HostRouter<O> {
    fn close(&mut self, outcome: Option<O>) {
        self.closed = Some(outcome);
    }
}

impl<O> Notify for HostRouter<O> {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

/// The Router the Help card's Presenter closes through: it has no outcome to hand a page.
pub type HelpRouter = HostRouter<()>;

/// What the Shell holds of the open Dialog, whatever its type.
trait AnyHosted {
    fn command(&self, command: DialogCommand, cx: &mut App);
    fn typed(&self, key: DialogKey, cx: &mut App);
    fn close_open_select(&self, cx: &mut App) -> bool;
    fn steps_list(&self, cx: &App) -> bool;
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
            if let Some(input) = command.input() {
                dialog.input(input, cx);
            }
            cx.notify();
        });
    }

    fn typed(&self, key: DialogKey, cx: &mut App) {
        // A key the keymap left unbound (`tab` once `next_field` moves) is no command here.
        if let Some(input) = typed_input(key) {
            self.update(cx, |dialog, cx| {
                dialog.input(input, cx);
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

    fn steps_list(&self, cx: &App) -> bool {
        self.read(cx).steps_list()
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

    /// Whether the Dialog takes the list step as a step rather than typing.
    pub fn steps_list(&self, cx: &App) -> bool {
        self.0.steps_list(cx)
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

    /// A click that runs `change` on the Dialog with its context: one that closes the Dialog.
    pub fn act(&self, change: impl Fn(&mut D, &mut Context<'_, D>) + 'static) -> OnClick {
        run(self.this.clone(), change)
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
