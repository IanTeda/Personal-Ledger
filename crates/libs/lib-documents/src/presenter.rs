//! One Presenter per Documents View: the Documents page (the Library and the Inbox), the Settings
//! › Documents page and each of its Dialogs (Add and Edit share the form). Each takes events
//! through `handle` and builds a plain-data view model through `view_model`, its text already
//! localised from [`crate::msg`], so a View only draws. They share one
//! [`SharedRepository`](crate::protocols::SharedRepository) and ask for navigation through the
//! Router the Client passes in. Writes go through [`crate::interactor`]'s Interactors.
//!
//! The Documents page's Dialogs (Add, Edit, Import, Edit facts, Accept all and the link picker)
//! aren't Presenters yet (#968): the page asks its Router to open each, and the Client hands back
//! what one did. The link picker words its rows with [`library`]'s functions.

mod default_notice;
mod documents_page;
pub mod inbox;
pub mod library;
mod remove_type;
mod type_form;
mod types_page;

use crate::msg;
use crate::types::TracksDate;

pub use default_notice::{DefaultNoticeEvent, DefaultNoticePresenter, DefaultNoticeViewModel};
pub use documents_page::{
    DocumentsFocus, DocumentsPageEvent, DocumentsPagePresenter, DocumentsPageViewModel,
    InboxViewModel, LibraryViewModel, PageLabels, inbox_below,
};
pub use remove_type::{RemoveTypeEvent, RemoveTypePresenter, RemoveTypeViewModel};
pub use type_form::{
    DocumentTypeField, NAME_MAX, NameError, REMIND_OPTIONS, TRACKS_OPTIONS, TypeFormEvent,
    TypeFormPresenter, TypeFormViewModel,
};
pub use types_page::{
    DocumentTypesPageEvent, DocumentTypesPagePresenter, DocumentTypesPageViewModel, TypeColumns,
    TypeLine,
};

/// The text of a Tracks date, in the page's cell and the form's options.
fn tracks_label(tracks: TracksDate) -> String {
    match tracks {
        TracksDate::Renews => msg::documents_types_tracks_renews(),
        TracksDate::Ends => msg::documents_types_tracks_ends(),
        TracksDate::Expires => msg::documents_types_tracks_expires(),
        TracksDate::Revalue => msg::documents_types_tracks_revalue(),
    }
}

/// A count as a Message's plural argument.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_core::{AccountId, BillPlanId, ItemId, Money, PayeeId, TransactionId};

    use super::library::Lookups;
    use crate::fake::InMemoryDocuments;
    use crate::protocols::{
        DocumentAccount, DocumentBillPlan, DocumentItem, DocumentPayee, DocumentTransaction,
        LinkRecords, SharedRepository,
    };
    use crate::types::{DocumentTypeRow, default_types};
    use crate::{Document, default_documents};

    /// Composes this crate's layer into the process Locale, which every test that reads text
    /// needs: the first explicit `init` wins, so each test calls it before reading any.
    pub fn locale() {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
    }

    /// The day the Documents page's tests are seeded on.
    pub fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date")
    }

    /// The records the stub Documents name, as the other domains' stubs hold them: enough of each
    /// for the seed's Links to land, and the Woolworths purchase the Inbox's strongest row matches.
    pub fn records(today: NaiveDate) -> LinkRecords {
        let day = |offset: i64| today + chrono::Duration::days(offset);
        let money = |cents: i64| Money(BigDecimal::new(cents.into(), 2));
        let account = |id: u32, name: &str| DocumentAccount {
            id: AccountId::seed(id),
            name: name.to_string(),
            institution: String::new(),
        };
        let payee = |id: u32, name: &str, aliases: &[&str]| DocumentPayee {
            id: PayeeId::seed(id),
            name: name.to_string(),
            aliases: aliases.iter().map(|alias| (*alias).to_string()).collect(),
            is_active: true,
        };
        let item = |id: u32, name: &str| DocumentItem {
            id: ItemId::seed(id),
            name: name.to_string(),
        };
        let plan = |id: u32, name: &str, payee: u32| DocumentBillPlan {
            id: BillPlanId::seed(id),
            name: name.to_string(),
            cadence: "Yearly".to_string(),
            payee_id: Some(PayeeId::seed(payee)),
        };
        LinkRecords {
            transactions: vec![
                DocumentTransaction {
                    id: TransactionId::seed(1),
                    date: day(-3),
                    amount: money(-21_240),
                    account_id: AccountId::seed(1),
                    payee_ids: vec![Some(PayeeId::seed(1))],
                    memo: None,
                },
                DocumentTransaction {
                    id: TransactionId::seed(2),
                    date: day(-40),
                    amount: money(-329_900),
                    account_id: AccountId::seed(1),
                    payee_ids: vec![None],
                    memo: Some("CAMERA HOUSE \u{2014} Sony A7 IV".to_string()),
                },
            ],
            payees: vec![
                payee(1, "Woolworths", &["WOOLWORTHS"]),
                payee(2, "AAMI", &[]),
                payee(3, "Brisbane City Council", &[]),
                payee(4, "Ray White Rentals", &[]),
            ],
            accounts: vec![
                account(1, "Amex Platinum"),
                account(2, "ANZ Everyday"),
                account(3, "ANZ Offset"),
                account(4, "Home Loan"),
                account(5, "Vanguard VAS"),
            ],
            plans: vec![
                plan(1, "Car Insurance \u{2014} AAMI", 2),
                plan(2, "Council Rates", 3),
            ],
            items: vec![
                item(1, "Sony A7 IV"),
                item(2, "Fridge"),
                item(3, "Toyota Corolla"),
                item(4, "Engagement ring"),
            ],
        }
    }

    /// The stub Documents over [`records`], as the page's builders read them.
    pub struct World {
        pub types: Vec<DocumentTypeRow>,
        pub records: LinkRecords,
        pub documents: Vec<Document>,
        pub today: NaiveDate,
    }

    pub fn world() -> World {
        locale();
        let today = today();
        let records = records(today);
        World {
            types: default_types(),
            documents: default_documents(&records, today).documents,
            records,
            today,
        }
    }

    impl World {
        pub fn lookups(&self) -> Lookups<'_> {
            Lookups {
                types: &self.types,
                records: &self.records,
                today: self.today,
                date_style: None,
            }
        }

        /// The Document titled `title`.
        pub fn named(&self, title: &str) -> &Document {
            self.documents
                .iter()
                .find(|document| document.title == title)
                .expect("the seed holds that Document")
        }

        /// The Document whose file is `file`.
        pub fn file(&self, file: &str) -> &Document {
            self.documents
                .iter()
                .find(|document| document.file_name() == file)
                .expect("the seed holds that file")
        }
    }

    /// The nine default types, shared, with the handle the test reads them back through.
    pub fn stub() -> (Rc<RefCell<InMemoryDocuments>>, SharedRepository) {
        locale();
        let documents = Rc::new(RefCell::new(InMemoryDocuments {
            documents: Vec::new(),
            types: default_types(),
        }));
        let shared: SharedRepository = documents.clone();
        (documents, shared)
    }
}
