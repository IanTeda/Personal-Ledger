//! The detail pane the Units page docks at X-Large (DESKTOP-125, DESKTOP-140): a selected Unit's
//! UNIT, PRICE SOURCE and ACCOUNTS groups, or a selected Price Source's PRICE SOURCE and VALUES,
//! then ACTIVITY. Built only while docked, as part of the page's view model.

use chrono::{DateTime, Utc};
use lib_core::{Money, UnitId};
use lib_locale::Label as _;
use lib_locale::format::{AmountStyle, format_amount};

use super::units_page::{ActivityLine, MenuAction, Row, SourceKind, UnitsPageEvent, UnitsRow};
use super::{ago, count, joined, updates_label};
use crate::protocols::UnitsPorts;
use crate::{MANUAL_ENTRY, PriceSource, PriceSourceAuth, SourceOrigin, UnitRow, last_fetched, msg};

/// The ✗ before LAST UPDATED for a source that hasn't fetched yet. A glyph, not a Message; both
/// Clients draw it from here.
pub const NOT_FETCHED_GLYPH: &str = "\u{2717}";

/// The pane's groups in drawn order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailPane {
    pub groups: Vec<PaneGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneGroup {
    Unit(UnitGroup),
    Source(SourceGroup),
    Accounts(AccountsGroup),
    Values(ValuesGroup),
    Activity(ActivityGroup),
}

impl PaneGroup {
    /// A stable name for the group, for a Client's element ids and tests.
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Unit(_) => "unit",
            Self::Source(_) => "source",
            Self::Accounts(_) => "accounts",
            Self::Values(_) => "values",
            Self::Activity(_) => "activity",
        }
    }
}

/// One fact row: a label over a right-aligned value, monospace for a URL part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub label: String,
    pub value: String,
    pub mono: bool,
}

impl Fact {
    pub(super) fn plain(label: String, value: String) -> Self {
        Self {
            label,
            value,
            mono: false,
        }
    }

    fn mono(label: String, value: String) -> Self {
        Self {
            label,
            value,
            mono: true,
        }
    }
}

/// A pane button and the event its click sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneButton {
    /// A stable name (`edit-unit`, `add-source`), for a Client's element ids and tests.
    pub slug: &'static str,
    pub label: String,
    pub event: UnitsPageEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitGroup {
    pub heading: String,
    /// `btc · Bitcoin`.
    pub title: String,
    /// `Crypto · valued in aud`.
    pub meta: String,
    /// FLAGS and Used by.
    pub facts: Vec<Fact>,
    pub buttons: Vec<PaneButton>,
}

/// PRICE SOURCE: a selected Unit's linked source (no title), or a selected Price Source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceGroup {
    pub heading: String,
    /// The selected source's name; `None` for a selected Unit's link.
    pub title: Option<String>,
    /// `built-in · prices Crypto · key not needed`; `None` for Manual entry and a Unit's link.
    pub meta: Option<String>,
    pub facts: Vec<Fact>,
    pub note: Option<String>,
    pub buttons: Vec<PaneButton>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountsGroup {
    pub heading: String,
    pub rows: Vec<AccountLine>,
    /// The bold Total closing the list; `None` with no Accounts.
    pub total: Option<AccountLine>,
    /// What shows with no Accounts.
    pub empty: Option<String>,
}

/// An Account held in the Unit: name over type, the balance over its value in the quote Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountLine {
    pub name: String,
    pub kind: String,
    /// `0.4120 btc`.
    pub balance: String,
    /// The balance at the Unit's last price, `61,063.11 aud`; `None` without a price.
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuesGroup {
    pub heading: String,
    pub rows: Vec<ValueLine>,
    pub note: Option<String>,
    /// What shows while no Unit uses the source.
    pub empty: Option<String>,
}

/// A Unit the source prices: `code · name` over its symbol and quote, its last price over its age.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueLine {
    pub code: String,
    pub name: String,
    /// What the source calls the Unit, with its label; `None` for a Unit priced by hand.
    pub symbol: Option<(String, String)>,
    /// `quoted in aud`.
    pub quoted_in: String,
    /// The symbol, then the quote: `symbol bitcoin · quoted in aud`.
    pub detail: String,
    pub price: String,
    pub age: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityGroup {
    pub heading: String,
    pub lines: Vec<ActivityLine>,
    pub note: String,
}

/// What the pane reads, gathered once per view model.
pub(super) struct PaneInput<'a> {
    pub ports: &'a dyn UnitsPorts,
    pub units: &'a [UnitRow],
    pub sources: &'a [PriceSource<'a>],
    pub base: UnitId,
    pub selection: Option<UnitsRow>,
    pub activity: &'a [ActivityLine],
    pub now: DateTime<Utc>,
}

pub(super) fn detail_pane(input: &PaneInput<'_>) -> DetailPane {
    let mut groups = match input.selection {
        Some(UnitsRow::Unit(id)) => match input.units.iter().find(|unit| unit.id == id) {
            Some(unit) => vec![
                PaneGroup::Unit(unit_group(input, unit)),
                PaneGroup::Source(linked_source(input, unit)),
                PaneGroup::Accounts(accounts_group(input, unit)),
            ],
            None => Vec::new(),
        },
        Some(UnitsRow::Source(id)) => match input.sources.iter().find(|source| source.id == id) {
            Some(source) => vec![
                PaneGroup::Source(source_group(input, source)),
                PaneGroup::Values(values_group(input, source)),
            ],
            None => Vec::new(),
        },
        None => Vec::new(),
    };
    groups.push(PaneGroup::Activity(ActivityGroup {
        heading: msg::units_pane_activity(),
        lines: input.activity.to_vec(),
        note: msg::units_pane_activity_note(),
    }));
    DetailPane { groups }
}

fn unit_of(units: &[UnitRow], id: UnitId) -> Option<&UnitRow> {
    units.iter().find(|unit| unit.id == id)
}

fn code_of(units: &[UnitRow], id: UnitId) -> String {
    unit_of(units, id)
        .map(|unit| unit.code.clone())
        .unwrap_or_default()
}

/// An amount at the Precision of the Unit it is in, followed by that Unit's code:
/// `148,212.40 aud`. A Unit that is gone, a quote mid-delete, leaves the amount at its own places.
pub(super) fn amount(money: &Money, unit: Option<&UnitRow>) -> String {
    let (style, code) = unit.map_or((AmountStyle::own(), ""), |unit| {
        (
            AmountStyle::fixed(unit.precision.places()),
            unit.code.as_str(),
        )
    });
    format!("{} {code}", format_amount(money, style).text)
}

fn not_fetched() -> String {
    format!("{NOT_FETCHED_GLYPH} {}", msg::units_not_fetched_yet())
}

/// The buttons for `row`'s actions, each sent as an action on that row.
fn buttons(row: Row<'_>, target: UnitsRow) -> Vec<PaneButton> {
    MenuAction::for_row(row)
        .iter()
        .map(|action| PaneButton {
            slug: action.slug(),
            label: action.pane_label(),
            event: UnitsPageEvent::Action(*action, target),
        })
        .collect()
}

fn unit_group(input: &PaneInput<'_>, unit: &UnitRow) -> UnitGroup {
    let holdings = input.ports.holdings().holdings(unit.id);
    let transactions: u32 = holdings.iter().map(|holding| holding.transactions).sum();
    let flags = if unit.id == input.base {
        msg::units_flag_base()
    } else if !unit.is_active {
        msg::units_inactive()
    } else {
        msg::units_none()
    };
    let base = Some(code_of(input.units, input.base))
        .filter(|code| !code.is_empty())
        .unwrap_or_else(|| unit.code.clone());
    UnitGroup {
        heading: msg::units_pane_unit(),
        title: msg::units_pane_title(&unit.code, &unit.name),
        meta: msg::units_pane_meta(&unit.kind.label(), &base),
        facts: vec![
            Fact::plain(msg::units_column_flags(), flags),
            Fact::plain(
                msg::units_pane_used_by(),
                msg::units_pane_used_by_value(count(holdings.len()), i64::from(transactions)),
            ),
        ],
        buttons: buttons(Row::Unit(unit), UnitsRow::Unit(unit.id)),
    }
}

/// A selected Unit's PRICE SOURCE: the source its link names with the Unit's own last price, or
/// while it sits on Manual entry the last price set by hand and Add price source.
fn linked_source(input: &PaneInput<'_>, unit: &UnitRow) -> SourceGroup {
    let heading = msg::units_price_sources_heading();
    let quote = unit_of(input.units, unit.link.quote);
    let last_price = Fact::plain(
        msg::units_pane_last_price(),
        unit.last_price()
            .map_or_else(msg::units_none, |price| amount(&price.close, quote)),
    );
    let source = (unit.link.source_id != MANUAL_ENTRY)
        .then(|| {
            input
                .sources
                .iter()
                .find(|source| source.id == unit.link.source_id)
        })
        .flatten();
    let Some(source) = source else {
        return SourceGroup {
            heading,
            title: None,
            meta: None,
            facts: vec![
                Fact::plain(
                    msg::units_column_source(),
                    lib_locale::msg::price_source_manual_entry(),
                ),
                last_price,
                Fact::plain(
                    msg::units_column_last_updated(),
                    unit.last_price()
                        .map_or_else(msg::units_none, |price| ago(price.written_at, input.now)),
                ),
            ],
            note: None,
            buttons: vec![PaneButton {
                slug: "add-source",
                label: msg::units_pane_add_source(),
                event: UnitsPageEvent::NewPriceSource,
            }],
        };
    };
    let facts = vec![
        Fact::plain(msg::units_column_source(), source.name.to_string()),
        Fact::mono(
            msg::units_pane_url(),
            source
                .endpoint()
                .map_or_else(msg::units_none, |endpoint| endpoint.address.to_string()),
        ),
        Fact::plain(
            msg::units_price_source_field_updates(),
            updates_label(source.updates),
        ),
        last_price,
        Fact::plain(
            msg::units_column_last_updated(),
            unit.last_price()
                .map_or_else(not_fetched, |price| ago(price.written_at, input.now)),
        ),
    ];
    SourceGroup {
        heading,
        title: None,
        meta: None,
        facts,
        note: None,
        buttons: buttons(
            Row::Source(source.id, SourceKind::of(source)),
            UnitsRow::Source(source.id),
        ),
    }
}

/// A selected Price Source's PRICE SOURCE: what its Edit Dialog shows, read-only. Manual entry has
/// only its title and note: it is neither edited nor deleted.
fn source_group(input: &PaneInput<'_>, source: &PriceSource<'_>) -> SourceGroup {
    let heading = msg::units_price_sources_heading();
    let kind = SourceKind::of(source);
    let Some(endpoint) = source.endpoint() else {
        return SourceGroup {
            heading,
            title: Some(source.name.to_string()),
            meta: None,
            facts: Vec::new(),
            note: Some(msg::units_pane_manual_note()),
            buttons: Vec::new(),
        };
    };
    let (origin, kinds, source_fact) = match source.origin {
        SourceOrigin::BuiltIn(built_in) => (
            msg::units_pane_origin_built_in(),
            built_in
                .kinds
                .iter()
                .map(lib_locale::Label::label)
                .collect::<Vec<_>>()
                .join(", "),
            msg::units_price_source_built_in(source.name),
        ),
        // A Custom source declares no Kinds, so it is offered for every one.
        SourceOrigin::Custom(_) | SourceOrigin::ManualEntry => (
            msg::units_pane_origin_custom(),
            msg::units_pane_every_kind(),
            msg::units_pane_source_custom(source.name),
        ),
    };
    let key = match endpoint.auth {
        PriceSourceAuth::None => msg::units_pane_key_not_needed(),
        PriceSourceAuth::ApiKey { .. } => msg::units_pane_key_needed(),
    };
    let mut facts = vec![
        Fact::plain(msg::units_column_source(), source_fact),
        Fact::mono(
            msg::units_price_source_field_address(),
            endpoint.address.to_string(),
        ),
        Fact::mono(
            msg::units_price_source_field_request(),
            endpoint.request.to_string(),
        ),
        Fact::mono(
            msg::units_price_source_field_price_path(),
            endpoint.price_path.to_string(),
        ),
    ];
    let custom = kind == SourceKind::Custom;
    if custom {
        // The key parameter's name, never the key: the key is never stored (ADR-0043).
        let (auth, key_param) = match &endpoint.auth {
            PriceSourceAuth::None => (msg::units_price_source_auth_none(), msg::units_none()),
            PriceSourceAuth::ApiKey { name, .. } => {
                (msg::units_price_source_auth_api_key(), name.to_string())
            }
        };
        facts.push(Fact::plain(msg::units_price_source_field_auth(), auth));
        facts.push(Fact::mono(
            msg::units_price_source_field_key_param(),
            key_param,
        ));
    }
    facts.push(Fact::plain(
        msg::units_price_source_field_updates(),
        updates_label(source.updates),
    ));
    facts.push(Fact::plain(
        msg::units_column_last_updated(),
        last_fetched(input.units, source.id).map_or_else(not_fetched, |at| ago(at, input.now)),
    ));
    SourceGroup {
        heading,
        title: Some(source.name.to_string()),
        meta: Some(msg::units_pane_source_meta(&origin, &kinds, &key)),
        facts,
        note: (!custom).then(msg::units_price_source_built_in_help),
        buttons: buttons(Row::Source(source.id, kind), UnitsRow::Source(source.id)),
    }
}

/// The Units `source` prices.
fn values_group(input: &PaneInput<'_>, source: &PriceSource<'_>) -> ValuesGroup {
    let manual = source.id == MANUAL_ENTRY;
    let rows: Vec<ValueLine> = input
        .units
        .iter()
        .filter(|unit| unit.link.source_id == source.id)
        .map(|unit| {
            let quote = unit_of(input.units, unit.link.quote);
            let symbol = (!unit.link.symbol.is_empty())
                .then(|| (msg::units_pane_symbol(), unit.link.symbol.clone()));
            let quoted_in = msg::units_pane_quoted_in(&code_of(input.units, unit.link.quote));
            ValueLine {
                code: unit.code.clone(),
                name: unit.name.clone(),
                detail: joined(
                    symbol
                        .iter()
                        .map(|(label, symbol)| msg::units_pane_symbol_value(label, symbol))
                        .chain([quoted_in.clone()]),
                ),
                symbol,
                quoted_in,
                price: unit
                    .last_price()
                    .map_or_else(msg::units_none, |price| amount(&price.close, quote)),
                age: match unit.last_price() {
                    Some(price) => Some(ago(price.written_at, input.now)),
                    None if manual => None,
                    None => Some(not_fetched()),
                },
            }
        })
        .collect();
    let note = (!rows.is_empty() && !manual).then(|| {
        if SourceKind::of(source) == SourceKind::Custom {
            msg::units_pane_values_note_custom()
        } else {
            msg::units_pane_values_note()
        }
    });
    ValuesGroup {
        heading: msg::units_pane_values(),
        empty: rows.is_empty().then(msg::units_pane_values_none),
        rows,
        note,
    }
}

/// The Accounts held in `unit`, each valued at the Unit's last price in its quote Unit.
fn accounts_group(input: &PaneInput<'_>, unit: &UnitRow) -> AccountsGroup {
    let holdings = input.ports.holdings().holdings(unit.id);
    let quote = unit_of(input.units, unit.link.quote);
    let line = |name: String, kind: String, balance: &Money| AccountLine {
        name,
        kind,
        balance: amount(balance, Some(unit)),
        value: unit.last_price().map(|price| {
            let precision = quote.map_or(unit.precision, |quote| quote.precision);
            amount(&price.value(balance, precision), quote)
        }),
    };
    let total = (!holdings.is_empty()).then(|| {
        let sum = Money(
            holdings
                .iter()
                .map(|holding| holding.balance.0.clone())
                .sum(),
        );
        line(msg::units_pane_accounts_total(), String::new(), &sum)
    });
    AccountsGroup {
        heading: msg::units_pane_accounts(),
        empty: holdings.is_empty().then(msg::units_pane_accounts_none),
        rows: holdings
            .into_iter()
            .map(|holding| line(holding.name, holding.kind.label(), &holding.balance))
            .collect(),
        total,
    }
}
