//! What a module Dialog's Presenter takes and closes through: a [`DialogInput`], the Dialog
//! keymap command or typed key the Client resolved, and a [`DialogRouter`], the one Router every
//! module's Dialogs close through (#988).
//!
//! The Dialog keymap itself is `lib-shell`'s (`lib_shell::dialog`), which only the bins may
//! depend on, so a feature crate names the commands here and the Client turns each into its input.
//! `back` isn't an input: the host decides it, closing an open select first, otherwise cancelling
//! the Dialog.

use lib_toast::Notify;

/// One input a module Dialog's Presenter takes: a Dialog keymap command other than `back`, or a key
/// typed into the focused field. What it means is the Presenter's call, from its focused field and
/// whether its select is open: `Select` opens a closed select, picks from an open one, and
/// otherwise confirms only a valid form; `Submit` confirms a valid form from any field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogInput {
    Select,
    NextField,
    PreviousField,
    SelectNext,
    SelectPrevious,
    Submit,
    /// `←` and `→`, which no Dialog command binds: a field that steps along a row (the Tag form's
    /// swatches) reads them, and the others ignore them.
    Left,
    Right,
    /// A printable character, typed without a modifier.
    Char(char),
    Backspace,
}

/// Where a module Dialog's Presenter asks to go: closing its Dialog, and Notices. A Dialog's Router
/// opens nothing, so this is the whole of it; what the page opens next is in the outcome. Generic
/// over the outcome `O`, so the Client implements it once for every module's Dialogs.
pub trait DialogRouter<O>: Notify {
    /// Closes the Dialog. `outcome` is what its confirm did, `None` when it was cancelled; the
    /// Client hands a confirmed one to the page that opened the Dialog.
    fn close(&mut self, outcome: Option<O>);
}

#[cfg(test)]
mod tests {
    use lib_toast::Notice;

    use super::*;

    /// A Router recording what it was asked, as a Client's would act on it.
    #[derive(Default)]
    struct Recording {
        closed: Vec<Option<&'static str>>,
        notices: Vec<Notice>,
    }

    impl DialogRouter<&'static str> for Recording {
        fn close(&mut self, outcome: Option<&'static str>) {
            self.closed.push(outcome);
        }
    }

    impl Notify for Recording {
        fn notify(&mut self, notice: Notice) {
            self.notices.push(notice);
        }
    }

    /// A Presenter that confirms on `Submit` and cancels on anything else, through any module's
    /// Router.
    fn presenter(input: DialogInput, router: &mut dyn DialogRouter<&'static str>) {
        if input == DialogInput::Submit {
            router.notify(Notice::Status("saved".to_string()));
            router.close(Some("saved"));
        } else {
            router.close(None);
        }
    }

    #[test]
    fn a_presenter_closes_with_its_outcome_and_notifies_through_one_router() {
        let mut router = Recording::default();

        presenter(DialogInput::Submit, &mut router);
        presenter(DialogInput::Char('x'), &mut router);

        assert_eq!(router.closed, [Some("saved"), None]);
        assert_eq!(router.notices, [Notice::Status("saved".to_string())]);
    }
}
