//! The Presenter contract every VIPER module's Presenters keep (ADR-0056): [`Presenter`] for any
//! Presenter, with [`PagePresenter`] for a page's and [`DialogPresenter`] for a Dialog's. A Client
//! hosts a Presenter through these traits alone, so one generic host serves every module (the
//! Desktop's `ModulePage` and `ModuleDialog`), and both Clients and the tests call a Presenter the
//! one way.
//!
//! Associated types rather than type parameters: a Presenter has exactly one event type, view
//! model and Router, so the traits say so and a host's bounds stay short.
//!
//! The Router is a generic associated type, `Router<'r>`, because the Router a Client lends a
//! Presenter borrows that call's context (the Desktop's holds its gpui `Context`), so it lives no
//! longer than the call. A plain `type Router: ?Sized` would default its trait object to
//! `'static`, which no borrowing Router can be.

use crate::dialog::{DialogInput, DialogRouter};
use crate::display::PageDisplay;
use crate::keymap::{KeyBindingConfig, Keymap};

/// A Presenter: it takes its events through [`Self::handle`], asking for navigation through the
/// Router it is lent for that call, and builds a plain-data view model through
/// [`Self::view_model`], its text already localised, so a View only draws.
pub trait Presenter {
    /// What the Presenter takes: a key's or a click's event, never a key.
    type Event;

    /// What the View draws.
    type ViewModel;

    /// Where the Presenter asks to go: its module's Router trait object (`dyn TagsRouter + 'r`),
    /// or a Dialog's [`DialogRouter`].
    type Router<'r>: ?Sized;

    /// Handles `event`, asking `router` for what it can't do itself.
    fn handle(&mut self, event: Self::Event, router: &mut Self::Router<'_>);

    fn view_model(&self) -> Self::ViewModel;

    /// Whether the Presenter takes `event` now. A key it refuses falls through to the Client's
    /// next step rather than being swallowed (Edit with no row to act on), so the default takes
    /// everything.
    fn takes(&self, _event: &Self::Event) -> bool {
        true
    }
}

/// A page's Presenter: it owns its keymap, so the host turns a chord or a palette Command into an
/// event through [`Self::keymap`] and holds no copy, and it hears how each Dialog it opened
/// closed through the event [`Self::dialog_closed`] builds. Its events are `Clone`, since a keymap
/// hands out a copy of the event a chord is bound to.
pub trait PagePresenter: Presenter<Event: Clone> {
    /// What a Dialog the page opened lands with once confirmed. A Dialog of another module maps
    /// its own outcome into this one when the page's Router opens it.
    type Outcome;

    /// The event telling the page its open Dialog closed: with what its confirm did, or `None`
    /// when it was cancelled. `None` for a page that opens no Dialog, which has nothing to be told.
    fn dialog_closed(outcome: Option<Self::Outcome>) -> Option<Self::Event>;

    /// The page's keys under `[keybindings]` for its current state (the tab shown, an open
    /// select), which its legend reads its chords from too. A palette Command of another state
    /// still reaches its event by name, shown first through [`Self::shows`].
    fn keymap(&self) -> &Keymap<Self::Event>;

    /// `[keybindings]` changed: the page builds everything key-shaped it holds afresh under
    /// `keys` (each keymap, and the chords its legend and Dialogs show). Required, so a new page
    /// can't keep its old keys after a rebind.
    fn rebind(&mut self, keys: &KeyBindingConfig);

    /// The Display Preferences changed, or a Ledger opened: a page that draws with them keeps
    /// what it reads. The rest draw nothing they choose, so the default ignores them.
    fn display(&mut self, _display: &PageDisplay) {}

    /// The event that shows the tab or mode `event` belongs to, when it isn't showing: a Client
    /// sends it ahead of a palette Command's own event, in the same input. A key never sends it,
    /// so a key the current tab or mode ignores stays ignored. `None` for an event the state
    /// showing takes.
    fn shows(&self, _event: &Self::Event) -> Option<Self::Event> {
        None
    }

    /// A key typed while the page is in a mode of its own ([`Self::mode`]), which its keymap
    /// didn't bind: the event it types, or `None` for one the mode gives no meaning to. A Client
    /// never offers it a key in Normal mode.
    fn typed(&self, _key: TextKey) -> Option<Self::Event> {
        None
    }

    /// The mode the page is in for something of its own (an edit in place, its Search, an open
    /// select), `None` in Normal mode. While it lasts, the Client offers the page every key first:
    /// to its keymap, then to [`Self::typed`]; a key neither takes goes nowhere else.
    fn mode(&self) -> Option<PageMode> {
        None
    }
}

/// A key typed into a page's mode, free of any toolkit: a character, or one of the keys a text
/// edit reads. A Client turns its own keystrokes into these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKey {
    /// A printable character, typed without Ctrl.
    Char(char),
    Backspace,
    Enter {
        shift: bool,
    },
    Tab {
        shift: bool,
    },
}

/// A mode a page enters for something of its own, which the Client hands the page's keys to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMode {
    /// An edit in place (a Budgets Plan cell).
    Insert,
    /// The page's Search box.
    Search,
    /// A filter popover or an open select's list.
    Filter,
}

/// A Dialog's Presenter: it closes through the one [`DialogRouter`] every module's Dialogs share,
/// and builds the events a host sends it for a Dialog input, a confirm and a cancel. A Dialog
/// confirms only through [`Self::confirm_event`] handed to [`Presenter::handle`], so a key and a
/// click confirm the same way and no Presenter has a `confirm(router)` beside `handle`.
pub trait DialogPresenter:
    for<'r> Presenter<Router<'r> = dyn DialogRouter<<Self as DialogPresenter>::Outcome> + 'r>
{
    /// What a confirm hands the page that opened the Dialog.
    type Outcome;

    /// The event for a Dialog keymap command or typed key (`lib_viper::dialog`).
    fn input_event(input: DialogInput) -> Self::Event;

    /// The event a confirm button sends, which saves a valid form and closes the Dialog; a
    /// refusal stays on the form.
    fn confirm_event() -> Self::Event;

    /// The event `back` and a cancel button send, which closes the Dialog having written nothing.
    fn cancel_event() -> Self::Event;

    /// Whether the form would confirm now, which enables its confirm button.
    fn can_confirm(&self) -> bool;

    /// Closes an open select's list, for `back` to do before it cancels. Returns whether one was
    /// open; a Dialog without a select never has one.
    fn close_open_select(&mut self) -> bool {
        false
    }

    /// Whether the Dialog steps a list right now, so the host sends the Dialog keymap's list step
    /// (`j`/`k` by default) as a select step rather than as typing.
    fn steps_list(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use lib_toast::{Notice, Notify};

    use super::*;
    use crate::keymap::{Binding, EDIT, NEW};

    /// A toy page: a count it bumps on `New`, and an Edit it takes only once there is a count.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum PageEvent {
        New,
        Edit,
        Closed(Option<&'static str>),
    }

    trait ToyRouter: Notify {
        fn open_dialog(&mut self) -> bool;
    }

    struct ToyPage {
        count: usize,
        landed: Vec<Option<&'static str>>,
        keymap: Keymap<PageEvent>,
    }

    impl Presenter for ToyPage {
        type Event = PageEvent;
        type ViewModel = usize;
        type Router<'r> = dyn ToyRouter + 'r;

        fn handle(&mut self, event: PageEvent, router: &mut Self::Router<'_>) {
            match event {
                PageEvent::New => {
                    self.count += 1;
                    router.open_dialog();
                }
                PageEvent::Edit => {}
                PageEvent::Closed(outcome) => self.landed.push(outcome),
            }
        }

        fn view_model(&self) -> usize {
            self.count
        }

        fn takes(&self, event: &PageEvent) -> bool {
            *event != PageEvent::Edit || self.count > 0
        }
    }

    impl PagePresenter for ToyPage {
        type Outcome = &'static str;

        fn dialog_closed(outcome: Option<&'static str>) -> Option<PageEvent> {
            Some(PageEvent::Closed(outcome))
        }

        fn keymap(&self) -> &Keymap<PageEvent> {
            &self.keymap
        }

        fn rebind(&mut self, keys: &KeyBindingConfig) {
            self.keymap = toy_keymap(keys);
        }
    }

    /// A toy Dialog: one text field that confirms once it holds something.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum DialogEvent {
        Input(DialogInput),
        Confirm,
        Cancel,
    }

    #[derive(Default)]
    struct ToyDialog {
        text: String,
    }

    impl Presenter for ToyDialog {
        type Event = DialogEvent;
        type ViewModel = String;
        type Router<'r> = dyn DialogRouter<&'static str> + 'r;

        fn handle(&mut self, event: DialogEvent, router: &mut Self::Router<'_>) {
            match event {
                DialogEvent::Input(DialogInput::Char(ch)) => self.text.push(ch),
                DialogEvent::Input(DialogInput::Submit) | DialogEvent::Confirm => {
                    if self.can_confirm() {
                        router.close(Some("saved"));
                    }
                }
                DialogEvent::Input(_) => {}
                DialogEvent::Cancel => router.close(None),
            }
        }

        fn view_model(&self) -> String {
            self.text.clone()
        }
    }

    impl DialogPresenter for ToyDialog {
        type Outcome = &'static str;

        fn input_event(input: DialogInput) -> DialogEvent {
            DialogEvent::Input(input)
        }

        fn confirm_event() -> DialogEvent {
            DialogEvent::Confirm
        }

        fn cancel_event() -> DialogEvent {
            DialogEvent::Cancel
        }

        fn can_confirm(&self) -> bool {
            !self.text.is_empty()
        }
    }

    /// A Router borrowing the log it writes to, as a Client's borrows its call's context.
    struct Borrowing<'a> {
        log: &'a mut Vec<String>,
    }

    impl Notify for Borrowing<'_> {
        fn notify(&mut self, notice: Notice) {
            self.log.push(format!("{notice:?}"));
        }
    }

    impl ToyRouter for Borrowing<'_> {
        fn open_dialog(&mut self) -> bool {
            self.log.push("open".to_string());
            true
        }
    }

    impl DialogRouter<&'static str> for Borrowing<'_> {
        fn close(&mut self, outcome: Option<&'static str>) {
            self.log.push(format!("close {outcome:?}"));
        }
    }

    /// What a generic host does with any page: a chord through the Presenter's own keymap, sent
    /// only when the Presenter takes it.
    fn press<P: PagePresenter>(page: &mut P, chord: &str, router: &mut P::Router<'_>) -> bool {
        let Some(event) = page.keymap().event(chord) else {
            return false;
        };
        if !page.takes(&event) {
            return false;
        }
        page.handle(event, router);
        true
    }

    /// What a generic host does with any Dialog: an input, then a confirm only while it can.
    fn confirm<P: DialogPresenter>(dialog: &mut P, router: &mut P::Router<'_>) {
        if dialog.can_confirm() {
            dialog.handle(P::confirm_event(), router);
        }
    }

    fn toy_keymap(keys: &KeyBindingConfig) -> Keymap<PageEvent> {
        const BINDINGS: &[Binding<PageEvent>] = &[
            Binding::hidden(NEW, "n", PageEvent::New),
            Binding::hidden(EDIT, "e", PageEvent::Edit),
        ];
        Keymap::new(BINDINGS, keys)
    }

    fn page() -> ToyPage {
        ToyPage {
            count: 0,
            landed: Vec::new(),
            keymap: toy_keymap(&KeyBindingConfig::default()),
        }
    }

    #[test]
    fn a_host_rebinds_a_page_through_the_trait_and_its_keymap_follows() {
        let mut log = Vec::new();
        let mut page = page();
        let mut keys = KeyBindingConfig::default();
        keys.bindings.insert(NEW.to_string(), "a".to_string());

        page.rebind(&keys);

        assert!(!press(&mut page, "n", &mut Borrowing { log: &mut log }));
        assert!(press(&mut page, "a", &mut Borrowing { log: &mut log }));
    }

    #[test]
    fn a_page_that_draws_no_display_preference_ignores_one() {
        let mut page = page();

        page.display(&PageDisplay::default());

        assert_eq!(page.view_model(), 0);
    }

    #[test]
    fn a_host_reaches_a_page_through_its_own_keymap_and_takes() {
        let mut log = Vec::new();
        let mut page = page();

        assert!(!press(&mut page, "e", &mut Borrowing { log: &mut log }));
        assert!(press(&mut page, "n", &mut Borrowing { log: &mut log }));
        assert!(press(&mut page, "e", &mut Borrowing { log: &mut log }));
        assert!(!press(&mut page, "q", &mut Borrowing { log: &mut log }));

        assert_eq!(page.view_model(), 1);
        assert_eq!(log, ["open"]);
    }

    #[test]
    fn a_closed_dialog_reaches_its_page_as_the_event_the_page_builds() {
        let mut log = Vec::new();
        let mut page = page();

        page.handle(
            ToyPage::dialog_closed(Some("saved")).expect("the toy page opens Dialogs"),
            &mut Borrowing { log: &mut log },
        );
        page.handle(
            ToyPage::dialog_closed(None).expect("the toy page opens Dialogs"),
            &mut Borrowing { log: &mut log },
        );

        assert_eq!(page.landed, [Some("saved"), None]);
    }

    #[test]
    fn a_dialog_confirms_only_through_handle_and_only_once_it_can() {
        let mut log = Vec::new();
        let mut dialog = ToyDialog::default();

        confirm(&mut dialog, &mut Borrowing { log: &mut log });
        assert!(log.is_empty(), "an empty form doesn't confirm");
        dialog.handle(
            ToyDialog::input_event(DialogInput::Char('x')),
            &mut Borrowing { log: &mut log },
        );
        confirm(&mut dialog, &mut Borrowing { log: &mut log });
        dialog.handle(ToyDialog::cancel_event(), &mut Borrowing { log: &mut log });

        assert_eq!(dialog.view_model(), "x");
        assert_eq!(log, ["close Some(\"saved\")", "close None"]);
    }

    #[test]
    fn a_page_without_a_mode_types_nothing() {
        let page = page();

        assert_eq!(page.mode(), None);
        assert_eq!(page.typed(TextKey::Char('x')), None);
        assert_eq!(page.typed(TextKey::Enter { shift: false }), None);
    }

    #[test]
    fn a_dialog_without_a_select_has_none_to_close_and_steps_no_list() {
        let mut dialog = ToyDialog::default();

        assert!(!dialog.close_open_select());
        assert!(!dialog.steps_list());
        assert!(dialog.takes(&DialogEvent::Cancel));
    }
}
