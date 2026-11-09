//! The Dialog host for a module's Dialogs (#820): the open Dialog is a gpui Entity holding its
//! Presenter, so every key and click updates the Presenter with a context, and the Dialog draws
//! itself as a View. Every Dialog lives here (#991, #1043).
//!
//! Every key reaches the open Dialog through `lib-shell`'s Dialog keymap ([`DialogCommand`]), so
//! `Enter`, `Esc`, `Tab` and select stepping are the same in each Dialog and follow
//! `[keybindings]`. What a command means is the Dialog's call, except `back`, which the Shell
//! decides: it closes an open select first, otherwise cancels the Dialog and closes it. A key the
//! keymap doesn't bind types into the focused field.
//!
//! Every Dialog's Presenter takes `lib-viper`'s [`DialogInput`]s: each command and typed key is one
//! ([`DialogPresenter::input_event`]), and the Presenter closes through the one [`HostRouter`].
//!
//! A Dialog's Presenter closes it through its Router. The Dialog then hands a confirm's outcome to
//! the page that opened it and emits [`DialogClosed`], which the Shell closes the host on, so a
//! key and a click close it the same way.
//!
//! [`ModuleDialog`] hosts any `lib-viper` `DialogPresenter` (ADR-0056): a domain writes a
//! [`DialogView`] on its Presenter (how it draws, reads back and is named) and no Entity of its
//! own. It is the only Dialog Entity there is, so [`DialogHandle`] holds an `Entity<ModuleDialog<P>>`
//! and nothing else.

use std::fmt;
use std::rc::Rc;

use gpui::{AnyElement, AnyEntity, AnyView, App, AppContext as _, Context, Entity, EventEmitter};
use gpui::{EntityId, IntoElement, Render, Subscription, WeakEntity, Window};
use lib_shell::DialogCommand;
use lib_toast::{Notice, Notify};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::keymap::Keymap;
use lib_viper::presenter::DialogPresenter;
use lib_viper::view::DialogFrame;

use crate::dialog::OnClick;
use crate::form::select_field::OnOptionClick;
use crate::view::event::ViewEvent;
use lib_shell::Hint;

/// A keystroke as a Dialog sees it, already stripped of modifiers by `Root`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKey {
    Backspace,
    Tab,
    /// `Shift-Tab`; a Dialog without its own handling treats it as `Tab`.
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Enter,
    /// `Ctrl-Enter`: a multi-line box takes it to save, and any other Dialog treats it as `Enter`.
    CtrlEnter,
    /// A printable character, typed without Ctrl/Alt/Cmd/Fn.
    Char(char),
    /// A character chord held with Ctrl alone (`Ctrl-N`), for a list Dialog's own keys.
    Ctrl(char),
    /// Anything else (arrows, function keys, a modified chord).
    Other,
}

/// What `Root` does after a Dialog has seen a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogOutcome {
    /// Not the Dialog's key; `Root` lets it fall through.
    Ignored,
    Handled,
    /// The form is valid and the user confirmed: `Root` applies it and closes the Dialog.
    Confirm,
}

/// A hosted Dialog's state as labelled values, read back by the UI tests: `kind` names which
/// Dialog is open (`AddTag`), and each value a field it shows (`name`, `confirm`).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogReadback {
    pub kind: &'static str,
    pub values: Vec<(&'static str, String)>,
}

impl DialogReadback {
    /// The value labelled `label`, if the Dialog shows one.
    pub fn value(&self, label: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(name, _)| *name == label)
            .map(|(_, value)| value.as_str())
    }
}

/// A Dialog's Presenter has closed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogClosed;

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

impl<P: DialogView> AnyHosted for Entity<ModuleDialog<P>> {
    fn command(&self, command: DialogCommand, cx: &mut App) {
        self.update(cx, |dialog, cx| {
            if let Some(input) = command.input() {
                dialog.handle(P::input_event(input), cx);
            }
            cx.notify();
        });
    }

    fn typed(&self, key: DialogKey, cx: &mut App) {
        // A key the keymap left unbound (`tab` once `next_field` moves) is no command here.
        if let Some(input) = typed_input(key) {
            self.update(cx, |dialog, cx| {
                dialog.handle(P::input_event(input), cx);
                cx.notify();
            });
        }
    }

    fn close_open_select(&self, cx: &mut App) -> bool {
        self.update(cx, |dialog, cx| {
            cx.notify();
            dialog.presenter.close_open_select()
        })
    }

    fn steps_list(&self, cx: &App) -> bool {
        self.read(cx).presenter.steps_list()
    }

    fn cancel(&self, cx: &mut App) {
        self.update(cx, |dialog, cx| {
            dialog.handle(P::cancel_event(), cx);
        });
    }

    /// The view model's hints: their chords are already the Dialog keymap's, so `keymap` is not
    /// read. A Dialog with none leaves the View's.
    fn hints(&self, _keymap: &Keymap<DialogCommand>, cx: &App) -> Option<Vec<Hint>> {
        let hints = P::hints(&self.read(cx).presenter.view_model());
        (!hints.is_empty()).then_some(hints)
    }

    fn readback(&self, cx: &App) -> DialogReadback {
        let view = self.read(cx).presenter.view_model();
        DialogReadback {
            kind: P::kind(&view),
            values: P::readback(&view),
        }
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
    pub fn new<P: DialogView>(dialog: Entity<ModuleDialog<P>>) -> Self {
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
    pub fn downcast<D: 'static>(&self) -> Option<Entity<D>> {
        self.0.entity().downcast::<D>().ok()
    }

    /// Runs `closed` once the Dialog's Presenter closes it, for as long as the subscription lives.
    pub fn on_closed(&self, closed: impl Fn(&mut App) + 'static, cx: &mut App) -> Subscription {
        self.0.on_closed(Box::new(closed), cx)
    }
}

/// Opens `presenter`'s Dialog in the host over page `Page`, whose confirm hands its outcome to
/// `landed` (the page's `ModulePage::landing`).
pub fn host_dialog<P: DialogView, Page: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, Page>,
    presenter: P,
    landed: impl Fn(P::Outcome, &mut App) + 'static,
) {
    let dialog = cx.new(|_| ModuleDialog::new(presenter, landed));
    cx.emit(ViewEvent::HostDialog(DialogHandle::new(dialog)));
}

/// A click that picks a value of `T`.
pub type OnChoose<T> = Rc<dyn Fn(T, &mut Window, &mut App)>;

/// The bin's half of a Dialog's Presenter: how [`ModuleDialog`] draws it, names it and reads it
/// back, each from the view model the host builds once per call. Implemented on the Presenter type
/// itself, so a module has no Dialog Entity of its own.
pub trait DialogView: DialogPresenter + Sized + 'static {
    /// The widget state that is gpui's to keep beside the Presenter, which the Presenter never
    /// sees: a select's list window, a scroll handle. `()` for a Dialog with none.
    type Widgets: Default + 'static;

    /// Which Dialog is open, as the UI tests name it (`AddTag`).
    fn kind(view: &Self::ViewModel) -> &'static str;

    /// Draws the Dialog from `view`. Every click goes through `clicks`, as one input.
    fn render(
        view: &Self::ViewModel,
        widgets: &mut Self::Widgets,
        clicks: &ModuleDialogClicks<Self>,
        cx: &App,
    ) -> AnyElement;

    /// The Dialog's labelled values for the UI tests ([`DialogReadback`]), [`framed`] by its title
    /// and whether it would confirm.
    fn readback(view: &Self::ViewModel) -> Vec<(&'static str, String)>;

    /// The status line's hints while the Dialog is open: its view model's, which its Presenter
    /// built from the `DialogKeys` it was handed at open.
    fn hints(view: &Self::ViewModel) -> Vec<Hint>;
}

/// `values` between the Dialog's title and whether it would confirm, as every Dialog reads back.
pub fn framed(
    frame: &DialogFrame,
    mut values: Vec<(&'static str, String)>,
) -> Vec<(&'static str, String)> {
    values.insert(0, ("title", frame.title.clone()));
    values.push(("can_confirm", frame.can_confirm.to_string()));
    values
}

/// What a confirmed Dialog hands the page that opened it, inside the confirm's input.
type Landed<O> = Box<dyn Fn(O, &mut App)>;

/// The open Dialog of any module: one Dialog Presenter, the widget state its [`DialogView`] keeps
/// beside it, and where a confirm's outcome lands. Every input, key or click, is one
/// [`Presenter::handle`](lib_viper::presenter::Presenter::handle) call through a fresh
/// [`HostRouter`], after which the Dialog acts on a close.
pub struct ModuleDialog<P: DialogView> {
    presenter: P,
    widgets: P::Widgets,
    landed: Landed<P::Outcome>,
}

impl<P: DialogView> ModuleDialog<P> {
    /// A Dialog over `presenter`, whose confirm hands its outcome to `landed`.
    pub fn new(presenter: P, landed: impl Fn(P::Outcome, &mut App) + 'static) -> Self {
        Self {
            presenter,
            widgets: P::Widgets::default(),
            landed: Box::new(landed),
        }
    }

    /// Opens `presenter`'s Dialog in the host over page `Page`, whose confirm hands its outcome to
    /// `landed` (the page's `ModulePage::landing`).
    pub fn open<Page: EventEmitter<ViewEvent> + 'static>(
        cx: &mut Context<'_, Page>,
        presenter: P,
        landed: impl Fn(P::Outcome, &mut App) + 'static,
    ) {
        host_dialog(cx, presenter, landed);
    }

    /// The Presenter, for the Shell to feed what changes behind the Dialog (a Toast raised under
    /// the Toast history).
    pub fn presenter_mut(&mut self) -> &mut P {
        &mut self.presenter
    }

    /// The Dialog as drawn, for a test's snapshot.
    pub fn view_model(&self) -> P::ViewModel {
        self.presenter.view_model()
    }

    /// The widget state the View keeps, for the Shell to place the Dialog by as it lays out (the
    /// Budgets Switcher's left edge).
    pub fn widgets_mut(&mut self) -> &mut P::Widgets {
        &mut self.widgets
    }

    /// Hands `event` to the Presenter. Once it has closed the Dialog, hands a confirm's outcome
    /// to the page and tells the host, returning `true`.
    fn handle(&mut self, event: P::Event, cx: &mut Context<'_, Self>) -> bool {
        let mut router = HostRouter::default();
        self.presenter.handle(event, &mut router);
        let Some(outcome) = router.closed() else {
            return false;
        };
        if let Some(outcome) = outcome {
            (self.landed)(outcome, cx);
        }
        cx.emit(DialogClosed);
        true
    }
}

impl<P: DialogView> EventEmitter<DialogClosed> for ModuleDialog<P> {}

impl<P: DialogView> Render for ModuleDialog<P> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let clicks = ModuleDialogClicks::new(cx);
        P::render(&self.presenter.view_model(), &mut self.widgets, &clicks, cx)
    }
}

/// A [`ModuleDialog`]'s clicks, each a list of its Presenter's events sent as one input
/// (`crate::input::send`) and stopping at the one that closes the Dialog: the cancel and confirm
/// buttons, and whatever its fields and lists send. The twin of `PageClicks` for a Dialog.
pub struct ModuleDialogClicks<P: DialogView> {
    this: WeakEntity<ModuleDialog<P>>,
    pub cancel: OnClick,
    /// Sends the confirm only while the form would confirm, as the button is drawn disabled.
    pub confirm: OnClick,
}

impl<P: DialogView> ModuleDialogClicks<P> {
    fn new(cx: &Context<'_, ModuleDialog<P>>) -> Self {
        let this = cx.weak_entity();
        Self {
            cancel: on_click(this.clone(), |_| vec![P::cancel_event()]),
            confirm: on_click(this.clone(), |dialog| {
                if dialog.presenter.can_confirm() {
                    vec![P::confirm_event()]
                } else {
                    Vec::new()
                }
            }),
            this,
        }
    }

    /// A click sending `event`.
    pub fn send(&self, event: P::Event) -> OnClick
    where
        P::Event: Clone,
    {
        self.send_all(vec![event])
    }

    /// A click sending `events` in order, as one input: a click that focuses a field, then acts.
    pub fn send_all(&self, events: Vec<P::Event>) -> OnClick
    where
        P::Event: Clone,
    {
        on_click(self.this.clone(), move |_| events.clone())
    }

    /// A click that picks a value (a segment, a chip), sending the events `events` builds from it.
    pub fn choose<T: 'static>(&self, events: impl Fn(T) -> Vec<P::Event> + 'static) -> OnChoose<T> {
        let this = self.this.clone();
        Rc::new(move |value, _window: &mut Window, cx: &mut App| {
            deliver(&this, events(value), cx);
        })
    }

    /// A click on row `index` of a list (an open select, a swatch row), sending the events
    /// `events` builds from it.
    pub fn option(&self, events: impl Fn(usize) -> Vec<P::Event> + 'static) -> OnOptionClick {
        let this = self.this.clone();
        Rc::new(move |index, _window: &mut Window, cx: &mut App| {
            deliver(&this, events(index), cx);
        })
    }
}

/// A click sending the events `events` reads off the Dialog as it is clicked.
fn on_click<P: DialogView>(
    this: WeakEntity<ModuleDialog<P>>,
    events: impl Fn(&ModuleDialog<P>) -> Vec<P::Event> + 'static,
) -> OnClick {
    Rc::new(move |_window: &mut Window, cx: &mut App| {
        // A Dialog that has closed takes no click.
        let Ok(events) = this.read_with(cx, |dialog, _| events(dialog)) else {
            return;
        };
        deliver(&this, events, cx);
    })
}

/// Sends `events` to the Dialog as one input, stopping once one closes it, then redraws it.
fn deliver<P: DialogView>(this: &WeakEntity<ModuleDialog<P>>, events: Vec<P::Event>, cx: &mut App) {
    crate::input::send(cx, |cx| {
        // A Dialog that has closed takes no click.
        let _ = this.update(cx, |dialog, cx| {
            for event in events {
                if dialog.handle(event, cx) {
                    break;
                }
            }
            cx.notify();
        });
    });
}
