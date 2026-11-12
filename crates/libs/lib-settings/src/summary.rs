//! What the Settings index's detail pane shows for a page (#1109, DESKTOP-182): its name, scope
//! note, what it is for and its current values read-only, as label and value pairs. A form page
//! quotes its fields; a list page quotes a count and one headline fact, matching its heading note,
//! never rows. A This Ledger page reads [`LedgerSummaries`] (the stores' counts, answered by the
//! Client) and, with no Ledger open, carries no values, says so, and is marked so **Open** can be
//! disabled.
//!
//! A This device page answers its own rows (each Presenter's `summary_rows`), so the summary never
//! reads a page's state; a This Ledger page's rows come from the Ledger's counts, so the index
//! never builds a domain's Presenter. The summary is a plain function of them, handed in as
//! [`SummarySources`], so the pane follows the cursor without holding state.

use lib_locale::format::{AmountStyle, format_amount};

use crate::msg;
use crate::page::{ScopeFacts, ScopeGroup, SettingsPage};
use crate::protocols::{HeadlineAmount, LedgerCounts, LedgerSummaries};

/// One read-only value: its label and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRow {
    pub label: String,
    pub value: String,
}

impl SummaryRow {
    pub fn new(label: String, value: impl Into<String>) -> Self {
        Self {
            label,
            value: value.into(),
        }
    }
}

/// A page as the detail pane summarises it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSummary {
    pub page: SettingsPage,
    /// The page's name.
    pub name: String,
    /// The page's scope note, as its heading draws it.
    pub scope_note: String,
    /// What the page is for.
    pub purpose: String,
    /// The page's values; empty for a This Ledger page with no Ledger open.
    pub rows: Vec<SummaryRow>,
    /// The line saying why there are no values (`No Ledger open: :open or :new first`).
    pub note: Option<String>,
    /// Whether **Open** may open the page: false for a This Ledger page with no Ledger open.
    pub open_enabled: bool,
}

/// What a summary reads: the page's own rows for a This device page, or the Ledger's counts.
pub struct SummarySources<'a> {
    /// The summarised page's rows as its Presenter answers them (`summary_rows`), for a This
    /// device page; ignored for a This Ledger page.
    pub device_rows: Vec<SummaryRow>,
    pub facts: ScopeFacts,
    /// The Open Ledger's counts; `None` with no Ledger open.
    pub ledger: Option<&'a dyn LedgerSummaries>,
}

impl ScopeGroup {
    /// The paragraph explaining the group on the X-Large detail pane: where its settings are kept
    /// and whether they sync.
    pub fn pane_paragraph(self) -> String {
        match self {
            Self::ThisDevice => msg::settings_pane_this_device(),
            Self::ThisLedger => msg::settings_pane_this_ledger(),
        }
    }
}

impl SettingsPage {
    /// What the page is for, in a sentence or two.
    pub fn purpose(self) -> String {
        match self {
            Self::General => msg::settings_purpose_general(),
            Self::Display => msg::settings_purpose_display(),
            Self::Ledger => msg::settings_purpose_ledger(),
            Self::Units => msg::settings_purpose_units(),
            Self::Institutions => msg::settings_purpose_institutions(),
            Self::Accounts => msg::settings_purpose_accounts(),
            Self::Categories => msg::settings_purpose_categories(),
            Self::Tags => msg::settings_purpose_tags(),
            Self::Payees => msg::settings_purpose_payees(),
            Self::Documents => msg::settings_purpose_documents(),
            Self::Inventory => msg::settings_purpose_inventory(),
            Self::SyncServer => msg::settings_purpose_sync_server(),
            Self::DataBackup => msg::settings_purpose_data_backup(),
            Self::Tracing => msg::settings_purpose_tracing(),
            Self::About => msg::settings_purpose_about(),
        }
    }
}

/// `page`'s summary under `sources`. The Ledger's counts are read only for a This Ledger page
/// that has them.
pub fn summarise(page: SettingsPage, sources: SummarySources<'_>) -> PageSummary {
    let mut summary = PageSummary {
        page,
        name: page.label(),
        scope_note: page.scope_note(sources.facts),
        purpose: page.purpose(),
        rows: Vec::new(),
        note: None,
        open_enabled: true,
    };
    if !page.needs_ledger() {
        summary.rows = sources.device_rows;
        return summary;
    }
    match sources.ledger {
        Some(ledger) => summary.rows = ledger_rows(page, &ledger.counts()),
        None => {
            summary.note = Some(msg::settings_status_refused_no_ledger(":open", ":new"));
            summary.open_enabled = false;
        }
    }
    summary
}

/// A This Ledger page's values, from the Ledger's counts.
fn ledger_rows(page: SettingsPage, counts: &LedgerCounts) -> Vec<SummaryRow> {
    let number = |label: String, n: usize| SummaryRow::new(label, n.to_string());
    match page {
        SettingsPage::Ledger => counts
            .base_unit
            .iter()
            .map(|code| SummaryRow::new(msg::settings_ledger_base_unit(), code.clone()))
            .collect(),
        SettingsPage::Units => {
            let mut rows = vec![number(msg::settings_summary_units(), counts.units)];
            rows.extend(
                counts
                    .base_unit
                    .iter()
                    .map(|code| SummaryRow::new(msg::settings_ledger_base_unit(), code.clone())),
            );
            rows
        }
        SettingsPage::Institutions => {
            vec![number(
                msg::settings_summary_institutions(),
                counts.institutions,
            )]
        }
        SettingsPage::Accounts => {
            let mut rows = vec![number(msg::settings_summary_accounts(), counts.accounts)];
            rows.extend(
                counts
                    .net_worth
                    .iter()
                    .map(|worth| SummaryRow::new(msg::settings_summary_net_worth(), amount(worth))),
            );
            rows
        }
        SettingsPage::Categories => vec![
            number(msg::settings_summary_categories(), counts.categories),
            SummaryRow::new(
                msg::settings_summary_levels_deep(),
                counts.category_levels.to_string(),
            ),
        ],
        SettingsPage::Tags => vec![
            number(msg::settings_summary_tags(), counts.tags),
            number(
                msg::settings_summary_likely_duplicates(),
                counts.likely_duplicate_tags,
            ),
        ],
        SettingsPage::Payees => vec![
            number(msg::settings_summary_payees(), counts.payees),
            number(
                msg::settings_summary_without_default(),
                counts.payees_without_default_category,
            ),
        ],
        SettingsPage::Documents => vec![
            number(
                msg::settings_summary_document_types(),
                counts.document_types,
            ),
            SummaryRow::new(
                msg::settings_summary_files_typed(),
                msg::settings_summary_typed_of(
                    &counts.files_typed.to_string(),
                    &counts.files.to_string(),
                ),
            ),
        ],
        SettingsPage::Inventory => vec![
            number(msg::settings_summary_locations(), counts.locations),
            number(msg::settings_summary_areas(), counts.areas),
            number(msg::settings_summary_items(), counts.items),
        ],
        // Neither has data of its own yet, so neither quotes a value (#1161).
        SettingsPage::SyncServer
        | SettingsPage::DataBackup
        | SettingsPage::General
        | SettingsPage::Display
        | SettingsPage::Tracing
        | SettingsPage::About => Vec::new(),
    }
}

/// A headline figure as the Accounts heading draws it: `83,995.87 aud`, with the Locale's minus.
fn amount(worth: &HeadlineAmount) -> String {
    let figure = format_amount(&worth.amount, AmountStyle::fixed(worth.places));
    msg::settings_summary_amount(&figure.text, &worth.code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FixedLedgerSummaries;

    fn locale() {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
    }

    const FACTS: ScopeFacts = ScopeFacts { log_capacity: 500 };

    /// Sources for a page whose Presenter answered `device_rows`, with `ledger` open or none.
    fn sources(
        device_rows: Vec<SummaryRow>,
        ledger: Option<&dyn LedgerSummaries>,
    ) -> SummarySources<'_> {
        SummarySources {
            device_rows,
            facts: FACTS,
            ledger,
        }
    }

    fn pairs(summary: &PageSummary) -> Vec<(&str, &str)> {
        summary
            .rows
            .iter()
            .map(|r| (r.label.as_str(), r.value.as_str()))
            .collect()
    }

    #[test]
    fn a_this_device_page_quotes_the_rows_its_presenter_answered() {
        locale();
        let rows = vec![SummaryRow::new("Toasts".to_string(), "On")];

        let summary = summarise(SettingsPage::General, sources(rows.clone(), None));

        assert_eq!(summary.rows, rows);
        assert_eq!(summary.name, "General");
        assert_eq!(summary.scope_note, "this device · never synced");
        assert_eq!(summary.purpose, SettingsPage::General.purpose());
        assert!(summary.open_enabled);
        assert_eq!(summary.note, None);
    }

    #[test]
    fn a_this_device_page_needs_no_ledger() {
        locale();
        for page in ScopeGroup::ThisDevice.pages() {
            let summary = summarise(page, sources(Vec::new(), None));
            assert!(summary.open_enabled, "{page:?} opens with no Ledger");
            assert_eq!(summary.note, None);
        }
    }

    #[test]
    fn a_this_ledger_page_ignores_device_rows() {
        locale();
        let ledger = FixedLedgerSummaries::sample();
        let rows = vec![SummaryRow::new("Toasts".to_string(), "On")];

        let summary = summarise(SettingsPage::Institutions, sources(rows, Some(&ledger)));

        assert_eq!(pairs(&summary), [("Institutions", "7")]);
    }

    #[test]
    fn list_pages_quote_a_count_and_a_headline_fact() {
        locale();
        let ledger = FixedLedgerSummaries::sample();
        let of = |page| summarise(page, sources(Vec::new(), Some(&ledger)));
        assert_eq!(
            pairs(&of(SettingsPage::Accounts)),
            [("Accounts", "7"), ("Net worth", "83,995.87 aud")]
        );
        assert_eq!(
            pairs(&of(SettingsPage::Tags)),
            [("Tags", "18"), ("Likely duplicates", "2")]
        );
        assert_eq!(
            pairs(&of(SettingsPage::Categories)),
            [("Categories", "12"), ("Levels deep", "3")]
        );
        assert_eq!(
            pairs(&of(SettingsPage::Documents)),
            [("Document types", "7"), ("Files typed", "392 of 412")]
        );
        assert_eq!(
            pairs(&of(SettingsPage::Inventory)),
            [("Locations", "3"), ("Areas", "14"), ("Items", "241")]
        );
        assert_eq!(
            pairs(&of(SettingsPage::Units)),
            [("Units", "3"), ("Base unit", "aud")]
        );
        assert_eq!(
            pairs(&of(SettingsPage::Institutions)),
            [("Institutions", "7")]
        );
        assert_eq!(
            pairs(&of(SettingsPage::Payees)),
            [("Payees", "11"), ("Without a default category", "3")]
        );
        assert_eq!(pairs(&of(SettingsPage::Ledger)), [("Base unit", "aud")]);
    }

    #[test]
    fn a_negative_net_worth_keeps_its_sign() {
        locale();
        let mut counts = FixedLedgerSummaries::sample();
        if let Some(worth) = counts.0.net_worth.as_mut() {
            worth.amount = lib_core::Money("-120.5".parse().expect("a decimal literal"));
        }
        let summary = summarise(SettingsPage::Accounts, sources(Vec::new(), Some(&counts)));
        assert_eq!(summary.rows[1].value, "\u{2212}120.50 aud");
    }

    #[test]
    fn every_this_ledger_page_with_data_has_values_with_a_ledger_open() {
        locale();
        let ledger = FixedLedgerSummaries::sample();
        for page in ScopeGroup::ThisLedger.pages() {
            let summary = summarise(page, sources(Vec::new(), Some(&ledger)));
            let stub = matches!(page, SettingsPage::SyncServer | SettingsPage::DataBackup);
            assert_eq!(summary.rows.is_empty(), stub, "{page:?} quotes values");
            assert!(summary.open_enabled);
            assert_eq!(summary.note, None);
        }
    }

    #[test]
    fn with_no_ledger_a_this_ledger_page_has_no_values_says_so_and_cannot_open() {
        locale();
        for page in ScopeGroup::ThisLedger.pages() {
            let summary = summarise(page, sources(Vec::new(), None));
            assert!(summary.rows.is_empty(), "{page:?} quotes nothing");
            assert_eq!(
                summary.note.as_deref(),
                Some("No Ledger open: :open or :new first")
            );
            assert!(!summary.open_enabled, "{page:?} cannot open");
            assert_eq!(summary.name, page.label());
            assert_eq!(summary.purpose, page.purpose());
            assert_eq!(summary.scope_note, page.scope_note(FACTS));
        }
    }

    #[test]
    fn every_page_says_what_it_is_for_and_each_group_what_it_stores() {
        locale();
        for page in SettingsPage::ALL {
            assert!(!page.purpose().is_empty(), "{page:?} has a purpose");
        }
        assert!(
            ScopeGroup::ThisDevice
                .pane_paragraph()
                .contains("personal-ledger.conf")
        );
        assert!(
            ScopeGroup::ThisLedger
                .pane_paragraph()
                .contains("Ledger file")
        );
    }
}
