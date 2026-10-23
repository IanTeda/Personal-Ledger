//! The **Tags** Settings page (ADR-0037, and its amendment's one folder per page): the Tags table
//! with its likely duplicates and their merge. [`TagsPage`] reads its rows from [`LedgerStore`],
//! renders itself (`render.rs`), handles its keys and movement, builds its Dialogs (`dialogs/`)
//! and reaches `Shell` only through [`ViewEvent`]. The Dialog host holds them, and they draw,
//! read back and confirm themselves (`dialogs/`, `apply.rs`), handing each outcome back to
//! [`TagsPage::land`]. The Settings View forwards to it while the page is open.
//!
//! The rules live in `lib_tags`: a Tag's name, usage, merge and remove, and the likely duplicates
//! that broke the uniqueness rule (ADR-0015 and its amendments).

pub mod apply;
pub mod dialogs;
pub mod form;
mod render;

use crate::settings::SettingsPage;
use crate::settings::{
    page::PageRowClicked,
    pages::{Page, focus_on_row_click},
    state::SettingsView,
};
use gpui::AnyElement;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_core::TagId;
use lib_ledger::Domain;
use lib_tags::{DuplicateGroup, Tag};

use crate::{
    chrome::{
        dialog_host::open_dialog,
        statusline::{PageStatus, owned_hints},
    },
    ledger_store::{LedgerStore, notify_on},
    navigation::movement::Movement,
    settings::hints::settings_tags_hints,
    view::event::ViewEvent,
};
use apply::TagOutcome;
use dialogs::TagsDialog;
use form::{MergeTagsForm, RemoveTagForm, TagForm, merge_options};
use render::scope_text;

impl EventEmitter<ViewEvent> for TagsPage {}

impl EventEmitter<PageRowClicked> for TagsPage {}

/// The Tags page's own state: the Tag id under the cursor.
pub struct TagsPage {
    ledger: Entity<LedgerStore>,
    /// It may name a Tag that no longer exists, so read it through [`Self::selected_id`].
    selected: Option<TagId>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    pub(crate) page_focused: bool,
    /// Repaints the page when the Tags its rows read change.
    _tags_changed: Subscription,
    /// Repaints the page when a Transaction changes, since usage drives the duplicates and the
    /// merge counts.
    _transactions_changed: Subscription,
}

impl TagsPage {
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _tags_changed = notify_on(&ledger, Domain::Tags, cx);
        let _transactions_changed = notify_on(&ledger, Domain::Transactions, cx);
        Self {
            ledger,
            selected: None,
            page_focused: false,
            _tags_changed,
            _transactions_changed,
        }
    }

    pub(crate) fn tags<'a>(&self, cx: &'a App) -> &'a [Tag] {
        self.ledger.read(cx).ledger().tags()
    }

    fn transactions<'a>(&self, cx: &'a App) -> &'a [lib_transactions::Transaction] {
        self.ledger.read(cx).ledger().transactions()
    }

    /// The likely duplicates, which the rows and the heading flag.
    pub(crate) fn duplicate_groups(&self, cx: &App) -> Vec<DuplicateGroup> {
        lib_tags::duplicate_groups(self.tags(cx), self.transactions(cx))
    }
}

impl Page for TagsPage {
    const PAGE: SettingsPage = SettingsPage::Tags;

    /// The page's bare `n`, `e`, `x` and `m`, each through the same handler its button uses.
    /// Returns whether the key was one of them.
    fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_dialog(cx),
            "e" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_edit_dialog(id, cx);
                }
            }
            "x" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_remove_dialog(id, cx);
                }
            }
            "m" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_merge_dialog(Some(id), cx);
                }
            }
            _ => return false,
        }
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the Tags list A–Z. `enter` has no hand-off here.
    fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) -> bool {
        let Some(step) = movement.step() else {
            return true;
        };
        let ids: Vec<TagId> = lib_tags::sorted_by_name(self.tags(cx))
            .iter()
            .map(|tag| tag.id)
            .collect();
        self.selected = lib_list_cursor::step_id(&ids, self.selected_id(cx), step);
        cx.notify();
        true
    }

    /// The page's legend and its Tag counts.
    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: owned_hints(settings_tags_hints()),
            right: scope_text(self.tags(cx), &self.duplicate_groups(cx)),
        })
    }

    fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    fn hints(&self, _cx: &App) -> Vec<(&'static str, String)> {
        settings_tags_hints()
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        self.scope_note(cx)
    }

    fn subscribe_row_clicks(
        entity: &Entity<Self>,
        cx: &mut Context<'_, SettingsView>,
    ) -> Option<Subscription> {
        focus_on_row_click(entity, cx)
    }
}

impl TagsPage {
    // -----------------------------------------------------------------------------------------
    // Selection
    // -----------------------------------------------------------------------------------------

    /// The selected Tag's id: the stored id while it still exists, else the first row, so a
    /// removed or merged-away Tag never leaves the page with nothing under the cursor.
    pub fn selected_id(&self, cx: &App) -> Option<TagId> {
        let sorted = lib_tags::sorted_by_name(self.tags(cx));
        lib_list_cursor::resolve(sorted.iter().map(|tag| tag.id), self.selected)
    }

    /// Selects Tag `id`, if it still exists.
    pub fn select_id(&mut self, id: TagId, cx: &mut Context<'_, Self>) {
        if lib_tags::get(self.tags(cx), id).is_some() {
            self.selected = Some(id);
            cx.notify();
        }
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// A click on a row: selects it and moves focus into the page.
    pub(crate) fn click_row(&mut self, id: TagId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        cx.emit(PageRowClicked);
    }

    /// **+ Add tag**: the same handler `n` reaches.
    pub(crate) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_dialog(cx);
    }

    pub(crate) fn click_edit(&mut self, id: TagId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        self.open_edit_dialog(id, cx);
    }

    /// A flagged Tag's **merge** opens the merge dialog on it, with its suggested target chosen.
    pub(crate) fn click_merge(&mut self, id: TagId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        self.open_merge_dialog(Some(id), cx);
    }

    pub(crate) fn click_remove(&mut self, id: TagId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        self.open_remove_dialog(id, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------------------------

    fn open_add_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let form = TagForm::new(self.tags(cx).to_vec());
        open_dialog(TagsDialog::Add(form), Self::land, cx);
    }

    /// Opens the Edit dialog pre-filled from Tag `id`. A no-op if the Tag is gone.
    fn open_edit_dialog(&mut self, id: TagId, cx: &mut Context<'_, Self>) {
        let rows = self.tags(cx);
        let Some(tag) = lib_tags::get(rows, id) else {
            return;
        };
        let form = TagForm::for_edit(tag, rows.to_vec());
        open_dialog(TagsDialog::Edit(id, form), Self::land, cx);
    }

    /// Opens the Remove dialog on Tag `id`, counting its Transactions. A no-op if the Tag is gone.
    fn open_remove_dialog(&mut self, id: TagId, cx: &mut Context<'_, Self>) {
        let Some(tag) = lib_tags::get(self.tags(cx), id) else {
            return;
        };
        let form = RemoveTagForm::new(
            &tag.name,
            lib_tags::transaction_count(self.transactions(cx), id),
        );
        open_dialog(TagsDialog::Remove(id, form), Self::land, cx);
    }

    /// Opens the merge dialog with `source` as the source Tag and, when it is flagged as a likely
    /// duplicate, its suggested target (#354). `None` (the palette's `tags merge`) leaves both
    /// selects empty.
    pub fn open_merge_dialog(&mut self, source: Option<TagId>, cx: &mut Context<'_, Self>) {
        let groups = self.duplicate_groups(cx);
        let target = source.and_then(|id| lib_tags::duplicate_of(&groups, id));
        let form = MergeTagsForm::new(self.merge_options(cx), source, target);
        open_dialog(TagsDialog::Merge(form), Self::land, cx);
    }

    /// Lands a confirmed Dialog: a save selects its Tag and a merge its target. A removal keeps
    /// the selection in range, since a removed Tag's id no longer resolves and the page lands on
    /// its first row.
    fn land(&mut self, outcome: TagOutcome, cx: &mut Context<'_, Self>) {
        match outcome {
            TagOutcome::Saved(id) | TagOutcome::Merged { target: id, .. } => self.select_id(id, cx),
            TagOutcome::Removed { .. } => {}
        }
    }

    /// The 7e selects' options, labelled `Shared (9 txns)`.
    fn merge_options(&self, cx: &App) -> Vec<form::MergeOption> {
        merge_options(self.tags(cx), self.transactions(cx), |name, count| {
            crate::msg::desktop_tags_merge_option(name, i64::try_from(count).unwrap_or(i64::MAX))
        })
    }
}

/// How many Tags are flagged as likely duplicates: every group member but its suggested target.
pub fn likely_duplicate_count(groups: &[DuplicateGroup]) -> usize {
    groups.iter().map(|group| group.duplicates.len()).sum()
}

/// The Tags page: names A–Z and the selected row's name. The open Dialog reads itself back
/// (`Dialog::readback`).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTagsSnapshot {
    pub tag_names: Vec<String>,
    /// Each Tag's id, in the same order as the names.
    pub tag_ids: Vec<TagId>,
    pub selected_tag: Option<String>,
}

impl TagsPage {
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsTagsSnapshot {
        let rows = self.tags(cx);
        SettingsTagsSnapshot {
            tag_names: lib_tags::sorted_by_name(rows)
                .into_iter()
                .map(|tag| tag.name.clone())
                .collect(),
            tag_ids: lib_tags::sorted_by_name(rows)
                .into_iter()
                .map(|tag| tag.id)
                .collect(),
            selected_tag: self
                .selected_id(cx)
                .and_then(|id| lib_tags::get(rows, id))
                .map(|tag| tag.name.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use lib_payees::default_payees;
    mod hosted {
        use std::{cell::RefCell, rc::Rc};

        use gpui::{AppContext, Entity, Subscription, TestAppContext};
        use lib_ledger::LedgerParts;
        use lib_tags::default_tags;
        use lib_toast::ToastKind;

        use super::super::TagsPage;
        use crate::{
            chrome::dialog_host::{Dialog, DialogScope, HostedOpen, Landed, OpenDialog, type_text},
            ledger_store::test_ledger,
            view::event::ViewEvent,
        };

        /// A Tags page over the default Tags, and the scope a hosted Dialog runs in.
        fn page(cx: &mut TestAppContext) -> (Entity<TagsPage>, DialogScope) {
            let ledger = test_ledger(
                cx,
                LedgerParts {
                    tags: default_tags(),
                    ..LedgerParts::default()
                },
            );
            let page = cx.new(|cx| TagsPage::new(ledger.clone(), cx));
            let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
            (page, DialogScope { ledger, today })
        }

        /// Runs `open` on the page and returns the hosted Dialog it asked the host to open.
        fn opened(
            page: &Entity<TagsPage>,
            cx: &mut TestAppContext,
            open: impl FnOnce(&mut TagsPage, &mut gpui::Context<'_, TagsPage>),
        ) -> HostedOpen {
            let opened = Rc::new(RefCell::new(None));
            let _subscription: Subscription = cx.update(|cx| {
                let opened = opened.clone();
                cx.subscribe(page, move |_, event: &ViewEvent, _| {
                    if let ViewEvent::OpenHostedDialog(open) = event {
                        *opened.borrow_mut() = Some((**open).clone());
                    }
                })
            });
            page.update(cx, open);
            opened.take().expect("the page opened a hosted Dialog")
        }

        fn selected(page: &Entity<TagsPage>, cx: &mut TestAppContext) -> Option<String> {
            cx.read(|cx| page.read(cx).snapshot(cx).selected_tag)
        }

        #[gpui::test]
        fn a_confirmed_add_hands_the_new_tag_to_the_page_to_select(cx: &mut TestAppContext) {
            let (page, scope) = page(cx);
            let HostedOpen {
                mut dialog,
                confirm,
            } = opened(&page, cx, |page, cx| page.click_add(cx));
            assert!(matches!(dialog, OpenDialog::Tags(_)));

            assert_eq!(
                cx.update(|cx| confirm(&mut dialog, &scope, cx)),
                None,
                "a blank name stays open"
            );
            type_text(&mut dialog, "zebra");
            let landed = cx.update(|cx| confirm(&mut dialog, &scope, cx));

            assert_eq!(
                landed,
                Some(Landed {
                    toast: None,
                    outcome: ()
                })
            );
            assert_eq!(selected(&page, cx).as_deref(), Some("zebra"));
        }

        #[gpui::test]
        fn a_confirmed_merge_toasts_and_selects_its_target(cx: &mut TestAppContext) {
            let (page, scope) = page(cx);
            let duplicate = cx
                .read(|cx| page.read(cx).duplicate_groups(cx))
                .iter()
                .find_map(|group| group.duplicates.first().copied())
                .expect("the seeded Tags hold a likely duplicate");
            let HostedOpen {
                mut dialog,
                confirm,
            } = opened(&page, cx, |page, cx| page.click_merge(duplicate, cx));
            let target = cx
                .read(|cx| dialog.readback(&scope, cx))
                .and_then(|readback| readback.value("target").map(str::to_string));
            assert!(
                target.is_some(),
                "a flagged duplicate opens with its target"
            );

            let landed = cx.update(|cx| confirm(&mut dialog, &scope, cx));

            let toast = landed.and_then(|landed| landed.toast);
            assert!(matches!(toast, Some((ToastKind::Success, _))));
            assert_eq!(selected(&page, cx), target);
        }
    }

    #[test]
    fn sorted_by_name_ignores_case() {
        let tags = default_tags();
        let names: Vec<String> = sorted_by_name(&tags)
            .iter()
            .map(|tag| tag.name.to_lowercase())
            .collect();
        let mut expected = names.clone();
        expected.sort();
        assert_eq!(names, expected);
    }

    #[test]
    fn only_the_non_target_members_count_as_likely_duplicates() {
        let groups = [
            DuplicateGroup {
                target: TagId::seed(1),
                duplicates: vec![TagId::seed(2), TagId::seed(3)],
            },
            DuplicateGroup {
                target: TagId::seed(4),
                duplicates: vec![TagId::seed(5)],
            },
        ];
        assert_eq!(likely_duplicate_count(&groups), 3);
        assert_eq!(likely_duplicate_count(&[]), 0);
    }

    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_accounts::Account;
    use lib_core::{Money, Total, TransactionStatus};
    use lib_transactions::Transaction;

    use super::*;
    use lib_core::TransactionId;
    use lib_core::{AccountId, CategoryId};
    use lib_tags::*;

    use crate::transactions::default_transactions;
    use lib_accounts::default_accounts;
    use lib_categories::default_categories;
    use lib_transactions::Split;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    pub(super) fn seeded() -> (Vec<Account>, Vec<Tag>, Vec<Transaction>) {
        let accounts = default_accounts();
        let tags = default_tags();
        let transactions = default_transactions(
            &accounts,
            &default_categories(),
            &default_payees(),
            &tags,
            today(),
        );
        (accounts, tags, transactions)
    }

    fn id(tags: &[Tag], name: &str) -> TagId {
        tags.iter().find(|t| t.name == name).unwrap().id
    }

    fn draft(name: &str) -> TagDraft {
        TagDraft {
            name: name.to_string(),
            color: None,
        }
    }

    fn split(cents: i64, tag_ids: &[TagId]) -> Split {
        Split {
            amount: Money(BigDecimal::new(cents.into(), 2)),
            category_id: CategoryId::seed(7),
            payee_id: None,
            tag_ids: tag_ids.to_vec(),
        }
    }

    fn transaction(
        id: TransactionId,
        account_id: AccountId,
        date: NaiveDate,
        splits: Vec<Split>,
    ) -> Transaction {
        Transaction {
            id,
            date,
            account_id,
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits,
        }
    }

    fn account_id(accounts: &[Account], name: &str) -> AccountId {
        accounts.iter().find(|a| a.name == name).unwrap().id
    }

    #[test]
    fn normalise_lower_cases_and_keeps_only_letters_and_digits() {
        assert_eq!(normalise("Work-Trip"), "worktrip");
        assert_eq!(normalise(" work_trip 2 "), "worktrip2");
        assert_eq!(normalise("Überweisung"), "überweisung");
        assert_eq!(normalise("--- !"), "");
    }

    #[test]
    fn seeded_tags_break_the_rule_only_for_the_one_duplicate_pair() {
        let (_, tags, transactions) = seeded();
        let groups = duplicate_groups(&tags, &transactions);
        assert_eq!(
            groups,
            vec![DuplicateGroup {
                target: id(&tags, "work-trip"),
                duplicates: vec![id(&tags, "Work Trip")],
            }]
        );
    }

    #[test]
    fn the_seed_has_one_inactive_tag_left_out_of_the_count() {
        let (_, tags, _) = seeded();
        assert!(!get(&tags, id(&tags, "Bali 2025")).unwrap().is_active);
        assert_eq!(active_count(&tags), tags.len() - 1);
    }

    #[test]
    fn seeded_usage_keeps_the_mockups_order() {
        let (_, tags, transactions) = seeded();
        let order: Vec<&str> = sorted_by_usage(&tags, &transactions)
            .iter()
            .map(|t| t.name.as_str())
            .take(6)
            .collect();
        assert_eq!(
            order,
            [
                "shared",
                "reimbursable",
                "tax-deductible",
                "work-trip",
                "gift",
                "one-off"
            ]
        );
        assert_eq!(transaction_count(&transactions, id(&tags, "one-off")), 3);
        assert!(transaction_count(&transactions, id(&tags, "Work Trip")) > 0);
        assert!(transaction_count(&transactions, id(&tags, "Bali 2025")) > 0);
    }

    #[test]
    fn find_by_name_ignores_case_but_not_punctuation() {
        let tags = default_tags();
        assert_eq!(find_by_name(&tags, "SHARED"), Some(id(&tags, "shared")));
        assert_eq!(find_by_name(&tags, "tax deductible"), None);
        assert_eq!(find_by_name(&tags, "missing"), None);
    }

    #[test]
    fn insert_trims_the_name_and_starts_active() {
        let mut tags = default_tags();
        let new = insert_tag(
            &mut tags,
            &TagDraft {
                name: "  holiday ".to_string(),
                color: Some(swatches()[3].clone()),
            },
        )
        .unwrap();
        let tag = get(&tags, new).unwrap();
        assert_eq!(tag.name, "holiday");
        assert_eq!(tag.color, Some(swatches()[3].clone()));
        assert!(tag.is_active);
        assert_eq!(new, tags.iter().map(|t| t.id).max().unwrap());
    }

    #[test]
    fn insert_refuses_empty_punctuation_only_and_normalised_duplicates() {
        let mut tags = default_tags();
        assert_eq!(
            insert_tag(&mut tags, &draft("   ")),
            Err(TagError::NameRequired)
        );
        assert_eq!(
            insert_tag(&mut tags, &draft("- !")),
            Err(TagError::NoLetterOrDigit)
        );
        assert_eq!(
            insert_tag(&mut tags, &draft("Tax Deductible")),
            Err(TagError::DuplicateName("tax-deductible".to_string()))
        );
        // An inactive Tag's name stays taken.
        assert_eq!(
            insert_tag(&mut tags, &draft("bali-2025")),
            Err(TagError::DuplicateName("Bali 2025".to_string()))
        );
    }

    #[test]
    fn edit_renames_and_recolours_and_may_keep_its_own_name() {
        let mut tags = default_tags();
        let shared = id(&tags, "shared");
        edit_tag(&mut tags, shared, &draft("Shared")).unwrap();
        assert_eq!(get(&tags, shared).unwrap().name, "Shared");
        assert_eq!(get(&tags, shared).unwrap().color, None);
        assert_eq!(
            edit_tag(&mut tags, shared, &draft("gift!")),
            Err(TagError::DuplicateName("gift".to_string()))
        );
        assert_eq!(
            edit_tag(&mut tags, TagId::seed(999), &draft("x")),
            Err(TagError::NotFound)
        );
    }

    #[test]
    fn set_active_toggles() {
        let mut tags = default_tags();
        let gift = id(&tags, "gift");
        set_active(&mut tags, gift, false).unwrap();
        assert!(!get(&tags, gift).unwrap().is_active);
        set_active(&mut tags, gift, true).unwrap();
        assert!(get(&tags, gift).unwrap().is_active);
        assert_eq!(
            set_active(&mut tags, TagId::seed(999), false),
            Err(TagError::NotFound)
        );
    }

    #[test]
    fn remove_untags_every_split_and_keeps_the_transactions() {
        let (_, mut tags, mut transactions) = seeded();
        let before = transactions.clone();
        let shared = id(&tags, "shared");
        remove_tag(&mut tags, &mut transactions, shared).unwrap();
        assert!(get(&tags, shared).is_none());
        assert_eq!(transaction_count(&transactions, shared), 0);
        assert_eq!(transactions.len(), before.len());
        for (after, before) in transactions.iter().zip(&before) {
            assert_eq!(after.total(), before.total());
            for (a, b) in after.splits.iter().zip(&before.splits) {
                assert_eq!((a.category_id, a.payee_id), (b.category_id, b.payee_id));
            }
        }
        assert_eq!(
            remove_tag(&mut tags, &mut transactions, shared),
            Err(TagError::NotFound)
        );
    }

    #[test]
    fn remove_deletes_an_unused_tag_too() {
        let (_, mut tags, mut transactions) = seeded();
        let new = insert_tag(&mut tags, &draft("unused")).unwrap();
        remove_tag(&mut tags, &mut transactions, new).unwrap();
        assert!(get(&tags, new).is_none());
    }

    #[test]
    fn merge_retags_without_doubling_and_deletes_the_source() {
        let mut tags = default_tags();
        let (source, target, other) = (
            id(&tags, "Work Trip"),
            id(&tags, "work-trip"),
            TagId::seed(1),
        );
        let day = today();
        let mut transactions = vec![
            transaction(
                TransactionId::seed(1),
                AccountId::seed(2),
                day,
                vec![split(-100, &[source])],
            ),
            transaction(
                TransactionId::seed(2),
                AccountId::seed(2),
                day,
                vec![split(-200, &[source, target]), split(-50, &[other])],
            ),
            transaction(
                TransactionId::seed(3),
                AccountId::seed(2),
                day,
                vec![split(-300, &[target])],
            ),
        ];
        merge_tags(&mut tags, &mut transactions, source, target).unwrap();
        assert!(get(&tags, source).is_none());
        assert_eq!(transactions[0].splits[0].tag_ids, vec![target]);
        assert_eq!(transactions[1].splits[0].tag_ids, vec![target]);
        assert_eq!(transactions[1].splits[1].tag_ids, vec![other]);
        assert_eq!(transaction_count(&transactions, target), 3);
        // The target keeps its own name, colour and state.
        let kept = get(&tags, target).unwrap();
        assert_eq!(kept.name, "work-trip");
        assert_eq!(kept.color, Some(swatches()[4].clone()));
    }

    #[test]
    fn merge_refuses_itself_and_missing_tags_and_allows_inactive_ones() {
        let (_, mut tags, mut transactions) = seeded();
        let gift = id(&tags, "gift");
        let bali = id(&tags, "Bali 2025");
        assert_eq!(
            merge_tags(&mut tags, &mut transactions, gift, gift),
            Err(TagError::MergeIntoItself)
        );
        assert_eq!(
            merge_tags(&mut tags, &mut transactions, TagId::seed(999), gift),
            Err(TagError::NotFound)
        );
        let expected =
            transaction_count(&transactions, gift) + transaction_count(&transactions, bali);
        merge_tags(&mut tags, &mut transactions, bali, gift).unwrap();
        assert_eq!(transaction_count(&transactions, gift), expected);
        assert!(get(&tags, gift).unwrap().is_active);
    }

    #[test]
    fn a_split_reuses_an_exact_match_and_refuses_a_normalised_one() {
        let tags = default_tags();
        assert_eq!(
            resolve_split_tag(&tags, " SHARED "),
            Ok(SplitTag::Existing(id(&tags, "shared")))
        );
        assert_eq!(
            resolve_split_tag(&tags, "one off"),
            Err(TagError::DuplicateName("one-off".to_string()))
        );
        assert_eq!(resolve_split_tag(&tags, "camping"), Ok(SplitTag::New));
        assert_eq!(resolve_split_tag(&tags, ""), Err(TagError::NameRequired));
    }

    #[test]
    fn usage_counts_distinct_transactions_sums_the_tagged_splits_and_takes_the_newest_date() {
        let accounts = default_accounts();
        let everyday = account_id(&accounts, "ANZ Everyday");
        let day = today();
        let transactions = vec![
            transaction(
                TransactionId::seed(1),
                everyday,
                day - chrono::Duration::days(3),
                vec![
                    split(-1_000, &[TagId::seed(5)]),
                    split(-250, &[TagId::seed(5)]),
                    split(-9_999, &[]),
                ],
            ),
            // Future-dated still counts as last used.
            transaction(
                TransactionId::seed(2),
                everyday,
                day + chrono::Duration::days(2),
                vec![split(-500, &[TagId::seed(5)])],
            ),
            transaction(
                TransactionId::seed(3),
                everyday,
                day,
                vec![split(-700, &[TagId::seed(6)])],
            ),
        ];
        let used = usage(&transactions, &accounts, TagId::seed(5), |_| {
            "aud".to_string()
        });
        assert_eq!(used.transactions, 2);
        assert_eq!(
            used.total,
            Total::Single {
                unit: "aud".to_string(),
                amount: Money(BigDecimal::new((-1_750).into(), 2)),
            }
        );
        assert_eq!(used.last_used, Some(day + chrono::Duration::days(2)));
    }

    #[test]
    fn usage_across_units_is_mixed_and_an_unused_tag_is_empty() {
        let accounts = default_accounts();
        let day = today();
        let transactions = vec![
            transaction(
                TransactionId::seed(1),
                account_id(&accounts, "ANZ Everyday"),
                day,
                vec![split(-1_000, &[TagId::seed(5)])],
            ),
            transaction(
                TransactionId::seed(2),
                account_id(&accounts, "Bitcoin"),
                day,
                vec![split(-500, &[TagId::seed(5)])],
            ),
        ];
        assert_eq!(
            usage(&transactions, &accounts, TagId::seed(5), |_| "aud"
                .to_string())
            .total,
            Total::Mixed
        );
        let unused = usage(&transactions, &accounts, TagId::seed(6), |_| {
            "aud".to_string()
        });
        assert_eq!(
            (unused.transactions, unused.total, unused.last_used),
            (0, Total::Empty, None)
        );
    }

    #[test]
    fn sort_ties_fall_back_to_the_name() {
        let tags = vec![
            Tag {
                id: TagId::seed(1),
                name: "zeta".to_string(),
                color: None,
                is_active: true,
            },
            Tag {
                id: TagId::seed(2),
                name: "Alpha".to_string(),
                color: None,
                is_active: true,
            },
            Tag {
                id: TagId::seed(3),
                name: "beta".to_string(),
                color: None,
                is_active: true,
            },
        ];
        let transactions = vec![transaction(
            TransactionId::seed(1),
            AccountId::seed(2),
            today(),
            vec![split(-1, &[TagId::seed(3)])],
        )];
        let order: Vec<TagId> = sorted_by_usage(&tags, &transactions)
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(order, [TagId::seed(3), TagId::seed(2), TagId::seed(1)]);
    }

    #[test]
    fn the_suggested_target_is_active_then_more_used_then_older() {
        let tag = |id, name: &str, is_active| Tag {
            id,
            name: name.to_string(),
            color: None,
            is_active,
        };
        let day = today();
        let used =
            |id, tag_id| transaction(id, AccountId::seed(2), day, vec![split(-1, &[tag_id])]);

        // More Transactions wins among active Tags.
        let tags = vec![
            tag(TagId::seed(1), "Shared", true),
            tag(TagId::seed(2), "shared", true),
        ];
        let transactions = vec![used(TransactionId::seed(1), TagId::seed(2))];
        let groups = duplicate_groups(&tags, &transactions);
        assert_eq!(
            groups[0],
            DuplicateGroup {
                target: TagId::seed(2),
                duplicates: vec![TagId::seed(1)]
            }
        );
        assert_eq!(duplicate_of(&groups, TagId::seed(1)), Some(TagId::seed(2)));
        assert_eq!(duplicate_of(&groups, TagId::seed(2)), None);

        // Active beats more used.
        let tags = vec![
            tag(TagId::seed(1), "Shared", true),
            tag(TagId::seed(2), "shared", false),
        ];
        assert_eq!(
            duplicate_groups(&tags, &transactions)[0].target,
            TagId::seed(1)
        );

        // A tie goes to the older (lower id); every other member is flagged against it.
        let tags = vec![
            tag(TagId::seed(4), "work trip", true),
            tag(TagId::seed(3), "Work-Trip", true),
            tag(TagId::seed(5), "WORK_TRIP", false),
        ];
        assert_eq!(
            duplicate_groups(&tags, &[]),
            vec![DuplicateGroup {
                target: TagId::seed(3),
                duplicates: vec![TagId::seed(4), TagId::seed(5)]
            }]
        );
    }
}
