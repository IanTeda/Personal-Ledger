//! The Tags Settings page (`docs/ux/desktop-mockups/16-settings/README.md`'s 2j): every Tag A–Z,
//! ignoring case, with its colour swatch and `edit · remove` per row. A likely duplicate (a Tag the
//! uniqueness rule would have refused, from data that broke it) carries its badge and a direct
//! **merge**. Usage lives in Transactions and Reports, so there are no transaction, total or
//! last-used columns.
//!
//! The selection is a Tag id, so it follows its row across a rename. It may name a Tag that has
//! since gone, so it is resolved on every read: a removed or merged-away Tag leaves the first row
//! selected, until a merge lands on its target.

use lib_core::{HexColor, TagId};
use lib_list_cursor::{Step, resolve, step_id};
use lib_locale::Segment;
use lib_viper::keymap::{EDIT, Keymap, MOVE_DOWN, MOVE_UP, NEW, SELECT};

use super::count;
use crate::keymap::{MERGE, REMOVE};
use crate::protocols::{SharedPorts, TagChange, TagsDialog, TagsOutcome, TagsRouter};
use crate::{
    DuplicateGroup, Tag, active_count, duplicate_groups, duplicate_of, get, msg, sorted_by_name,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagsPageEvent {
    /// `j`/`k`, `g g`, `G`, `Ctrl-d`/`Ctrl-u` and their arrow, Home/End and Page keys.
    Move(Step),
    /// A click on a row.
    Select(TagId),
    /// Shows the selected row's Transactions.
    OpenTransactions,
    /// New opens Add; Edit, Remove and Merge act on the selected Tag.
    New,
    Edit,
    Remove,
    Merge,
    /// A row's own **edit**, **remove** or **merge**: selects the row, then opens its Dialog.
    EditRow(TagId),
    RemoveRow(TagId),
    MergeRow(TagId),
    /// The open Dialog closed, with what its confirm did or `None` when cancelled.
    DialogClosed(Option<TagsOutcome>),
}

/// The page as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagsPageViewModel {
    /// The heading's meta: `18 tags · 2 likely duplicates`, the duplicates' number tagged `strong`.
    pub scope: Vec<Segment>,
    /// The same, plain, for the status line.
    pub status: String,
    pub kicker: String,
    pub add_button: String,
    pub column_tag: String,
    pub column_actions: String,
    /// A–Z, ignoring case.
    pub rows: Vec<TagLine>,
    /// What shows in place of the table while there are no Tags.
    pub empty: Option<String>,
    pub footnote: String,
    /// The tag beside an inactive Tag's name.
    pub inactive: String,
    pub row_merge: String,
    pub row_edit: String,
    pub row_remove: String,
    pub selected: Option<TagId>,
    pub hints: Vec<Hint>,
    /// The Dialog the page last opened, until it closes.
    pub open_dialog: Option<TagsDialog>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagLine {
    pub id: TagId,
    pub name: String,
    /// `None` shows no swatch.
    pub color: Option<HexColor>,
    /// Drawn dimmed and tagged, rather than hidden.
    pub is_active: bool,
    /// `looks like a duplicate of "work-trip"`, for a flagged Tag, which also offers **merge**.
    pub duplicate_of: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub chord: String,
    pub action: String,
}

pub struct TagsPagePresenter {
    ports: SharedPorts,
    keymap: Keymap<TagsPageEvent>,
    selected: Option<TagId>,
    open_dialog: Option<TagsDialog>,
}

impl TagsPagePresenter {
    /// `keymap` is the one the Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(ports: SharedPorts, keymap: Keymap<TagsPageEvent>) -> Self {
        Self {
            ports,
            keymap,
            selected: None,
            open_dialog: None,
        }
    }

    /// Whether the page takes `event` now: Edit, Remove and Merge act on the selected Tag, so with
    /// no Tag to act on the page refuses them rather than ignoring them.
    pub fn takes(&self, event: &TagsPageEvent) -> bool {
        !matches!(
            event,
            TagsPageEvent::Edit | TagsPageEvent::Remove | TagsPageEvent::Merge
        ) || self.selected().is_some()
    }

    pub fn handle(&mut self, event: TagsPageEvent, router: &mut dyn TagsRouter) {
        match event {
            TagsPageEvent::Move(step) => {
                self.selected = step_id(&self.ids(), self.selected(), step)
            }
            TagsPageEvent::Select(id) => self.select(id),
            TagsPageEvent::OpenTransactions => {
                if let Some(id) = self.selected() {
                    router.open_transactions(id);
                }
            }
            TagsPageEvent::New => self.open(TagsDialog::AddTag, router),
            TagsPageEvent::Edit => {
                if let Some(id) = self.selected() {
                    self.open(TagsDialog::EditTag(id), router);
                }
            }
            TagsPageEvent::Remove => {
                if let Some(id) = self.selected() {
                    self.open(TagsDialog::RemoveTag(id), router);
                }
            }
            TagsPageEvent::Merge => {
                if let Some(id) = self.selected() {
                    self.open_merge(Some(id), router);
                }
            }
            TagsPageEvent::EditRow(id) => {
                self.select(id);
                self.open(TagsDialog::EditTag(id), router);
            }
            TagsPageEvent::RemoveRow(id) => {
                self.select(id);
                self.open(TagsDialog::RemoveTag(id), router);
            }
            TagsPageEvent::MergeRow(id) => {
                self.select(id);
                self.open_merge(Some(id), router);
            }
            TagsPageEvent::DialogClosed(outcome) => {
                self.open_dialog = None;
                // A save selects its Tag and a merge its target; a remove's selection falls to
                // the first row, and its Toast is its Dialog's.
                if let Some(TagsOutcome {
                    change: TagChange::Saved | TagChange::Merged,
                    id,
                    ..
                }) = outcome
                {
                    self.select(id);
                }
            }
        }
    }

    pub fn view_model(&self) -> TagsPageViewModel {
        let ports = self.ports.borrow();
        let tags = ports.tags();
        let groups = duplicate_groups(&tags, &*ports);
        let rows: Vec<TagLine> = sorted_by_name(&tags)
            .into_iter()
            .map(|tag| TagLine {
                id: tag.id,
                name: tag.name.clone(),
                color: tag.color.clone(),
                is_active: tag.is_active,
                duplicate_of: duplicate_of(&groups, tag.id)
                    .and_then(|target| get(&tags, target))
                    .map(|target| msg::tags_duplicate_of(&target.name)),
            })
            .collect();
        let selected = resolve(rows.iter().map(|row| row.id), self.selected);
        let (scope, status) = scope(&tags, &groups);
        TagsPageViewModel {
            scope,
            status,
            kicker: msg::tags_kicker(),
            add_button: msg::tags_add_button("+"),
            column_tag: msg::tags_column_tag(),
            column_actions: lib_locale::msg::column_actions(),
            empty: rows.is_empty().then(|| msg::tags_empty(self.chord(NEW))),
            rows,
            footnote: msg::tags_footnote(),
            inactive: msg::tags_inactive(),
            row_merge: msg::tags_row_merge(),
            row_edit: msg::tags_row_edit(),
            row_remove: msg::tags_row_remove(),
            selected,
            hints: self.hints(),
            open_dialog: self.open_dialog,
        }
    }

    /// The Tag ids A–Z, as the rows run.
    fn ids(&self) -> Vec<TagId> {
        let tags = self.ports.borrow().tags();
        sorted_by_name(&tags)
            .into_iter()
            .map(|tag: &Tag| tag.id)
            .collect()
    }

    /// The selected Tag: the stored id while it still exists, else the first row.
    fn selected(&self) -> Option<TagId> {
        resolve(self.ids(), self.selected)
    }

    /// Selects Tag `id`, if it still exists.
    fn select(&mut self, id: TagId) {
        if self.ids().contains(&id) {
            self.selected = Some(id);
        }
    }

    /// Opens the Merge Dialog with `source` as the source Tag and, when it is flagged as a likely
    /// duplicate, its suggested target (#354).
    fn open_merge(&mut self, source: Option<TagId>, router: &mut dyn TagsRouter) {
        let target = source.and_then(|id| {
            let ports = self.ports.borrow();
            let groups = duplicate_groups(&ports.tags(), &*ports);
            duplicate_of(&groups, id)
        });
        self.open(TagsDialog::MergeTags { source, target }, router);
    }

    fn open(&mut self, dialog: TagsDialog, router: &mut dyn TagsRouter) {
        if router.open_dialog(dialog) {
            self.open_dialog = Some(dialog);
        }
    }

    fn chord(&self, command: &str) -> &str {
        self.keymap.chord(command).unwrap_or_default()
    }

    fn hints(&self) -> Vec<Hint> {
        let hint = |chord: String, action: String| Hint { chord, action };
        vec![
            hint(
                format!("{}/{}", self.chord(MOVE_DOWN), self.chord(MOVE_UP)),
                msg::tags_hint_row(),
            ),
            hint(
                self.chord(SELECT).to_string(),
                msg::tags_hint_open_transactions(),
            ),
            hint(self.chord(EDIT).to_string(), msg::tags_hint_edit()),
            hint(self.chord(REMOVE).to_string(), msg::tags_hint_remove()),
            hint(self.chord(MERGE).to_string(), msg::tags_hint_merge()),
            hint(self.chord(NEW).to_string(), msg::tags_hint_new()),
        ]
    }
}

/// The heading's meta and its plain form: the active count, then the likely duplicates when any.
fn scope(tags: &[Tag], groups: &[DuplicateGroup]) -> (Vec<Segment>, String) {
    let plain = |text: String| Segment {
        text,
        tag: None,
        token: None,
    };
    let total = msg::tags_count(count(active_count(tags)));
    let duplicates = groups.iter().map(|group| group.duplicates.len()).sum();
    if duplicates == 0 {
        return (vec![plain(total.clone())], total);
    }
    let mut scope = vec![plain(total.clone()), plain(" \u{b7} ".to_string())];
    scope.extend(msg::tags_likely_duplicates(count(duplicates)));
    let status = format!(
        "{total} \u{b7} {}",
        msg::tags_status_duplicates(count(duplicates))
    );
    (scope, status)
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;

    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::keymap::tags_page_keymap;
    use crate::protocols::TagUse;

    struct Page {
        presenter: TagsPagePresenter,
        router: RecordingRouter,
    }

    impl Page {
        fn new(shared: SharedPorts) -> Self {
            Self::with_keys(shared, &KeyBindingConfig::default())
        }

        fn with_keys(shared: SharedPorts, config: &KeyBindingConfig) -> Self {
            Self {
                presenter: TagsPagePresenter::new(shared, tags_page_keymap(config)),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: TagsPageEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> TagsPageViewModel {
            self.presenter.view_model()
        }

        fn selected_name(&self) -> Option<String> {
            let view = self.view();
            view.rows
                .into_iter()
                .find(|row| Some(row.id) == view.selected)
                .map(|row| row.name)
        }
    }

    const SHARED: TagId = TagId::seed(2);
    const WORK_TRIP: TagId = TagId::seed(5);
    const WORK_TRIP_DUPLICATE: TagId = TagId::seed(9);

    fn plain(segments: &[Segment]) -> String {
        segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect()
    }

    #[test]
    fn open_transactions_asks_the_router_for_the_selected_rows_transactions() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        page.send(TagsPageEvent::Move(Step::Last));
        let selected = page.presenter.selected().expect("a stub row is selected");
        assert_eq!(
            page.send(TagsPageEvent::OpenTransactions),
            [RouterCall::OpenTransactions(selected)]
        );
    }

    #[test]
    fn the_rows_run_a_to_z_with_the_flagged_duplicate_naming_its_target() {
        let (_, shared) = stub();
        let view = Page::new(shared).view();

        let names: Vec<&str> = view.rows.iter().map(|row| row.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_by_key(|name| name.to_lowercase());
        assert_eq!(names, sorted);
        let flagged: Vec<(&str, &str)> = view
            .rows
            .iter()
            .filter_map(|row| Some((row.name.as_str(), row.duplicate_of.as_deref()?)))
            .collect();
        assert_eq!(
            flagged,
            [("Work Trip", "looks like a duplicate of \"work-trip\"")]
        );
        assert_eq!(view.selected, view.rows.first().map(|row| row.id));
        assert_eq!(view.empty, None);
    }

    #[test]
    fn the_scope_counts_active_tags_and_flags_the_duplicates() {
        let (_, shared) = stub();
        let view = Page::new(shared).view();

        assert_eq!(view.status, "8 tags \u{b7} 1 likely duplicate");
        assert_eq!(plain(&view.scope), "8 tags \u{b7} 1 likely duplicate");
        let strong: Vec<&str> = view
            .scope
            .iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(strong, ["1"]);
    }

    #[test]
    fn with_nothing_flagged_the_scope_is_the_count_alone() {
        let (ports, shared) = stub();
        ports
            .borrow_mut()
            .tags
            .tags
            .retain(|tag| tag.id != WORK_TRIP_DUPLICATE);

        let view = Page::new(shared).view();

        assert_eq!(view.status, "7 tags");
        assert_eq!(plain(&view.scope), "7 tags");
    }

    #[test]
    fn the_empty_page_names_the_key_that_adds() {
        let (ports, shared) = stub();
        ports.borrow_mut().tags.tags.clear();

        let view = Page::new(shared).view();

        assert_eq!(
            view.empty.as_deref(),
            Some("No tags yet. Press n to add one.")
        );
        assert_eq!(view.selected, None);
    }

    #[test]
    fn movement_walks_the_rows_and_stops_at_the_ends() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        let names: Vec<String> = page.view().rows.into_iter().map(|row| row.name).collect();

        assert_eq!(page.selected_name().as_ref(), names.first());
        page.send(TagsPageEvent::Move(Step::Next));
        assert_eq!(page.selected_name().as_ref(), names.get(1));
        page.send(TagsPageEvent::Move(Step::Last));
        assert_eq!(page.selected_name().as_ref(), names.last());
        page.send(TagsPageEvent::Move(Step::Next));
        assert_eq!(page.selected_name().as_ref(), names.last());
        page.send(TagsPageEvent::Move(Step::First));
        assert_eq!(page.selected_name().as_ref(), names.first());
    }

    #[test]
    fn new_edit_and_remove_ask_the_router_for_their_dialogs() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        page.send(TagsPageEvent::Select(SHARED));

        assert_eq!(
            page.send(TagsPageEvent::New),
            [RouterCall::OpenDialog(TagsDialog::AddTag)]
        );
        assert_eq!(
            page.send(TagsPageEvent::Edit),
            [RouterCall::OpenDialog(TagsDialog::EditTag(SHARED))]
        );
        assert_eq!(
            page.send(TagsPageEvent::Remove),
            [RouterCall::OpenDialog(TagsDialog::RemoveTag(SHARED))]
        );
        assert_eq!(
            page.view().open_dialog,
            Some(TagsDialog::RemoveTag(SHARED)),
            "the last one asked for stays until it closes"
        );
    }

    #[test]
    fn a_row_action_selects_its_row_before_opening() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        let calls = page.send(TagsPageEvent::EditRow(SHARED));

        assert_eq!(calls, [RouterCall::OpenDialog(TagsDialog::EditTag(SHARED))]);
        assert_eq!(page.selected_name().as_deref(), Some("shared"));
        assert_eq!(
            page.send(TagsPageEvent::RemoveRow(WORK_TRIP)),
            [RouterCall::OpenDialog(TagsDialog::RemoveTag(WORK_TRIP))]
        );
        assert_eq!(page.selected_name().as_deref(), Some("work-trip"));
    }

    #[test]
    fn merge_on_a_flagged_tag_brings_its_target_and_on_any_other_only_the_source() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        assert_eq!(
            page.send(TagsPageEvent::MergeRow(WORK_TRIP_DUPLICATE)),
            [RouterCall::OpenDialog(TagsDialog::MergeTags {
                source: Some(WORK_TRIP_DUPLICATE),
                target: Some(WORK_TRIP),
            })]
        );
        page.send(TagsPageEvent::Select(SHARED));
        assert_eq!(
            page.send(TagsPageEvent::Merge),
            [RouterCall::OpenDialog(TagsDialog::MergeTags {
                source: Some(SHARED),
                target: None,
            })]
        );
    }

    #[test]
    fn the_suggested_target_follows_the_usage() {
        let (ports, shared) = stub();
        let mut page = Page::new(shared);
        // The more-used of the pair is the one to keep.
        ports.borrow_mut().usage.uses.insert(
            WORK_TRIP_DUPLICATE,
            TagUse {
                transactions: 9,
                ..TagUse::default()
            },
        );

        assert_eq!(
            page.send(TagsPageEvent::MergeRow(WORK_TRIP)),
            [RouterCall::OpenDialog(TagsDialog::MergeTags {
                source: Some(WORK_TRIP),
                target: Some(WORK_TRIP_DUPLICATE),
            })]
        );
    }

    #[test]
    fn a_dialog_that_cannot_open_leaves_the_page_without_one() {
        struct Refusing(RecordingRouter);
        impl lib_toast::Notify for Refusing {
            fn notify(&mut self, notice: lib_toast::Notice) {
                self.0.notify(notice);
            }
        }
        impl TagsRouter for Refusing {
            fn open_dialog(&mut self, _dialog: TagsDialog) -> bool {
                false
            }
            fn open_transactions(&mut self, id: TagId) {
                self.0.open_transactions(id);
            }
            fn close_dialog(&mut self, outcome: Option<TagsOutcome>) {
                self.0.close_dialog(outcome);
            }
        }
        let (_, shared) = stub();
        let mut presenter =
            TagsPagePresenter::new(shared, tags_page_keymap(&KeyBindingConfig::default()));

        presenter.handle(
            TagsPageEvent::New,
            &mut Refusing(RecordingRouter::default()),
        );

        assert_eq!(presenter.view_model().open_dialog, None);
    }

    #[test]
    fn a_saved_tag_is_selected_when_its_dialog_closes_and_a_remove_falls_to_the_first_row() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        let outcome = |change, id| {
            Some(TagsOutcome {
                change,
                id,
                name: String::new(),
            })
        };

        page.send(TagsPageEvent::New);
        page.send(TagsPageEvent::DialogClosed(outcome(
            TagChange::Saved,
            SHARED,
        )));
        assert_eq!(page.selected_name().as_deref(), Some("shared"));
        assert_eq!(page.view().open_dialog, None);
        page.send(TagsPageEvent::DialogClosed(outcome(
            TagChange::Merged,
            WORK_TRIP,
        )));
        assert_eq!(page.selected_name().as_deref(), Some("work-trip"));
        page.send(TagsPageEvent::DialogClosed(outcome(
            TagChange::Removed,
            WORK_TRIP,
        )));
        assert_eq!(page.selected_name().as_deref(), Some("work-trip"));
        page.send(TagsPageEvent::DialogClosed(None));
    }

    #[test]
    fn the_selection_falls_to_the_first_row_when_its_tag_goes() {
        let (ports, shared) = stub();
        let mut page = Page::new(shared);
        page.send(TagsPageEvent::Select(SHARED));
        ports.borrow_mut().tags.tags.retain(|tag| tag.id != SHARED);

        let view = page.view();

        assert_eq!(view.selected, view.rows.first().map(|row| row.id));
    }

    #[test]
    fn the_legend_follows_the_keymap() {
        let (_, shared) = stub();
        let hints = |config: &KeyBindingConfig| {
            Page::with_keys(shared.clone(), config)
                .view()
                .hints
                .into_iter()
                .map(|hint| (hint.chord, hint.action))
                .collect::<Vec<_>>()
        };

        let row = |chord: &str, action: &str| (chord.to_string(), action.to_string());
        assert_eq!(
            hints(&KeyBindingConfig::default()),
            [
                row("j/k", "row"),
                row("enter", "transactions"),
                row("e", "edit"),
                row("x", "remove"),
                row("m", "merge"),
                row("n", "new"),
            ]
        );
        let mut config = KeyBindingConfig::default();
        config.bindings.insert("tags.merge".into(), "u".into());
        config.bindings.insert("tags.remove".into(), "d".into());
        let rebound = hints(&config);
        assert_eq!(rebound[3], row("d", "remove"));
        assert_eq!(rebound[4], row("u", "merge"));
    }
}
