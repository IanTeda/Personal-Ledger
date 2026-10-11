//! The Tags destination's Entities (ADR-0032). [`TagsStore`] reads the rows through
//! [`LedgerStore`], which holds the only copy; [`TagsView`] owns the two selections (the Tags page's usage-order row and
//! the Settings list's id). `Shell` opens the dialogs; their writes are Ledger commands.

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_ledger::Domain;
use lib_tags::Tag;

use crate::ledger_store::{LedgerChanged, LedgerStore};

/// Emitted by [`TagsStore`] after any write, so `Shell` can refresh the Views that read the rows
/// (ADR-0032's View events).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagsEvent {
    Changed,
}

/// The shared Tags rows. Every reader (the Tags page, Settings, the Transactions chips and filter
/// form, and the Transactions Split tag picker) reads them from here, so a change is seen
/// everywhere on the next render.
pub struct TagsStore {
    ledger: Entity<LedgerStore>,
    _ledger_changed: Subscription,
}

impl EventEmitter<TagsEvent> for TagsStore {}

impl TagsStore {
    /// A store over `ledger`'s Tags.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Tags) {
                cx.emit(TagsEvent::Changed);
            }
        });
        Self {
            ledger,
            _ledger_changed,
        }
    }

    pub fn tags<'a>(&self, cx: &'a App) -> &'a [Tag] {
        self.ledger.read(cx).ledger().tags()
    }
}

/// The Tags page's own state. `selected` is a position in `lib_tags::sorted_by_usage`'s order;
/// `settings_selected` is the Tag id the Settings Tags list has under the cursor.
pub struct TagsView {
    store: Entity<TagsStore>,
    selected: usize,
    settings_selected: Option<u32>,
}

impl TagsView {
    pub fn new(store: Entity<TagsStore>) -> Self {
        Self {
            store,
            selected: 0,
            settings_selected: None,
        }
    }

    /// The Tags page's stored row position. Clamp it before use: removing Tags can leave it past
    /// the end.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The Settings Tags list's stored id. It may name a Tag that no longer exists, so read it
    /// through the Shell's selection rule.
    pub fn settings_selected(&self) -> Option<u32> {
        self.settings_selected
    }

    pub fn set_selected(&mut self, selected: usize, cx: &mut Context<'_, Self>) {
        self.selected = selected;
        cx.notify();
    }

    pub fn set_settings_selected(&mut self, id: Option<u32>, cx: &mut Context<'_, Self>) {
        self.settings_selected = id;
        cx.notify();
    }
}
