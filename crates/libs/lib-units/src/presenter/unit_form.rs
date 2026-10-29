//! The Add unit and Edit unit Dialogs: one form, Edit pre-filled from the row.
//!
//! The Source select offers only the sources able to price the chosen Type, so changing Type
//! re-filters it and falls back to Manual entry when the chosen source no longer fits. Add starts
//! Precision on the Type's and follows Type until a Precision is typed; Edit never re-defaults it. Manual entry
//! has no Symbol or Quoted in to fill, so those fields are hidden and `Tab` skips them. The options
//! are copied when the Dialog opens, as the Dialog is modal and nothing changes them under it.

use lib_core::{PriceSourceId, UnitId, UnitKind};
use lib_locale::Label as _;
use lib_viper::view::{Choice, DialogFrame, SelectView, TextFieldView};

use super::{UnitChange, UnitsOutcome, cycle, unique_label, uses_text};
use crate::interactor::{AddUnit, EditUnit};
use crate::protocols::{SharedPorts, UnitsRouter};
use crate::{Error, MANUAL_ENTRY, Precision, PriceLink, UnitUse, msg};

/// The form's fields, in `Tab` order: the mockup's reading order, Code and Type, Name and
/// Precision, then the PRICE SOURCE group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnitField {
    #[default]
    Code,
    Type,
    Name,
    Precision,
    Source,
    Symbol,
    Quote,
}

impl UnitField {
    const ORDER: [Self; 7] = [
        Self::Code,
        Self::Type,
        Self::Name,
        Self::Precision,
        Self::Source,
        Self::Symbol,
        Self::Quote,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitFormEvent {
    /// A text field's whole new text, as its View's input holds it.
    Code(String),
    Name(String),
    Precision(String),
    Symbol(String),
    Kind(UnitKind),
    Source(PriceSourceId),
    Quote(UnitId),
    /// `Tab`/`Shift-Tab`, skipping hidden fields and wrapping.
    NextField,
    PrevField,
    /// A click on a field. A hidden field can't take focus.
    Focus(UnitField),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitFormViewModel {
    pub frame: DialogFrame,
    pub code: TextFieldView,
    pub kind: SelectView<UnitKind>,
    pub name: TextFieldView,
    /// The decimal places, as typed: valid only as a whole number from 0 to 18.
    pub precision: TextFieldView,
    pub precision_help: String,
    /// The PRICE SOURCE group's heading.
    pub source_group: String,
    pub source: SelectView<PriceSourceId>,
    pub source_help: String,
    /// Symbol and Quoted in, hidden on Manual entry.
    pub symbol: Option<TextFieldView>,
    /// `CoinGecko's id for this unit.`
    pub symbol_help: Option<String>,
    pub quote: Option<SelectView<UnitId>>,
    /// The Test price button's label, shown beside a fetching source.
    pub test_price: Option<String>,
    /// The fields on screen, in `Tab` order.
    pub fields: Vec<UnitField>,
    /// What names the Unit, for Edit: `Used by 2 accounts, 1 budget.`; `None` for Add.
    pub usage: Option<String>,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
}

/// A Price Source as the Source select offers it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceOption {
    id: PriceSourceId,
    label: String,
    kinds: Vec<UnitKind>,
}

pub struct UnitFormPresenter {
    ports: SharedPorts,
    /// The Unit an Edit rewrites; `None` for Add.
    editing: Option<UnitId>,
    code: String,
    name: String,
    kind: UnitKind,
    /// The Precision as typed, so a half-typed one shows as it is.
    precision: String,
    /// Whether a person has typed a Precision, which stops Add following Type.
    precision_typed: bool,
    source: PriceSourceId,
    symbol: String,
    quote: Option<UnitId>,
    focused: UnitField,
    sources: Vec<SourceOption>,
    quotes: Vec<Choice<UnitId>>,
    /// What names the Unit being edited, read when the Dialog opens.
    usage: Option<UnitUse>,
    refusal: Option<String>,
}

impl UnitFormPresenter {
    /// An empty Add unit form: the first Kind, on Manual entry, quoted in the base Unit.
    pub fn add(ports: SharedPorts) -> Self {
        let mut form = Self::blank(ports);
        form.quote = form.base();
        form
    }

    /// The Edit unit form pre-filled from Unit `id`, or `None` when it is gone. Its source isn't
    /// re-filtered: a stored link whose source can't price the stored Kind stays as it was until
    /// Type changes, so opening and saving never rewrites it.
    pub fn edit(ports: SharedPorts, id: UnitId) -> Option<Self> {
        let unit = ports.borrow().units().unit(id)?;
        let mut form = Self::blank(ports);
        let quote = form
            .quotes
            .iter()
            .any(|choice| choice.value == unit.link.quote)
            .then_some(unit.link.quote);
        form.quote = quote.or_else(|| form.base());
        form.editing = Some(id);
        form.code = unit.code;
        form.name = unit.name;
        form.kind = unit.kind;
        form.precision = unit.precision.to_string();
        form.source = unit.link.source_id;
        form.symbol = unit.link.symbol;
        form.usage = Some(form.ports.borrow().usage().unit_usage(id));
        Some(form)
    }

    fn blank(ports: SharedPorts) -> Self {
        let (sources, quotes) = {
            let ports = ports.borrow();
            let catalogue = ports.source_catalogue();
            let manual = catalogue.manual_name();
            // Manual entry is labelled first so a Custom source named the same takes the suffix.
            let mut seen = vec![manual.to_string()];
            let sources = catalogue
                .sources()
                .iter()
                .map(|source| SourceOption {
                    id: source.id,
                    label: if source.id == MANUAL_ENTRY {
                        manual.to_string()
                    } else {
                        unique_label(source.name, &mut seen)
                    },
                    kinds: UnitKind::all()
                        .iter()
                        .copied()
                        .filter(|kind| source.prices(*kind))
                        .collect(),
                })
                .collect();
            let base = ports.base_unit().base_unit();
            let mut seen = Vec::new();
            let quotes = ports
                .units()
                .units()
                .iter()
                .map(|unit| {
                    let label = if unit.id == base {
                        msg::units_quote_base(&unit.code)
                    } else {
                        unit.code.clone()
                    };
                    Choice {
                        value: unit.id,
                        label: unique_label(&label, &mut seen),
                    }
                })
                .collect();
            (sources, quotes)
        };
        Self {
            ports,
            editing: None,
            code: String::new(),
            name: String::new(),
            kind: UnitKind::default(),
            precision: Precision::of_kind(UnitKind::default()).to_string(),
            precision_typed: false,
            source: MANUAL_ENTRY,
            symbol: String::new(),
            quote: None,
            focused: UnitField::default(),
            sources,
            quotes,
            usage: None,
            refusal: None,
        }
    }

    /// The Unit a new link is quoted in: the base Unit, or the first while the base is gone.
    fn base(&self) -> Option<UnitId> {
        let base = self.ports.borrow().base_unit().base_unit();
        self.quotes
            .iter()
            .find(|choice| choice.value == base)
            .or(self.quotes.first())
            .map(|choice| choice.value)
    }

    pub fn handle(&mut self, event: UnitFormEvent, router: &mut dyn UnitsRouter) {
        match event {
            UnitFormEvent::Code(text) => self.code = text,
            UnitFormEvent::Name(text) => self.name = text,
            UnitFormEvent::Symbol(text) => self.symbol = text,
            UnitFormEvent::Precision(text) => {
                self.precision = text;
                self.precision_typed = true;
            }
            UnitFormEvent::Kind(kind) => {
                self.kind = kind;
                if self.editing.is_none() && !self.precision_typed {
                    self.precision = Precision::of_kind(kind).to_string();
                }
                self.sync_source();
            }
            UnitFormEvent::Source(id) => {
                if self.sources_for_kind().any(|source| source.id == id) {
                    self.source = id;
                    self.sync_source();
                }
            }
            UnitFormEvent::Quote(id) => {
                if self.quotes.iter().any(|choice| choice.value == id) {
                    self.quote = Some(id);
                }
            }
            UnitFormEvent::NextField | UnitFormEvent::PrevField => {
                let backward = event == UnitFormEvent::PrevField;
                self.focused = cycle(&UnitField::ORDER, self.focused, backward, |field| {
                    self.shows(field)
                });
            }
            UnitFormEvent::Focus(field) => {
                if self.shows(field) {
                    self.focused = field;
                }
            }
            UnitFormEvent::Confirm => self.confirm(router),
            UnitFormEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> UnitFormViewModel {
        let text = |label, text: &str, placeholder, field| TextFieldView {
            label,
            text: text.to_string(),
            placeholder,
            focused: self.focused == field,
        };
        let fetching = !self.is_manual();
        let (title, submit) = match self.editing {
            Some(_) => (msg::units_edit_title(&self.code), msg::units_edit_submit()),
            None => (msg::units_add_title(), msg::units_add_submit()),
        };
        UnitFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.is_valid(),
                destructive: false,
                hints: super::form_hints(),
            },
            code: text(
                lib_locale::msg::column_code(),
                &self.code,
                msg::units_code_placeholder(),
                UnitField::Code,
            ),
            kind: SelectView {
                label: lib_locale::msg::column_type(),
                options: UnitKind::all()
                    .iter()
                    .map(|kind| Choice {
                        value: *kind,
                        label: kind.label(),
                    })
                    .collect(),
                chosen: Some(self.kind),
                focused: self.focused == UnitField::Type,
            },
            name: text(
                lib_locale::msg::column_name(),
                &self.name,
                msg::units_name_placeholder(),
                UnitField::Name,
            ),
            precision: text(
                msg::units_field_precision(),
                &self.precision,
                msg::units_precision_placeholder(&Precision::MAX.to_string()),
                UnitField::Precision,
            ),
            precision_help: msg::units_precision_help(),
            source_group: msg::units_group_source(),
            source: SelectView {
                label: msg::units_field_source(),
                options: self
                    .sources_for_kind()
                    .map(|source| Choice {
                        value: source.id,
                        label: source.label.clone(),
                    })
                    .collect(),
                chosen: Some(self.source),
                focused: self.focused == UnitField::Source,
            },
            source_help: msg::units_source_help(),
            symbol: fetching.then(|| {
                text(
                    msg::units_field_symbol(),
                    &self.symbol,
                    msg::units_symbol_placeholder(),
                    UnitField::Symbol,
                )
            }),
            symbol_help: fetching.then(|| msg::units_symbol_help(self.source_label())),
            quote: fetching.then(|| SelectView {
                label: msg::units_field_quote(),
                options: self.quotes.clone(),
                chosen: self.quote,
                focused: self.focused == UnitField::Quote,
            }),
            test_price: fetching.then(msg::units_test_price),
            fields: UnitField::ORDER
                .into_iter()
                .filter(|field| self.shows(*field))
                .collect(),
            usage: self.usage.map(|uses| {
                if uses.is_used() {
                    msg::units_edit_usage(&uses_text(uses))
                } else {
                    msg::units_edit_usage_none()
                }
            }),
            refusal: self.refusal.clone(),
        }
    }

    fn sources_for_kind(&self) -> impl Iterator<Item = &SourceOption> {
        self.sources
            .iter()
            .filter(|source| source.kinds.contains(&self.kind))
    }

    fn source_label(&self) -> &str {
        self.sources
            .iter()
            .find(|source| source.id == self.source)
            .map_or("", |source| source.label.as_str())
    }

    fn is_manual(&self) -> bool {
        self.source == MANUAL_ENTRY
    }

    /// Whether `field` is on screen: Manual entry hides Symbol and Quoted in.
    fn shows(&self, field: UnitField) -> bool {
        !(self.is_manual() && matches!(field, UnitField::Symbol | UnitField::Quote))
    }

    /// Falls back to Manual entry when the chosen source can't price the chosen Type, and moves
    /// focus off a field that fallback hid.
    fn sync_source(&mut self) {
        if !self
            .sources_for_kind()
            .any(|source| source.id == self.source)
        {
            self.source = MANUAL_ENTRY;
        }
        if !self.shows(self.focused) {
            self.focused = UnitField::Source;
        }
    }

    /// Code, Name and a Precision in range are required, and a source that fetches needs a Unit to
    /// quote in. Type always has a value, so it never blocks.
    fn is_valid(&self) -> bool {
        !self.code.trim().is_empty()
            && !self.name.trim().is_empty()
            && Precision::parse(&self.precision).is_some()
            && (self.is_manual() || self.quote.is_some())
    }

    /// The link the form describes for Unit `id`. A Manual entry link keeps no Symbol and never
    /// reads its quote, so with no Units at all the Unit is quoted in itself.
    fn link(&self, id: UnitId) -> PriceLink {
        let quote = self.quote.unwrap_or(id);
        if self.is_manual() {
            PriceLink::manual(quote)
        } else {
            PriceLink {
                source_id: self.source,
                symbol: self.symbol.trim().to_string(),
                quote,
            }
        }
    }

    fn confirm(&mut self, router: &mut dyn UnitsRouter) {
        let Some(precision) = Precision::parse(&self.precision).filter(|_| self.is_valid()) else {
            return;
        };
        let id = self.editing.unwrap_or_else(UnitId::new);
        let (code, name, kind, link) = (
            self.code.clone(),
            self.name.clone(),
            self.kind,
            self.link(id),
        );
        let saved = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            match self.editing {
                None => AddUnit {
                    id,
                    code,
                    name,
                    kind,
                    precision,
                    link,
                }
                .run(ports.units, ports.price_sources)
                .map_err(refusal_text),
                Some(_) => EditUnit {
                    id,
                    code,
                    name,
                    kind,
                    precision,
                    link,
                }
                .run(ports.units, ports.price_sources)
                .map_err(refusal_text),
            }
        };
        match saved {
            Ok(unit) => {
                self.refusal = None;
                router.close_dialog(Some(UnitsOutcome::Unit {
                    change: UnitChange::Saved,
                    code: unit.code,
                }));
            }
            Err(text) => self.refusal = Some(text),
        }
    }
}

/// The Message for an Add or Edit unit refusal. Add never returns `UnitGone`, but the Message is the
/// same either way, so both share this.
fn refusal_text(refusal: Error) -> String {
    match refusal {
        Error::UnitGone => msg::units_refusal_unit_gone(),
        Error::PriceSourceGone => msg::units_refusal_price_source_gone(),
        Error::QuoteGone => msg::units_refusal_quote_gone(),
        refusal => {
            debug_assert!(false, "AddUnit and EditUnit don't return {refusal:?}");
            msg::units_refusal_unit_gone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{hints, pairs, stub};
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::{COINGECKO, FRANKFURTER, YAHOO_FINANCE};

    fn send(form: &mut UnitFormPresenter, events: Vec<UnitFormEvent>) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            form.handle(event, &mut router);
        }
        router.calls
    }

    fn source_labels(form: &UnitFormPresenter) -> Vec<String> {
        form.view_model()
            .source
            .options
            .into_iter()
            .map(|choice| choice.label)
            .collect()
    }

    #[test]
    fn a_new_form_is_on_manual_entry_quoted_in_the_base_unit_and_hides_symbol_and_quote() {
        let (_, ports) = stub();
        let form = UnitFormPresenter::add(ports);
        let view = form.view_model();

        assert_eq!(view.frame.title, "Add unit");
        assert_eq!(view.source.chosen, Some(MANUAL_ENTRY));
        assert_eq!(view.symbol, None);
        assert_eq!(view.quote, None);
        assert_eq!(
            view.fields,
            [
                UnitField::Code,
                UnitField::Type,
                UnitField::Name,
                UnitField::Precision,
                UnitField::Source
            ]
        );
        assert!(!view.frame.can_confirm, "Code and Name are required");
    }

    #[test]
    fn the_source_list_holds_only_sources_that_price_the_kind() {
        let (_, ports) = stub();
        let mut form = UnitFormPresenter::add(ports);

        send(&mut form, vec![UnitFormEvent::Kind(UnitKind::Crypto)]);
        assert_eq!(source_labels(&form), ["CoinGecko", "Manual entry"]);
        send(&mut form, vec![UnitFormEvent::Kind(UnitKind::Currency)]);
        assert_eq!(source_labels(&form), ["Frankfurter", "Manual entry"]);
        send(&mut form, vec![UnitFormEvent::Source(COINGECKO)]);
        assert_eq!(
            form.view_model().source.chosen,
            Some(MANUAL_ENTRY),
            "a source that can't price the Kind is not taken"
        );
    }

    #[test]
    fn a_fetching_source_shows_symbol_and_quote_and_changing_type_falls_back_to_manual_entry() {
        let (_, ports) = stub();
        let mut form = UnitFormPresenter::add(ports);
        send(
            &mut form,
            vec![
                UnitFormEvent::Kind(UnitKind::Crypto),
                UnitFormEvent::Source(COINGECKO),
                UnitFormEvent::Focus(UnitField::Symbol),
            ],
        );
        let view = form.view_model();
        assert_eq!(
            view.symbol_help.as_deref(),
            Some("CoinGecko's id for this unit.")
        );
        let quote = view.quote.expect("a fetching source shows Quoted in");
        assert_eq!(quote.chosen, Some(UnitId::SEED_AUD));
        assert_eq!(quote.options[0].label, "aud (base unit)");

        send(&mut form, vec![UnitFormEvent::Kind(UnitKind::Equity)]);

        let view = form.view_model();
        assert_eq!(view.source.chosen, Some(MANUAL_ENTRY));
        assert_eq!(view.symbol, None);
        assert!(
            view.source.focused,
            "focus leaves the hidden Symbol for Source"
        );
        assert!(source_labels(&form).contains(&"Yahoo Finance".to_string()));
    }

    #[test]
    fn tab_skips_fields_manual_entry_hides_and_wraps() {
        let (_, ports) = stub();
        let mut form = UnitFormPresenter::add(ports);
        send(&mut form, vec![UnitFormEvent::Focus(UnitField::Symbol)]);
        assert!(
            form.view_model().code.focused,
            "a hidden field can't take focus"
        );

        send(
            &mut form,
            vec![
                UnitFormEvent::Focus(UnitField::Source),
                UnitFormEvent::NextField,
            ],
        );
        assert!(form.view_model().code.focused);
        send(&mut form, vec![UnitFormEvent::PrevField]);
        assert!(form.view_model().source.focused);
    }

    #[test]
    fn adding_a_unit_saves_its_link_and_closes_with_its_code() {
        let (ports, shared) = stub();
        let mut form = UnitFormPresenter::add(shared);

        let calls = send(
            &mut form,
            vec![
                UnitFormEvent::Code("eth".into()),
                UnitFormEvent::Name("Ethereum".into()),
                UnitFormEvent::Kind(UnitKind::Crypto),
                UnitFormEvent::Source(COINGECKO),
                UnitFormEvent::Symbol(" ethereum ".into()),
                UnitFormEvent::Confirm,
            ],
        );

        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::Unit {
                change: UnitChange::Saved,
                code: "eth".into()
            }))]
        );
        let ports = ports.borrow();
        let added = ports.units.units.last().expect("eth was added");
        assert_eq!(
            added.link,
            PriceLink {
                source_id: COINGECKO,
                symbol: "ethereum".into(),
                quote: UnitId::SEED_AUD,
            }
        );
    }

    #[test]
    fn an_invalid_form_does_not_confirm() {
        let (ports, shared) = stub();
        let mut form = UnitFormPresenter::add(shared);

        let calls = send(
            &mut form,
            vec![
                UnitFormEvent::Code("eth".into()),
                UnitFormEvent::Name("  ".into()),
                UnitFormEvent::Confirm,
            ],
        );

        assert_eq!(calls, []);
        assert_eq!(ports.borrow().units.units.len(), 3);
    }

    #[test]
    fn editing_shows_the_real_usage_counts_and_adding_shows_none() {
        let (ports, shared) = stub();
        let form = UnitFormPresenter::edit(shared.clone(), UnitId::SEED_BTC).unwrap();
        assert_eq!(
            form.view_model().usage.as_deref(),
            Some("Nothing uses this unit yet.")
        );

        ports.borrow_mut().usage.uses.insert(
            UnitId::SEED_BTC,
            UnitUse {
                accounts: 2,
                budgets: 1,
                ..UnitUse::default()
            },
        );
        let form = UnitFormPresenter::edit(shared.clone(), UnitId::SEED_BTC).unwrap();
        assert_eq!(
            form.view_model().usage.as_deref(),
            Some("Used by 2 accounts, 1 budget.")
        );
        assert_eq!(UnitFormPresenter::add(shared).view_model().usage, None);
    }

    #[test]
    fn editing_prefills_the_row_keeps_an_unfitting_stored_source_and_saves_under_its_id() {
        let (ports, shared) = stub();
        ports.borrow_mut().units.units[1].kind = UnitKind::Currency;
        let mut form = UnitFormPresenter::edit(shared, UnitId::SEED_BTC).expect("btc is seeded");
        let view = form.view_model();
        assert_eq!(view.frame.title, "Edit unit — btc");
        assert_eq!(view.code.text, "btc");
        assert_eq!(
            view.source.chosen,
            Some(COINGECKO),
            "opening doesn't rewrite the link"
        );

        let calls = send(
            &mut form,
            vec![UnitFormEvent::Code("xbt".into()), UnitFormEvent::Confirm],
        );

        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::Unit {
                change: UnitChange::Saved,
                code: "xbt".into()
            }))]
        );
        let edited = ports.borrow().units.units[1].clone();
        assert_eq!(
            (edited.code.as_str(), edited.link.source_id),
            ("xbt", COINGECKO)
        );
    }

    #[test]
    fn a_unit_or_source_removed_under_the_dialog_is_refused_as_text_and_it_stays_open() {
        let (ports, shared) = stub();
        let mut edit = UnitFormPresenter::edit(shared.clone(), UnitId::SEED_VAS).unwrap();
        ports.borrow_mut().units.units.remove(2);
        assert_eq!(send(&mut edit, vec![UnitFormEvent::Confirm]), []);
        assert_eq!(
            edit.view_model().refusal.as_deref(),
            Some("This unit was removed while the dialog was open.")
        );

        let custom = crate::interactor::AddPriceSource {
            name: "Broker".into(),
            endpoint: crate::built_in(YAHOO_FINANCE).unwrap().endpoint.clone(),
            updates: crate::Updates::Daily,
        }
        .run(&mut ports.borrow_mut().price_sources);
        let mut add = UnitFormPresenter::add(shared);
        send(
            &mut add,
            vec![
                UnitFormEvent::Code("x".into()),
                UnitFormEvent::Name("x".into()),
                UnitFormEvent::Source(custom.id),
            ],
        );
        ports.borrow_mut().price_sources.records.clear();
        assert_eq!(send(&mut add, vec![UnitFormEvent::Confirm]), []);
        assert_eq!(
            add.view_model().refusal.as_deref(),
            Some("That price source was removed while the dialog was open.")
        );
    }

    #[test]
    fn a_fetching_source_needs_a_quote_unit_to_be_valid() {
        let (ports, shared) = stub();
        ports.borrow_mut().units.units.clear();
        let mut form = UnitFormPresenter::add(shared);
        send(
            &mut form,
            vec![
                UnitFormEvent::Code("x".into()),
                UnitFormEvent::Name("x".into()),
                UnitFormEvent::Kind(UnitKind::Currency),
            ],
        );
        assert!(
            form.view_model().frame.can_confirm,
            "Manual entry never reads its quote"
        );

        send(&mut form, vec![UnitFormEvent::Source(FRANKFURTER)]);
        assert!(!form.view_model().frame.can_confirm);
    }

    #[test]
    fn a_quote_unit_removed_under_the_dialog_is_refused_as_text_and_nothing_is_written() {
        let (ports, shared) = stub();
        let mut add = UnitFormPresenter::add(shared.clone());
        send(
            &mut add,
            vec![
                UnitFormEvent::Code("x".into()),
                UnitFormEvent::Name("x".into()),
            ],
        );
        let mut edit = UnitFormPresenter::edit(shared, UnitId::SEED_BTC).unwrap();
        ports.borrow_mut().units.units.remove(0);
        let before = ports.borrow().clone();

        for form in [&mut add, &mut edit] {
            assert_eq!(send(form, vec![UnitFormEvent::Confirm]), []);
            assert_eq!(
                form.view_model().refusal.as_deref(),
                Some("The unit it's quoted in was removed while the dialog was open."),
                "aud, the quote, is gone"
            );
        }
        assert_eq!(*ports.borrow(), before, "a refused save writes nothing");
    }

    #[test]
    fn the_first_unit_on_an_empty_ledger_is_quoted_in_itself() {
        let (ports, shared) = stub();
        ports.borrow_mut().units.units.clear();
        let mut form = UnitFormPresenter::add(shared);
        send(
            &mut form,
            vec![
                UnitFormEvent::Code("x".into()),
                UnitFormEvent::Name("x".into()),
                UnitFormEvent::Confirm,
            ],
        );

        let added = ports.borrow().units.units.clone();
        assert_eq!(added.len(), 1, "{:?}", form.view_model().refusal);
        assert_eq!(added[0].link.quote, added[0].id);
    }

    #[test]
    fn a_new_units_precision_follows_its_type_until_one_is_typed() {
        let (_, ports) = stub();
        let mut form = UnitFormPresenter::add(ports);
        assert_eq!(form.view_model().precision.text, "2", "Currency's");

        send(&mut form, vec![UnitFormEvent::Kind(UnitKind::Crypto)]);
        assert_eq!(form.view_model().precision.text, "8");

        send(
            &mut form,
            vec![
                UnitFormEvent::Precision("6".into()),
                UnitFormEvent::Kind(UnitKind::Currency),
            ],
        );
        assert_eq!(
            form.view_model().precision.text,
            "6",
            "a typed Precision stays"
        );
    }

    #[test]
    fn editing_prefills_the_precision_and_a_type_change_keeps_it() {
        let (_, shared) = stub();
        let mut form = UnitFormPresenter::edit(shared, UnitId::SEED_AUD).expect("aud is seeded");
        assert_eq!(form.view_model().precision.text, "2");

        send(&mut form, vec![UnitFormEvent::Kind(UnitKind::Crypto)]);

        assert_eq!(form.view_model().precision.text, "2");
    }

    #[test]
    fn precision_sits_after_name_in_tab_order() {
        let (_, ports) = stub();
        let mut form = UnitFormPresenter::add(ports);
        send(
            &mut form,
            vec![
                UnitFormEvent::Focus(UnitField::Name),
                UnitFormEvent::NextField,
            ],
        );
        let view = form.view_model();
        assert!(view.precision.focused);
        assert_eq!(
            view.fields,
            [
                UnitField::Code,
                UnitField::Type,
                UnitField::Name,
                UnitField::Precision,
                UnitField::Source
            ]
        );
    }

    #[test]
    fn a_precision_outside_0_to_18_does_not_confirm_and_one_inside_is_saved() {
        let (ports, shared) = stub();
        let mut form = UnitFormPresenter::edit(shared, UnitId::SEED_BTC).expect("btc is seeded");

        for refused in ["", "19", "2.5"] {
            let calls = send(
                &mut form,
                vec![
                    UnitFormEvent::Precision(refused.into()),
                    UnitFormEvent::Confirm,
                ],
            );
            assert_eq!(calls, [], "{refused:?}");
            assert!(!form.view_model().frame.can_confirm, "{refused:?}");
        }
        send(
            &mut form,
            vec![
                UnitFormEvent::Precision("18".into()),
                UnitFormEvent::Confirm,
            ],
        );

        assert_eq!(
            ports.borrow().units.units[1].precision,
            Precision::new(18).expect("in range")
        );
    }

    #[test]
    fn cancel_closes_with_no_outcome() {
        let (_, ports) = stub();
        let mut form = UnitFormPresenter::add(ports);
        assert_eq!(
            send(&mut form, vec![UnitFormEvent::Cancel]),
            [RouterCall::CloseDialog(None)]
        );
    }

    #[test]
    fn the_form_takes_next_field_choose_and_cancel() {
        let (_, ports) = stub();
        let view = UnitFormPresenter::add(ports).view_model();

        assert_eq!(
            hints(&view.frame),
            pairs(&[
                ("next_field", "next field"),
                ("select_previous/select_next", "choose"),
                ("back", "cancel")
            ])
        );
    }
}
