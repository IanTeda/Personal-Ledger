//! The stub Documents.

use std::path::PathBuf;

use bigdecimal::BigDecimal;
use chrono::{Datelike, Duration, NaiveDate};
use lib_core::{
    AccountId, BillPlanId, DocumentId, ItemId, Money, PayeeId, TransactionId, financial_year_start,
};

use super::document::{
    Document, DocumentLink, DocumentType, ExtractedFacts, FileKind, Filing, Intake, KeyDate,
    KeyDateKind, Source,
};
use crate::protocols::{DocumentTransaction, LinkRecords};

/// The stub Documents. Their Inventory Links name Items the Inventory seeds.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentsSeed {
    pub documents: Vec<Document>,
}

pub(super) fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

/// Name lookups into the shared stubs' records, so the seed reads in names rather than ids.
struct Lookup<'a> {
    records: &'a LinkRecords,
}

impl Lookup<'_> {
    fn account(&self, name: &str) -> Option<AccountId> {
        let accounts = &self.records.accounts;
        accounts.iter().find(|a| a.name == name).map(|a| a.id)
    }

    fn payee(&self, name: &str) -> Option<PayeeId> {
        let payees = &self.records.payees;
        payees.iter().find(|p| p.name == name).map(|p| p.id)
    }

    fn plan(&self, name: &str) -> Option<BillPlanId> {
        let plans = &self.records.plans;
        plans.iter().find(|p| p.name == name).map(|p| p.id)
    }

    fn item(&self, name: &str) -> Option<ItemId> {
        let items = &self.records.items;
        items.iter().find(|i| i.name == name).map(|i| i.id)
    }
}

/// A Document under construction, filled in by the seed's builders.
pub(super) struct Draft {
    file: String,
    kind: FileKind,
    pages: u32,
    bytes: u64,
    title: String,
    doc_type: DocumentType,
    date: NaiveDate,
    key_date: Option<KeyDate>,
    links: Vec<Option<DocumentLink>>,
    text: Option<&'static str>,
    intake: Option<Intake>,
}

impl Draft {
    pub(super) fn filed(file: &str, title: &str, doc_type: DocumentType, date: NaiveDate) -> Self {
        let kind = if file.ends_with(".jpg") {
            FileKind::Jpg
        } else if file.ends_with(".png") {
            FileKind::Png
        } else {
            FileKind::Pdf
        };
        Draft {
            file: file.to_string(),
            kind,
            pages: 1,
            bytes: 180_000,
            title: title.to_string(),
            doc_type,
            date,
            key_date: None,
            links: Vec::new(),
            text: None,
            intake: None,
        }
    }

    pub(super) fn unfiled(
        file: &str,
        source: Source,
        received_at: NaiveDate,
        facts: ExtractedFacts,
    ) -> Self {
        let title = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
        let mut draft = Draft::filed(file, title, DocumentType::OTHER, received_at);
        draft.intake = Some(Intake {
            source,
            received_at,
            facts,
        });
        draft
    }

    fn size(mut self, pages: u32, bytes: u64) -> Self {
        self.pages = pages;
        self.bytes = bytes;
        self
    }

    fn key(mut self, kind: KeyDateKind, date: NaiveDate, reminder: bool) -> Self {
        self.key_date = Some(KeyDate {
            kind,
            date,
            reminder,
        });
        self
    }

    fn link(mut self, link: Option<DocumentLink>) -> Self {
        self.links.push(link);
        self
    }

    fn text(mut self, text: &'static str) -> Self {
        self.text = Some(text);
        self
    }

    pub(super) fn build(self, id: DocumentId) -> Document {
        let folder = if self.intake.is_some() {
            "inbox".to_string()
        } else {
            self.date.year().to_string()
        };
        Document {
            id,
            path: PathBuf::from("documents").join(folder).join(self.file),
            kind: self.kind,
            pages: self.pages,
            bytes: self.bytes,
            title: self.title,
            doc_type: self.doc_type,
            date: self.date,
            key_date: self.key_date,
            // A record the stubs lack is dropped rather than leaving a dangling Link.
            links: self.links.into_iter().flatten().collect(),
            extracted_text: self.text.map(str::to_string),
            filing: if self.intake.is_some() {
                Filing::Unfiled
            } else {
                Filing::Filed
            },
            intake: self.intake,
        }
    }
}

/// Seeds the handoff's sample Library and Inbox rows against the shared stubs, plus filler
/// statements and receipts so the facets have weight.
///
/// The handoff's own records that the stubs lack are swapped for ones they have (sample data is
/// not final copy): Amex Platinum for "ANZ Platinum", Westpac's Home Loan for Macquarie's, Vanguard
/// VAS for CMC Markets and Brisbane City Council for "City Council" (plus a lease, so no Type facet is empty), while the NRMA policy's
/// Payee and Bill links move to an AAMI car policy whose Payee and Bill Plan exist. The sample
/// purchases (Camera House, Woolworths, Bunnings, Sonos) are `lib-transactions`' stub's, read from
/// `records` with the rest of its rows: the Camera House receipt links its purchase, and the
/// filler receipts link older ones. Key Dates sit so three Library rows Need Review on any
/// `today`.
pub fn default_documents(records: &LinkRecords, today: NaiveDate) -> DocumentsSeed {
    let lookup = Lookup { records };
    let transactions = &records.transactions;
    let day = |offset: i64| today + Duration::days(offset);
    let camera = transactions
        .iter()
        .find(|t| t.memo.as_deref() == Some("CAMERA HOUSE \u{2014} Sony A7 IV"))
        .map(|t| t.id);

    let transaction = |id: Option<TransactionId>| id.map(DocumentLink::Transaction);
    let account = |name: &str| lookup.account(name).map(DocumentLink::Account);
    let payee = |name: &str| lookup.payee(name).map(DocumentLink::Payee);
    let plan = |name: &str| lookup.plan(name).map(DocumentLink::BillPlan);
    let item = |name: &str| lookup.item(name).map(DocumentLink::InventoryItem);
    let first_of_month = day(0).with_day(1).unwrap_or(today);
    let statement_month = (first_of_month - Duration::days(1)).format("%b %Y");
    let fy = financial_year_start(today) % 100;
    let money = |cents: i64| Some(cents_money(cents));
    use KeyDateKind::*;

    let mut drafts = vec![
        // The Library's sample rows, in the handoff's order.
        Draft::filed(
            "amex-platinum-statement.pdf",
            &format!("Amex Platinum statement — {statement_month}"),
            DocumentType::STATEMENTS,
            first_of_month,
        )
        .size(6, 420_000)
        .link(account("Amex Platinum"))
        .text("American Express Platinum Card statement of account closing balance"),
        Draft::filed(
            "receipt-camera-house.pdf",
            "Receipt — Camera House",
            DocumentType::RECEIPTS,
            day(-212),
        )
        .link(item("Sony A7 IV"))
        .link(transaction(camera))
        .text("Camera House tax invoice Sony A7 IV body 4,380.00 paid by card"),
        Draft::filed(
            "nrma-home-contents-pds.pdf",
            "NRMA home contents — PDS & schedule",
            DocumentType::INSURANCE,
            day(-320),
        )
        .size(24, 1_400_000)
        .key(Renews, day(45), true)
        .link(item("Fridge"))
        .link(item("Sony A7 IV"))
        .text("NRMA Insurance home contents product disclosure statement and schedule"),
        Draft::filed(
            "aami-car-policy.pdf",
            "AAMI car insurance — policy schedule",
            DocumentType::INSURANCE,
            day(-345),
        )
        .size(8, 610_000)
        .key(Renews, day(20), true)
        .link(payee("AAMI"))
        .link(plan("Car Insurance — AAMI"))
        .link(item("Toyota Corolla"))
        .text("AAMI comprehensive car insurance certificate of insurance"),
        Draft::filed(
            "westpac-home-loan-annual.pdf",
            "Westpac home loan — annual statement",
            DocumentType::CONTRACTS,
            day(-95),
        )
        .size(4, 380_000)
        .key(Ends, day(-30), false)
        .link(account("Home Loan"))
        .text("Westpac fixed rate period ends; interest charged for the year"),
        Draft::filed(
            "vanguard-tax-statement.pdf",
            &format!("Vanguard VAS — tax statement FY{fy:02}"),
            DocumentType::TAX,
            day(-80),
        )
        .size(3, 260_000)
        .link(account("Vanguard VAS"))
        .text("Vanguard annual tax statement franked distributions capital gains"),
        Draft::filed(
            "fridge-warranty-manual.pdf",
            "Fridge warranty & manual",
            DocumentType::WARRANTIES,
            day(-420),
        )
        .size(48, 3_200_000)
        .key(Ends, day(310), false)
        .link(item("Fridge"))
        .text("Warranty card and user manual, two year manufacturer's warranty"),
        Draft::filed(
            "engagement-ring-valuation.jpg",
            "Engagement ring valuation",
            DocumentType::INSURANCE,
            day(-560),
        )
        .size(1, 2_100_000)
        .key(Revalue, day(150), false)
        .link(item("Engagement ring"))
        .text("Valuation certificate for insurance purposes"),
        Draft::filed(
            "rates-notice-q1.pdf",
            "Rates notice — Q1",
            DocumentType::BILLS,
            day(-40),
        )
        .size(2, 210_000)
        .link(payee("Brisbane City Council"))
        .link(plan("Council Rates"))
        .text("Brisbane City Council rates notice quarter one"),
        Draft::filed(
            "lease-agreement-12-elm-st.pdf",
            "Lease agreement — 12 Elm St",
            DocumentType::CONTRACTS,
            day(-250),
        )
        .size(12, 900_000)
        .key(Ends, day(115), true)
        .link(payee("Ray White Rentals"))
        .text("Residential tenancy agreement fixed term lease Ray White"),
        Draft::filed(
            "passport-scan.jpg",
            "Passport — scan",
            DocumentType::IDENTITY,
            day(-700),
        )
        .size(1, 1_800_000)
        .key(Expires, day(2_040), false),
        // The Inbox's sample rows: two Strong, a Likely, a Likely and a Weak from the stubs' own
        // Transactions, one readable with nothing to suggest, and one Unreadable.
        Draft::unfiled(
            "IMG_4471.jpg",
            Source::Scanned,
            day(-3),
            ExtractedFacts {
                merchant: Some("Woolworths Metro".to_string()),
                date: Some(day(-3)),
                total: money(21_240),
                doc_type: Some(DocumentType::RECEIPTS),
            },
        )
        .size(1, 1_200_000)
        .text("WOOLWORTHS METRO tax invoice total 212.40"),
        Draft::unfiled(
            "bunnings-invoice-INV88213.pdf",
            Source::Downloads,
            day(-6),
            ExtractedFacts {
                merchant: Some("Bunnings Warehouse".to_string()),
                date: Some(day(-6)),
                total: money(61_235),
                doc_type: Some(DocumentType::RECEIPTS),
            },
        )
        .size(2, 240_000)
        .text("Bunnings Warehouse tax invoice INV88213 total 612.35"),
        Draft::unfiled(
            "Sonos_order_confirmation.pdf",
            Source::Emailed,
            day(-10),
            ExtractedFacts {
                merchant: Some("Sonos".to_string()),
                date: Some(day(-11)),
                total: money(49_900),
                doc_type: Some(DocumentType::RECEIPTS),
            },
        )
        .text("Sonos order confirmation Era 100 total 499.00"),
        Draft::unfiled(
            "origin-energy-bill.pdf",
            Source::Downloads,
            day(-14),
            ExtractedFacts {
                merchant: Some("Origin Energy".to_string()),
                date: Some(day(-16)),
                total: money(31_240),
                doc_type: Some(DocumentType::BILLS),
            },
        )
        .size(3, 300_000)
        .text("Origin Energy electricity bill amount due 312.40"),
        Draft::unfiled(
            "IMG_4460.jpg",
            Source::Scanned,
            day(-6),
            ExtractedFacts {
                merchant: Some("KMART".to_string()),
                // No date read, so the date Signal can never agree: payee alone, Weak.
                date: None,
                total: money(4_100),
                doc_type: None,
            },
        )
        .size(1, 980_000),
        Draft::unfiled(
            "ATO_NOA_2026.pdf",
            Source::Emailed,
            day(-18),
            ExtractedFacts {
                merchant: Some("Australian Taxation Office".to_string()),
                date: Some(day(-18)),
                total: money(128_400),
                doc_type: Some(DocumentType::TAX),
            },
        )
        .size(4, 150_000)
        .text("Australian Taxation Office notice of assessment refund 1,284.00"),
        Draft::unfiled(
            "scan0012.pdf",
            Source::Scanned,
            day(-1),
            ExtractedFacts::default(),
        )
        .size(1, 90_000),
    ];
    drafts.extend(filler(&lookup, transactions, today));

    let documents = drafts
        .into_iter()
        .zip(1..)
        .map(|(draft, id)| draft.build(DocumentId::seed(id)))
        .collect();
    DocumentsSeed { documents }
}

/// How many monthly statements each banking Account gets.
const FILLER_STATEMENT_MONTHS: u32 = 18;

/// Filler so the facets carry weight: a monthly statement for each banking Account, and a receipt
/// for every fourth purchase of 100.00 or more older than the Inbox's window.
fn filler(
    lookup: &Lookup<'_>,
    transactions: &[DocumentTransaction],
    today: NaiveDate,
) -> Vec<Draft> {
    let mut drafts = Vec::new();
    let mut month = today.with_day(1).unwrap_or(today);
    for _ in 0..FILLER_STATEMENT_MONTHS {
        // The latest Amex statement is the hand-authored sample row.
        let issued = month;
        month = (month - Duration::days(1)).with_day(1).unwrap_or(month);
        let label = month.format("%b %Y");
        for name in ["ANZ Everyday", "ANZ Offset", "Amex Platinum"] {
            if issued == today.with_day(1).unwrap_or(today) && name == "Amex Platinum" {
                continue;
            }
            let slug = name.to_lowercase().replace(' ', "-");
            drafts.push(
                Draft::filed(
                    &format!("{slug}-{}.pdf", month.format("%Y-%m")),
                    &format!("{name} statement — {label}"),
                    DocumentType::STATEMENTS,
                    // The month's last day, so the hand-authored sample (issued the 1st) leads.
                    issued - Duration::days(1),
                )
                .size(4, 350_000)
                .link(lookup.account(name).map(DocumentLink::Account)),
            );
        }
    }

    let cutoff = today - Duration::days(30);
    let hundred = cents_money(-10_000).0;
    let purchases = transactions
        .iter()
        .filter(|t| t.date < cutoff && t.amount.0 <= hundred)
        .filter(|t| t.first_payee().is_some())
        .filter(|t| t.id.seed_number() % 4 == 0);
    for purchase in purchases {
        let Some(name) = purchase
            .first_payee()
            .and_then(|id| lookup.records.payee(id))
            .map(|payee| payee.name.clone())
        else {
            continue;
        };
        let slug = name.to_lowercase().replace(' ', "-");
        drafts.push(
            Draft::filed(
                &format!("receipt-{slug}-{}.pdf", purchase.id.seed_number()),
                &format!("Receipt — {name}"),
                DocumentType::RECEIPTS,
                purchase.date,
            )
            .size(1, 120_000)
            .link(Some(DocumentLink::Transaction(purchase.id))),
        );
    }
    drafts
}
