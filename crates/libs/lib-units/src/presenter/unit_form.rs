//! The Add unit and Edit unit Dialogs: one form, Edit pre-filled from the row.
//!
//! The Source select offers only the sources able to price the chosen Type, so changing Type
//! re-filters it and falls back to Manual entry when the chosen source no longer fits. Manual entry
//! has no Symbol or Quoted in to fill, so those fields are hidden and `Tab` skips them. The options
//! are copied when the Dialog opens, as the Dialog is modal and nothing changes them under it.

use lib_core::{PriceSourceId, UnitId, UnitKind};
use lib_locale::Label as _;

use super::{Choice, DialogFrame, SelectView, TextFieldView, UnitsOutcome, cycle, unique_label};
use crate::interactor::{AddUnit, EditUnit, EditUnitRefusal, PriceSourceGone};
use crate::protocols::{SharedPorts, UnitsRouter};
use crate::{MANUAL_ENTRY, PriceLink, msg, price_sources};

/// The form's fields, in `Tab` order: the mockup's reading order, Code and Type, Name, then the
/// PRICE SOURCE group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnitField {
    #[default]
    Code,
    Type,
    Name,
    Source,
    Symbol,
    Quote,
}

impl UnitField {
    const ORDER: [Self; 6] = [
        Self::Code,
        Self::Type,
        Self::Name,
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
    source: PriceSourceId,
    symbol: String,
    quote: Option<UnitId>,
    focused: UnitField,
    sources: Vec<SourceOption>,
    quotes: Vec<Choice<UnitId>>,
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
        form.source = unit.link.source_id;
        form.symbol = unit.link.symbol;
        Some(form)
    }

    fn blank(ports: SharedPorts) -> Self {
        let (sources, quotes) = {
            let ports = ports.borrow();
            let records = ports.price_sources().price_source_records();
            let manual = lib_locale::msg::price_source_manual_entry();
            // Manual entry is labelled first so a Custom source named the same takes the suffix.
            let mut seen = vec![manual.clone()];
            let sources = price_sources(&records, &manual)
                .iter()
                .map(|source| SourceOption {
                    id: source.id,
                    label: if source.id == MANUAL_ENTRY {
                        manual.clone()
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
            source: MANUAL_ENTRY,
            symbol: String::new(),
            quote: None,
            focused: UnitField::default(),
            sources,
            quotes,
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
            UnitFormEvent::Kind(kind) => {
                self.kind = kind;
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

    /// Code and Name are required, and a source that fetches needs a Unit to quote in. Type always
    /// has a value, so it never blocks.
    fn is_valid(&self) -> bool {
        !self.code.trim().is_empty()
            && !self.name.trim().is_empty()
            && (self.is_manual() || self.quote.is_some())
    }

    /// The link the form describes. A Manual entry link keeps no Symbol and never reads its
    /// quote, so with no Units at all it gets an id naming none.
    fn link(&self) -> PriceLink {
        let quote = self.quote.unwrap_or_else(UnitId::new);
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
        if !self.is_valid() {
            return;
        }
        let (code, name, kind, link) =
            (self.code.clone(), self.name.clone(), self.kind, self.link());
        let saved = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            match self.editing {
                None => AddUnit {
                    code,
                    name,
                    kind,
                    link,
                }
                .run(ports.units, ports.price_sources)
                .map_err(|PriceSourceGone| msg::units_refusal_price_source_gone()),
                Some(id) => EditUnit {
                    id,
                    code,
                    name,
                    kind,
                    link,
                }
                .run(ports.units, ports.price_sources)
                .map_err(|refusal| match refusal {
                    EditUnitRefusal::UnitGone => msg::units_refusal_unit_gone(),
                    EditUnitRefusal::PriceSourceGone => msg::units_refusal_price_source_gone(),
                }),
            }
        };
        match saved {
            Ok(unit) => {
                self.refusal = None;
                router.close_dialog(Some(UnitsOutcome::Saved { code: unit.code }));
            }
            Err(text) => self.refusal = Some(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
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
            [RouterCall::CloseDialog(Some(UnitsOutcome::Saved {
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
            [RouterCall::CloseDialog(Some(UnitsOutcome::Saved {
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
    fn cancel_closes_with_no_outcome() {
        let (_, ports) = stub();
        let mut form = UnitFormPresenter::add(ports);
        assert_eq!(
            send(&mut form, vec![UnitFormEvent::Cancel]),
            [RouterCall::CloseDialog(None)]
        );
    }
}
