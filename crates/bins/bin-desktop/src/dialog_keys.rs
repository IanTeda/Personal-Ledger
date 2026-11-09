//! The Dialog keys an open Ledger's pages hand each Dialog's Presenter at open, shared so that a
//! rebind in Settings reaches the next Dialog opened: the hints a Dialog shows come from the keys
//! its Presenter was given, so a copy taken once at Ledger open would show the old chords.

use std::cell::RefCell;
use std::rc::Rc;

use lib_viper::dialog::DialogKeys;

/// One Ledger's Dialog keys, which every page's Router reads as it opens a Dialog and the Shell
/// replaces when `[keybindings]` change.
#[derive(Debug, Clone, Default)]
pub struct LiveDialogKeys(Rc<RefCell<DialogKeys>>);

impl LiveDialogKeys {
    pub fn new(keys: DialogKeys) -> Self {
        Self(Rc::new(RefCell::new(keys)))
    }

    /// The keys now, for a Dialog's Presenter about to open.
    pub fn get(&self) -> DialogKeys {
        self.0.borrow().clone()
    }

    /// Replaces the keys every page's next Dialog reads, after a rebind.
    pub fn set(&self, keys: DialogKeys) {
        *self.0.borrow_mut() = keys;
    }
}
