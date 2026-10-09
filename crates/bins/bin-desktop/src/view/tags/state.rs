//! The Tags destination's Entities (ADR-0032). [`TagsStore`] owns the rows, read through
//! `lib_tags`' service; [`TagsView`] owns the two selections (the Tags page's usage-order row and
//! the Settings list's id). `Shell` opens the dialogs and runs the rules, since the dialog host
//! and the Transactions store are `Shell`'s.

use gpui::{Context, Entity};
use lib_tags::{Tag, TagService};

/// The shared Tags rows. Every reader (the Tags page, Settings, the Transactions chips and filter
/// form, and the Transactions Split tag picker) reads them from here, so a change is seen
/// everywhere on the next render.
pub struct TagsStore {
    service: TagService,
}

impl TagsStore {
    /// A store over `tags`, which the Desktop seeds from `lib_tags::default_tags`.
    pub fn new(tags: Vec<Tag>) -> Self {
        Self {
            service: TagService::new(tags),
        }
    }

    pub fn tags(&self) -> &[Tag] {
        self.service.tags()
    }

    /// Applies `change` to the rows, then tells every subscriber they changed. Mutations go
    /// through here so no write can skip the notification.
    pub fn mutate<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut Vec<Tag>) -> R,
    ) -> R {
        let result = change(self.service.tags_mut());
        cx.notify();
        result
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
