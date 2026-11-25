//! The Remove all API keys Dialog's Presenter (#1392): a destructive confirm with nothing to fill
//! in, since every saved Price Source API key is deleted from the Ledger, on every device that
//! syncs it (ADR-0043). `Enter` or Confirm deletes them through [`ApiKeys`] and raises the Toast;
//! `Esc` or Cancel keeps them. The Client hosts it as a module Dialog.

use std::rc::Rc;

use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::Hint;

use crate::interactor::RemoveApiKeys;
use crate::msg;
use crate::protocols::ApiKeys;

/// What a confirmed Remove all API keys did, for the page that opened it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveApiKeysOutcome {
    /// Every key row was deleted.
    Removed,
}

/// What the Remove all API keys Dialog takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveApiKeysEvent {
    /// A Dialog command or typed key: `Enter` confirms, and the Dialog has no field to type into.
    Input(DialogInput),
    /// The confirm button.
    Confirm,
    Cancel,
}

/// The Dialog as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveApiKeysViewModel {
    pub title: String,
    pub warning: String,
    /// The confirm button's label.
    pub confirm: String,
    /// The status line's legend while the Dialog is open.
    pub hints: Vec<Hint>,
}

/// The Remove all API keys Dialog's Presenter.
pub struct RemoveApiKeysPresenter {
    keys: Rc<dyn ApiKeys>,
    dialog_keys: DialogKeys,
}

impl RemoveApiKeysPresenter {
    /// The Dialog over `keys`. `dialog_keys` are the Dialog keymap's chords, which its hints show.
    pub fn new(keys: Rc<dyn ApiKeys>, dialog_keys: DialogKeys) -> Self {
        Self { keys, dialog_keys }
    }

    /// Deletes every key, says so and closes. Nothing can refuse it.
    fn remove(&mut self, router: &mut dyn DialogRouter<RemoveApiKeysOutcome>) {
        RemoveApiKeys.run(&*self.keys);
        router.notify(Notice::Toast(
            ToastKind::Info,
            msg::settings_backup_remove_keys_toast(),
        ));
        router.close(Some(RemoveApiKeysOutcome::Removed));
    }
}

impl Presenter for RemoveApiKeysPresenter {
    type Event = RemoveApiKeysEvent;
    type ViewModel = RemoveApiKeysViewModel;
    type Router<'r> = dyn DialogRouter<RemoveApiKeysOutcome> + 'r;

    /// `Enter` confirms; every other input is swallowed. `Esc`, the Shell's `back`, keeps the keys.
    fn handle(&mut self, event: RemoveApiKeysEvent, router: &mut Self::Router<'_>) {
        match event {
            RemoveApiKeysEvent::Input(DialogInput::Select | DialogInput::Submit)
            | RemoveApiKeysEvent::Confirm => self.remove(router),
            RemoveApiKeysEvent::Input(_) => {}
            RemoveApiKeysEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> RemoveApiKeysViewModel {
        RemoveApiKeysViewModel {
            title: msg::settings_backup_remove_keys_title(),
            warning: msg::settings_backup_remove_keys_warning(),
            confirm: msg::settings_backup_remove_keys(),
            hints: vec![
                self.dialog_keys
                    .hint(BACK, lib_locale::msg::dialog_hint_cancel()),
                self.dialog_keys
                    .hint(SELECT, lib_locale::msg::dialog_hint_confirm()),
            ],
        }
    }
}

impl DialogPresenter for RemoveApiKeysPresenter {
    type Outcome = RemoveApiKeysOutcome;

    fn input_event(input: DialogInput) -> RemoveApiKeysEvent {
        RemoveApiKeysEvent::Input(input)
    }

    fn confirm_event() -> RemoveApiKeysEvent {
        RemoveApiKeysEvent::Confirm
    }

    fn cancel_event() -> RemoveApiKeysEvent {
        RemoveApiKeysEvent::Cancel
    }

    fn can_confirm(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use lib_toast::Notify;

    use super::*;
    use crate::fake::InMemoryApiKeys;

    #[derive(Default)]
    struct Recording {
        closed: Vec<Option<RemoveApiKeysOutcome>>,
        notices: Vec<Notice>,
    }

    impl Notify for Recording {
        fn notify(&mut self, notice: Notice) {
            self.notices.push(notice);
        }
    }

    impl DialogRouter<RemoveApiKeysOutcome> for Recording {
        fn close(&mut self, outcome: Option<RemoveApiKeysOutcome>) {
            self.closed.push(outcome);
        }
    }

    fn presenter() -> (RemoveApiKeysPresenter, Rc<InMemoryApiKeys>) {
        let keys = Rc::new(InMemoryApiKeys::new(Some("3F9A-21C0"), 2));
        (
            RemoveApiKeysPresenter::new(keys.clone(), DialogKeys::default()),
            keys,
        )
    }

    #[test]
    fn enter_removes_every_key_raises_an_info_toast_and_closes() {
        let (mut presenter, keys) = presenter();
        let mut router = Recording::default();

        presenter.handle(RemoveApiKeysEvent::Input(DialogInput::Select), &mut router);

        assert_eq!(keys.saved(), 0);
        assert_eq!(router.closed, [Some(RemoveApiKeysOutcome::Removed)]);
        assert_eq!(
            router.notices,
            [Notice::Toast(
                ToastKind::Info,
                "API keys removed".to_string()
            )]
        );
    }

    #[test]
    fn cancel_keeps_the_keys() {
        let (mut presenter, keys) = presenter();
        let mut router = Recording::default();

        presenter.handle(RemoveApiKeysEvent::Cancel, &mut router);

        assert_eq!(keys.saved(), 2);
        assert_eq!(router.closed, [None]);
        assert!(router.notices.is_empty());
    }

    #[test]
    fn typing_and_tab_are_swallowed() {
        let (mut presenter, keys) = presenter();
        let mut router = Recording::default();

        for input in [DialogInput::Char('x'), DialogInput::NextField] {
            presenter.handle(RemoveApiKeysEvent::Input(input), &mut router);
        }

        assert!(router.closed.is_empty());
        assert_eq!(keys.saved(), 2);
    }
}
