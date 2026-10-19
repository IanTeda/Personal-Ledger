//! The Add, Edit and Delete unit dialogs' live form state -- pure, `gpui`-free; `Shell`'s Dialog
//! host holds it inside [`UnitsDialog`](super::dialogs::UnitsDialog) and `dialogs/` draws it.

use lib_locale::Label;
use lib_units::{
    MANUAL_ENTRY, PriceLink, PriceSource, PriceSourceId, PriceSourceRecord, UnitId, UnitKind,
    UnitRow,
};

use crate::{
    chrome::dialog_host::{DialogKey, DialogOutcome},
    form::{
        field::TextField,
        select::{SelectKey, SelectState},
    },
};

/// The Add and Edit unit dialogs' fields, in `Tab` order: the mockup's reading order, Code and
/// Type, Name, then the PRICE SOURCE group.
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
    const ORDER: [UnitField; 6] = [
        Self::Code,
        Self::Type,
        Self::Name,
        Self::Source,
        Self::Symbol,
        Self::Quote,
    ];

    pub fn is_select(self) -> bool {
        matches!(self, Self::Type | Self::Source | Self::Quote)
    }
}

/// One Price Source as the Source select offers it: copied from the Ledger when the dialog opens,
/// so a Type change can re-filter without reading it again.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceOption {
    id: PriceSourceId,
    name: String,
    /// The Kinds it can price.
    kinds: Vec<UnitKind>,
}

/// What the three selects choose from, copied from the Ledger when the dialog opens.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnitOptions {
    /// Every source in the Ledger's order (alphabetical, Manual entry among them).
    sources: Vec<SourceOption>,
    /// Each Unit's id and the label the Quoted in select shows for it, in the Ledger's order.
    units: Vec<(UnitId, String)>,
    /// The Unit a new link is quoted in.
    base: Option<UnitId>,
}

impl UnitOptions {
    /// Reads the sources and Units from the Ledger. `manual_name` is Manual entry's Message and
    /// `base_label` the Message that marks the base Unit's option.
    pub fn new(
        records: &[PriceSourceRecord],
        manual_name: &str,
        units: &[UnitRow],
        base_label: impl Fn(&str) -> String,
    ) -> Self {
        let sources = lib_units::price_sources(records, manual_name)
            .iter()
            .map(|source: &PriceSource<'_>| SourceOption {
                id: source.id,
                name: source.name.to_string(),
                kinds: UnitKind::all()
                    .iter()
                    .copied()
                    .filter(|kind| source.prices(*kind))
                    .collect(),
            })
            .collect();
        let base = lib_units::base_unit(units)
            .or(units.first())
            .map(|unit| unit.id);
        let units = units
            .iter()
            .map(|unit| {
                let label = if Some(unit.id) == base {
                    base_label(&unit.code)
                } else {
                    unit.code.clone()
                };
                (unit.id, label)
            })
            .collect();
        Self {
            sources,
            units,
            base,
        }
    }

    /// The names of the sources able to price `kind`, in order.
    fn source_names(&self, kind: UnitKind) -> Vec<String> {
        self.sources
            .iter()
            .filter(|source| source.kinds.contains(&kind))
            .map(|source| source.name.clone())
            .collect()
    }

    fn source_named(&self, name: &str) -> Option<&SourceOption> {
        self.sources.iter().find(|source| source.name == name)
    }

    fn manual_name(&self) -> Option<&str> {
        self.sources
            .iter()
            .find(|source| source.id == MANUAL_ENTRY)
            .map(|source| source.name.as_str())
    }

    fn unit_labels(&self) -> Vec<String> {
        self.units.iter().map(|(_, label)| label.clone()).collect()
    }

    fn unit_label(&self, id: UnitId) -> Option<&str> {
        self.units
            .iter()
            .find(|(unit, _)| *unit == id)
            .map(|(_, label)| label.as_str())
    }
}

/// The Add/Edit unit dialogs' own live form state (issues #184/#185/#715) -- pure, `gpui`-free,
/// mirroring `navigation/nav.rs`/`chrome/palette.rs`'s "state here, chrome renders it" split.
/// `Shell` owns it wrapped in [`UnitsDialog`](super::dialogs::UnitsDialog) while the dialog is
/// open. Shared between both dialogs -- issue #185's own body: "same form as Add unit".
///
/// Code, Name and Symbol are typed into; Type, Source and Quoted in are [`SelectState`]s. The
/// Source list holds only the sources able to price the chosen Type, so changing Type re-filters
/// it and falls back to Manual entry when the chosen source no longer fits. Manual entry has no
/// Symbol or Quoted in to fill, so those fields are hidden and `Tab` skips them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitForm {
    pub code: TextField,
    pub name: TextField,
    pub kind: SelectState,
    pub source: SelectState,
    pub symbol: TextField,
    pub quote: SelectState,
    pub focused_field: UnitField,
    options: UnitOptions,
}

impl Default for UnitForm {
    /// A form with no Ledger behind it, for the host's own tests.
    fn default() -> Self {
        Self::new(&UnitOptions::default())
    }
}

impl UnitForm {
    /// An empty Add unit form: the first Kind, on Manual entry, quoted in the base Unit.
    pub fn new(options: &UnitOptions) -> Self {
        let kind = UnitKind::default();
        Self {
            code: TextField::default(),
            name: TextField::default(),
            kind: SelectState::new(Some(kind.label())),
            source: SelectState::new(options.manual_name().map(str::to_string)),
            symbol: TextField::default(),
            quote: SelectState::new(
                options
                    .base
                    .and_then(|base| options.unit_label(base))
                    .map(str::to_string),
            ),
            focused_field: UnitField::default(),
            options: options.clone(),
        }
    }

    /// Pre-fills a form from an existing row (issue #185's own Edit unit dialog).
    pub fn from_row(row: &UnitRow, options: &UnitOptions) -> Self {
        let source = options
            .sources
            .iter()
            .find(|source| source.id == row.link.source_id)
            .map(|source| source.name.clone());
        let mut form = Self {
            code: TextField::new(row.code.as_str()),
            name: TextField::new(row.name.as_str()),
            kind: SelectState::new(Some(row.kind.label())),
            source: SelectState::new(source),
            symbol: TextField::new(row.link.symbol.as_str()),
            quote: SelectState::new(options.unit_label(row.link.quote).map(str::to_string)),
            focused_field: UnitField::default(),
            options: options.clone(),
        };
        form.sync_source();
        form
    }

    /// The chosen Type; the first Kind while the select somehow holds no label.
    pub fn selected_kind(&self) -> UnitKind {
        self.kind
            .value()
            .and_then(|label| UnitKind::all().iter().copied().find(|k| k.label() == label))
            .unwrap_or_default()
    }

    /// The source the Source select holds.
    fn selected_source(&self) -> Option<&SourceOption> {
        self.options.source_named(self.source.value()?)
    }

    /// Whether the link is Manual entry, which hides Symbol, Quoted in and Test price.
    pub fn is_manual(&self) -> bool {
        self.selected_source()
            .is_none_or(|source| source.id == MANUAL_ENTRY)
    }

    /// The chosen source's name, for the Symbol field's help.
    pub fn source_name(&self) -> &str {
        self.source.value().unwrap_or_default()
    }

    /// The Link the form describes. A Manual entry link keeps no Symbol; with no Units at all
    /// the quote is an id naming none, which Manual entry never reads.
    pub fn link(&self) -> PriceLink {
        let quote = self
            .quote
            .value()
            .and_then(|label| {
                self.options
                    .units
                    .iter()
                    .find(|(_, unit)| unit == label)
                    .map(|(id, _)| *id)
            })
            .or(self.options.base)
            .unwrap_or_else(UnitId::new);
        match self.selected_source() {
            Some(source) if source.id != MANUAL_ENTRY => PriceLink {
                source_id: source.id,
                symbol: self.symbol.text().trim().to_string(),
                quote,
            },
            _ => PriceLink::manual(quote),
        }
    }

    /// The options behind `field`; empty for a text field. The Source list follows the Type.
    pub fn options_for(&self, field: UnitField) -> Vec<String> {
        match field {
            UnitField::Type => UnitKind::all().iter().map(|kind| kind.label()).collect(),
            UnitField::Source => self.options.source_names(self.selected_kind()),
            UnitField::Quote => self.options.unit_labels(),
            _ => Vec::new(),
        }
    }

    /// Falls back to Manual entry when the chosen source can't price the chosen Type.
    fn sync_source(&mut self) {
        let names = self.options.source_names(self.selected_kind());
        if self
            .source
            .value()
            .is_none_or(|name| !names.iter().any(|candidate| candidate == name))
        {
            self.source = SelectState::new(self.options.manual_name().map(str::to_string));
        }
        // Typing must never reach a field Manual entry just hid.
        if !self.is_visible(self.focused_field) {
            self.focused_field = UnitField::Source;
        }
    }

    fn select_mut(&mut self, field: UnitField) -> Option<&mut SelectState> {
        match field {
            UnitField::Type => Some(&mut self.kind),
            UnitField::Source => Some(&mut self.source),
            UnitField::Quote => Some(&mut self.quote),
            _ => None,
        }
    }

    /// Whether `field` is on screen: Manual entry hides Symbol and Quoted in.
    pub fn is_visible(&self, field: UnitField) -> bool {
        !(self.is_manual() && matches!(field, UnitField::Symbol | UnitField::Quote))
    }

    /// The names of the fields on screen, in `Tab` order, for the UI tests.
    pub fn visible_fields(&self) -> Vec<String> {
        UnitField::ORDER
            .iter()
            .filter(|field| self.is_visible(**field))
            .map(|field| format!("{field:?}"))
            .collect()
    }

    /// Whether any select's list is open.
    pub fn any_select_open(&self) -> bool {
        self.kind.is_open() || self.source.is_open() || self.quote.is_open()
    }

    /// Closes whichever list is open, discarding its highlight -- the first `Esc`. Returns whether
    /// one was open; if not, `Esc` should close the dialog.
    pub fn close_open_select(&mut self) -> bool {
        let was_open = self.any_select_open();
        self.kind.cancel();
        self.source.cancel();
        self.quote.cancel();
        was_open
    }

    /// Moves focus to `field`, closing any list open on another field. A hidden field is refused.
    pub fn focus(&mut self, field: UnitField) {
        if !self.is_visible(field) {
            return;
        }
        if field != self.focused_field {
            self.close_open_select();
        }
        self.focused_field = field;
    }

    /// `Tab` / `Shift-Tab`: commits an open list's highlight, then moves to the next visible
    /// field, wrapping.
    pub fn cycle_focus(&mut self, backward: bool) {
        let focused = self.focused_field;
        let list = self.options_for(focused);
        if let Some(state) = self.select_mut(focused) {
            state.commit(&list);
        }
        self.sync_source();
        let count = UnitField::ORDER.len();
        let mut index = UnitField::ORDER
            .iter()
            .position(|field| *field == self.focused_field)
            .unwrap_or(0);
        for _ in 0..count {
            index = if backward {
                (index + count - 1) % count
            } else {
                (index + 1) % count
            };
            let candidate = UnitField::ORDER[index];
            if self.is_visible(candidate) {
                self.focused_field = candidate;
                return;
            }
        }
    }

    /// A key on the focused select. Returns whether the focused field is a select (and so took
    /// the key).
    pub fn handle_select_key(&mut self, key: SelectKey) -> bool {
        let focused = self.focused_field;
        let list = self.options_for(focused);
        let Some(state) = self.select_mut(focused) else {
            return false;
        };
        match (key, state.is_open()) {
            (SelectKey::Up, true) => state.move_highlight(&list, -1),
            (SelectKey::Down, true) => state.move_highlight(&list, 1),
            (SelectKey::Up, false) => state.step(&list, -1),
            (SelectKey::Down, false) => state.step(&list, 1),
            (SelectKey::Activate, true) => state.commit(&list),
            (SelectKey::Activate, false) => state.open(&list),
        }
        self.sync_source();
        true
    }

    /// A click on a select's closed field: focuses it and toggles its list.
    pub fn click_select(&mut self, field: UnitField) {
        if !field.is_select() || !self.is_visible(field) {
            return;
        }
        self.focus(field);
        let list = self.options_for(field);
        if let Some(state) = self.select_mut(field) {
            if state.is_open() {
                state.cancel();
            } else {
                state.open(&list);
            }
        }
    }

    /// A click on row `index` of an open list.
    pub fn choose_option(&mut self, field: UnitField, index: usize) {
        let list = self.options_for(field);
        if let Some(state) = self.select_mut(field) {
            state.choose(&list, index);
        }
        self.sync_source();
    }

    /// The text field typing and `Backspace` edit, `None` while a select has focus.
    pub fn focused_text(&mut self) -> Option<&mut TextField> {
        match self.focused_field {
            UnitField::Code => Some(&mut self.code),
            UnitField::Name => Some(&mut self.name),
            UnitField::Symbol => Some(&mut self.symbol),
            _ => None,
        }
    }

    /// The keys that are not plain typing: select stepping and opening and `Shift-Tab`. A select
    /// swallows typed characters rather than letting them fall through to the shell.
    pub fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let on_select = self.focused_field.is_select();
        match key {
            DialogKey::Up => {
                self.handle_select_key(SelectKey::Up);
            }
            DialogKey::Down => {
                self.handle_select_key(SelectKey::Down);
            }
            DialogKey::BackTab => self.cycle_focus(true),
            DialogKey::Enter | DialogKey::Char(' ') if on_select => {
                self.handle_select_key(SelectKey::Activate);
            }
            DialogKey::Char(_) if on_select => {}
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    /// The README's own "Dialog lifecycle" row: "fill Code / Name / Type (all required)" --
    /// Type always has a value (a select can't be empty), so only Code/Name gate the Add/Save
    /// button's enabled state.
    pub fn is_valid(&self) -> bool {
        !self.code.is_blank() && !self.name.is_blank()
    }
}

/// The Delete unit dialog's own live form state (issue #186) -- pure, `gpui`-free. Just the one
/// typed-back confirmation field: unlike [`UnitForm`], there's nothing to `Tab` between, so no
/// `focused_field` -- the confirmation input is implicitly the only thing you can type into
/// while this dialog is open. The code to type back is copied in when the dialog opens, so the
/// form validates without reading `Shell`'s Units (the dialog is modal: they can't change under
/// it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteUnitForm {
    pub code: String,
    pub confirm_input: TextField,
}

impl DeleteUnitForm {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            confirm_input: TextField::default(),
        }
    }

    /// The README's own "disabled until the typed value matches the code exactly" -- a plain
    /// case-sensitive `==`, not a trim/lowercase-tolerant comparison, per the ticket's own body.
    pub fn is_valid(&self) -> bool {
        self.confirm_input.text() == self.code
    }
}

#[cfg(test)]
mod tests {
    use lib_units::COINGECKO;

    use super::*;

    fn options() -> (UnitOptions, Vec<UnitRow>) {
        crate::locale::init_for_tests();
        let units = lib_units::default_units(chrono::Utc::now());
        let options = UnitOptions::new(&[], "Manual entry", &units, |code| {
            format!("{code} (base unit)")
        });
        (options, units)
    }

    fn pick(form: &mut UnitForm, field: UnitField, label: &str) {
        let index = form
            .options_for(field)
            .iter()
            .position(|option| option == label)
            .expect("an option with that label");
        form.choose_option(field, index);
    }

    #[test]
    fn a_new_form_is_on_manual_entry_quoted_in_the_base_unit() {
        let (options, units) = options();
        let form = UnitForm::new(&options);
        assert!(form.is_manual());
        assert_eq!(
            form.link(),
            PriceLink::manual(lib_units::base_unit(&units).unwrap().id)
        );
        assert_eq!(form.quote.value(), Some("aud (base unit)"));
    }

    #[test]
    fn the_source_list_holds_only_sources_that_price_the_kind() {
        let (options, _) = options();
        let mut form = UnitForm::new(&options);
        pick(&mut form, UnitField::Type, &UnitKind::Crypto.label());
        assert_eq!(
            form.options_for(UnitField::Source),
            ["CoinGecko", "Manual entry"]
        );
        pick(&mut form, UnitField::Type, &UnitKind::Currency.label());
        assert_eq!(
            form.options_for(UnitField::Source),
            ["Frankfurter", "Manual entry"]
        );
    }

    #[test]
    fn changing_type_falls_back_to_manual_entry_when_the_source_no_longer_fits() {
        let (options, _) = options();
        let mut form = UnitForm::new(&options);
        pick(&mut form, UnitField::Type, &UnitKind::Crypto.label());
        pick(&mut form, UnitField::Source, "CoinGecko");
        assert!(!form.is_manual());
        pick(&mut form, UnitField::Type, &UnitKind::Equity.label());
        assert!(form.is_manual());
        assert!(
            form.options_for(UnitField::Source)
                .contains(&"Yahoo Finance".to_string())
        );
    }

    #[test]
    fn a_fetching_source_links_its_symbol_and_quote() {
        let (options, units) = options();
        let mut form = UnitForm::new(&options);
        pick(&mut form, UnitField::Type, &UnitKind::Crypto.label());
        pick(&mut form, UnitField::Source, "CoinGecko");
        form.focus(UnitField::Symbol);
        form.focused_text().unwrap().push('x');
        assert_eq!(
            form.link(),
            PriceLink {
                source_id: COINGECKO,
                symbol: "x".to_string(),
                quote: lib_units::base_unit(&units).unwrap().id,
            }
        );
    }

    #[test]
    fn manual_entry_hides_symbol_and_quote_and_tab_skips_them() {
        let (options, _) = options();
        let mut form = UnitForm::new(&options);
        assert!(!form.is_visible(UnitField::Symbol));
        form.focus(UnitField::Symbol);
        assert_eq!(form.focused_field, UnitField::Code);
        form.focused_field = UnitField::Source;
        form.cycle_focus(false);
        assert_eq!(form.focused_field, UnitField::Code);
    }

    #[test]
    fn from_row_prefills_the_link_and_keeps_a_fitting_source() {
        let (options, units) = options();
        let btc = units
            .iter()
            .find(|unit| unit.id == UnitId::SEED_BTC)
            .unwrap();
        let form = UnitForm::from_row(btc, &options);
        assert_eq!(form.code.text(), "btc");
        assert_eq!(form.selected_kind(), UnitKind::Crypto);
        assert_eq!(form.source.value(), Some("CoinGecko"));
        assert_eq!(form.link(), btc.link);
    }

    #[test]
    fn add_unit_form_is_invalid_until_code_and_name_are_both_filled() {
        let mut form = UnitForm::default();
        assert!(!form.is_valid());
        form.focused_text().unwrap().push('a');
        assert!(!form.is_valid());
        form.focused_field = UnitField::Name;
        form.focused_text().unwrap().push(' ');
        assert!(!form.is_valid());
        form.focused_text().unwrap().push('b');
        assert!(form.is_valid());
    }
}
