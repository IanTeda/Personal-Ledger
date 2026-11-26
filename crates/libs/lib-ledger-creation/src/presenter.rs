//! The Create ledger Dialog's Presenter, in two stages (#1344): Base unit, the base Unit as Add
//! unit asks for it, prefilled from the Locale's Currency, over the file name the person chose;
//! then Ledger, the Ledger name (prefilled from the file name), Owner and Financial year start
//! (ADR-0061). The Presenter holds one form and the stage only decides which fields show, so Back
//! keeps everything typed, both ways. Nothing is written until Create on the Ledger stage, which
//! runs the Interactor once; a cancel writes nothing. The Price Source is Manual entry, so Add
//! unit's Source, Symbol and Quoted in fields, which only matter to a Unit priced from elsewhere
//! and quoted in another, are not asked.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use lib_core::{FinancialYearStart, LedgerName, Owner, UnitId};
use lib_locale::Currency;
use lib_locale::Label as _;
use lib_units::interactor::AddUnit;
use lib_units::{Precision, PrecisionStep, PriceLink, UnitKind};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{
    Choice, DialogFrame, DialogHint, Hint, SelectList, SelectMove, SelectView, TextFieldView,
};

use crate::interactor::{CreateLedger, LedgerCreated, base_unit_for};
use crate::msg;
use crate::protocols::LedgerCreation;

/// The creation port a Dialog shares with its Client.
pub type SharedLedgerCreation = Rc<RefCell<dyn LedgerCreation>>;

/// The Dialog's stages, in order: the base Unit, then the Ledger's own Preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LedgerStage {
    #[default]
    BaseUnit,
    Ledger,
}

impl LedgerStage {
    const ALL: [Self; 2] = [Self::BaseUnit, Self::Ledger];

    /// The stage's place, counted from 1, as its step and the status line show it.
    fn number(self) -> usize {
        match self {
            Self::BaseUnit => 1,
            Self::Ledger => 2,
        }
    }

    fn label(self) -> String {
        match self {
            Self::BaseUnit => msg::ledger_creation_stage_base_unit(),
            Self::Ledger => msg::ledger_creation_stage_ledger(),
        }
    }

    /// The stage's fields, in `Tab` order: `Tab` cycles within a stage and never crosses one.
    fn order(self) -> &'static [LedgerField] {
        match self {
            Self::BaseUnit => &[
                LedgerField::Code,
                LedgerField::Type,
                LedgerField::Name,
                LedgerField::Precision,
            ],
            Self::Ledger => &[
                LedgerField::LedgerName,
                LedgerField::Owner,
                LedgerField::FinancialYearStart,
            ],
        }
    }
}

/// The form's fields, the base Unit's then the Ledger's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LedgerField {
    #[default]
    Code,
    Type,
    Name,
    Precision,
    LedgerName,
    Owner,
    FinancialYearStart,
}

impl LedgerField {
    /// Whether the field is a select, which swallows typing.
    fn is_select(self) -> bool {
        matches!(self, Self::Type | Self::FinancialYearStart)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateLedgerEvent {
    /// A Dialog command or typed key: the field keys move focus within the stage, `select` and
    /// `submit` confirm a valid stage (`select` opens and commits a select's list on that field),
    /// and typing edits the focused text field.
    Input(DialogInput),
    /// A text field's whole new text.
    Code(String),
    Name(String),
    Precision(String),
    /// A click on one of the Precision field's arrows: focuses it, closing an open list, and steps
    /// it a place.
    StepPrecision(PrecisionStep),
    LedgerName(String),
    Owner(String),
    /// A Type picked, closing its list.
    Kind(UnitKind),
    /// A click on the Type select: focuses it and opens or closes its list.
    ToggleKind,
    /// A month picked for the Financial year start, closing its list.
    Month(FinancialYearStart),
    /// A click on the Financial year starts select: focuses it and opens or closes its list.
    ToggleMonth,
    /// A click on a text field, which closes an open list.
    Focus(LedgerField),
    /// A click on a step: Base unit goes Back, Ledger goes there only when Next would.
    Step(LedgerStage),
    /// Closes the open list without picking: the Desktop host's `back`.
    CloseList,
    /// Next on Base unit, Create on Ledger.
    Confirm,
    /// `back`: to Base unit from Ledger, keeping everything typed; a cancel on Base unit.
    Back,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateLedgerViewModel {
    pub frame: DialogFrame,
    /// The stage showing, which decides the fields drawn and the footer's buttons.
    pub stage: LedgerStage,
    /// Each stage's step, `1 · Base unit` and `2 · Ledger`, in order, beside its stage.
    pub steps: Vec<(LedgerStage, String)>,
    /// The Back button's label, drawn on the Ledger stage.
    pub back: String,
    /// The status line: the stage showing and, while the Ledger name is refused, why.
    pub hints: Vec<Hint>,
    pub code: TextFieldView,
    pub kind: SelectView<UnitKind>,
    pub name: TextFieldView,
    pub precision: TextFieldView,
    pub precision_help: String,
    /// Why the base Unit matters.
    pub help: String,
    /// What the Ledger stage asks for.
    pub ledger_help: String,
    pub ledger_name: TextFieldView,
    pub ledger_name_help: String,
    /// Why the Ledger name can't be created with, while it can't.
    pub ledger_name_error: Option<String>,
    pub owner: TextFieldView,
    pub owner_help: String,
    pub financial_year: SelectView<FinancialYearStart>,
    pub financial_year_help: String,
    /// The focused select's list: Type's on Base unit, Financial year starts' on Ledger.
    pub list: SelectList,
}

pub struct CreateLedgerPresenter {
    creation: SharedLedgerCreation,
    path: PathBuf,
    personal: String,
    stage: LedgerStage,
    /// The Ledger name as typed, prefilled once from the file, which is fixed for the Dialog's life.
    ledger_name: String,
    owner: String,
    financial_year_start: FinancialYearStart,
    id: UnitId,
    code: String,
    name: String,
    kind: UnitKind,
    precision: String,
    /// Whether a person has typed a Precision, which stops it following Type.
    precision_typed: bool,
    focused: LedgerField,
    list: SelectList,
}

impl CreateLedgerPresenter {
    /// The Dialog for the Ledger file at `path`, prefilled with `currency`, the Locale's, and the
    /// Personal Institution named `personal`, its Ledger stage prefilled with `ledger_name`,
    /// `owner` and `financial_year_start`.
    pub fn new(
        creation: SharedLedgerCreation,
        path: PathBuf,
        currency: Currency,
        personal: String,
        ledger_name: LedgerName,
        owner: Option<Owner>,
        financial_year_start: FinancialYearStart,
    ) -> Self {
        let base = base_unit_for(currency);
        Self {
            creation,
            path,
            personal,
            stage: LedgerStage::default(),
            ledger_name: ledger_name.as_str().to_string(),
            owner: owner
                .map(|owner| owner.as_str().to_string())
                .unwrap_or_default(),
            financial_year_start,
            id: base.id,
            code: base.code,
            name: base.name,
            kind: base.kind,
            precision: base.precision.to_string(),
            precision_typed: false,
            focused: LedgerField::default(),
            list: SelectList::default(),
        }
    }

    fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<LedgerCreated>) {
        match input {
            DialogInput::NextField | DialogInput::PreviousField => {
                self.list.close();
                let order = self.stage.order();
                let count = order.len();
                let at = order
                    .iter()
                    .position(|field| *field == self.focused)
                    .unwrap_or(0);
                let to = if input == DialogInput::NextField {
                    (at + 1) % count
                } else {
                    (at + count - 1) % count
                };
                self.focused = order[to];
            }
            DialogInput::Submit => self.confirm(router),
            // A select swallows typing, so it never reaches a text field.
            _ if self.focused.is_select() => self.select_input(input),
            DialogInput::Select => self.confirm(router),
            // `↑` and `↓` step the Precision, as its arrows do.
            DialogInput::SelectPrevious if self.focused == LedgerField::Precision => {
                self.step_precision(PrecisionStep::Up);
            }
            DialogInput::SelectNext if self.focused == LedgerField::Precision => {
                self.step_precision(PrecisionStep::Down);
            }
            DialogInput::Char(ch) => {
                // A Unit's code is upper case, so the Code field shows what will be saved.
                let upper = self.focused == LedgerField::Code;
                if let Some(text) = self.text_mut() {
                    if upper {
                        text.extend(ch.to_uppercase());
                    } else {
                        text.push(ch);
                    }
                }
                self.typed();
            }
            DialogInput::Backspace => {
                if let Some(text) = self.text_mut() {
                    text.pop();
                }
                self.typed();
            }
            _ => {}
        }
    }

    /// `input` on the focused select: `select` opens a closed list and commits an open one, the
    /// arrows step the value or the highlight, and `submit` alone confirms from here.
    fn select_input(&mut self, input: DialogInput) {
        let view = self.view_model();
        if self.focused == LedgerField::FinancialYearStart {
            let select = &view.financial_year;
            if let SelectMove::Chose(start) =
                self.list
                    .input(input, &select.options, select.chosen.as_ref())
            {
                self.financial_year_start = start;
            }
        } else if let SelectMove::Chose(kind) =
            self.list
                .input(input, &view.kind.options, view.kind.chosen.as_ref())
        {
            self.pick_kind(kind);
        }
    }

    fn text_mut(&mut self) -> Option<&mut String> {
        match self.focused {
            LedgerField::Code => Some(&mut self.code),
            LedgerField::Name => Some(&mut self.name),
            LedgerField::Precision => Some(&mut self.precision),
            LedgerField::LedgerName => Some(&mut self.ledger_name),
            LedgerField::Owner => Some(&mut self.owner),
            LedgerField::Type | LedgerField::FinancialYearStart => None,
        }
    }

    fn typed(&mut self) {
        if self.focused == LedgerField::Precision {
            self.precision_typed = true;
        }
    }

    /// Steps the Precision a place, from the Type's when what is typed is no number. A stepped
    /// Precision is a chosen one, so it stops following Type, as a typed one does.
    fn step_precision(&mut self, step: PrecisionStep) {
        self.precision =
            Precision::step(&self.precision, step, Precision::of_kind(self.kind)).to_string();
        self.precision_typed = true;
    }

    fn pick_kind(&mut self, kind: UnitKind) {
        self.kind = kind;
        if !self.precision_typed {
            self.precision = Precision::of_kind(kind).to_string();
        }
    }

    /// Code, Name and a Precision in range are required, as Add unit requires them.
    fn base_unit_is_valid(&self) -> bool {
        !self.code.trim().is_empty()
            && !self.name.trim().is_empty()
            && Precision::parse(&self.precision).is_some()
    }

    /// Whether the showing stage would confirm: Next on Base unit, Create on Ledger.
    fn is_valid(&self) -> bool {
        match self.stage {
            LedgerStage::BaseUnit => self.base_unit_is_valid(),
            LedgerStage::Ledger => {
                self.base_unit_is_valid() && LedgerName::new(&self.ledger_name).is_ok()
            }
        }
    }

    /// Why the typed Ledger name can't be created with, `None` while it can.
    fn ledger_name_error(&self) -> Option<String> {
        LedgerName::new(&self.ledger_name)
            .err()
            .map(lib_locale::format::ledger_name_error_message)
    }

    /// Shows `stage`, focusing its first field.
    fn show(&mut self, stage: LedgerStage) {
        self.list.close();
        self.stage = stage;
        self.focused = stage.order()[0];
    }

    fn back(&mut self, router: &mut dyn DialogRouter<LedgerCreated>) {
        match self.stage {
            LedgerStage::Ledger => self.show(LedgerStage::BaseUnit),
            LedgerStage::BaseUnit => router.close(None),
        }
    }

    fn confirm(&mut self, router: &mut dyn DialogRouter<LedgerCreated>) {
        if !self.is_valid() {
            return;
        }
        match self.stage {
            LedgerStage::BaseUnit => self.show(LedgerStage::Ledger),
            LedgerStage::Ledger => self.create(router),
        }
    }

    fn create(&mut self, router: &mut dyn DialogRouter<LedgerCreated>) {
        let (Some(precision), Ok(name)) = (
            Precision::parse(&self.precision),
            LedgerName::new(&self.ledger_name),
        ) else {
            return;
        };
        let created = CreateLedger {
            path: self.path.clone(),
            base_unit: AddUnit {
                id: self.id,
                code: self.code.clone(),
                name: self.name.clone(),
                kind: self.kind,
                precision,
                link: PriceLink::manual(self.id),
            },
            personal: self.personal.clone(),
            name,
            owner: Owner::new(&self.owner),
            financial_year_start: self.financial_year_start,
        }
        .run(&mut *self.creation.borrow_mut(), router);
        // A refusal has raised its Error Toast; the Dialog stays on this stage for another try or
        // a cancel.
        if let Ok(created) = created {
            router.close(Some(created));
        }
    }

    /// Opens or closes `field`'s list, focusing it first.
    fn toggle(&mut self, field: LedgerField) {
        if self.focused != field {
            self.list.close();
            self.focused = field;
        }
        let view = self.view_model();
        if field == LedgerField::FinancialYearStart {
            self.list.toggle(
                &view.financial_year.options,
                view.financial_year.chosen.as_ref(),
            );
        } else {
            self.list
                .toggle(&view.kind.options, view.kind.chosen.as_ref());
        }
    }
}

impl Presenter for CreateLedgerPresenter {
    type Event = CreateLedgerEvent;
    type ViewModel = CreateLedgerViewModel;
    type Router<'r> = dyn DialogRouter<LedgerCreated> + 'r;

    fn handle(&mut self, event: CreateLedgerEvent, router: &mut Self::Router<'_>) {
        match event {
            CreateLedgerEvent::Input(input) => self.input(input, router),
            CreateLedgerEvent::Code(text) => self.code = text.to_uppercase(),
            CreateLedgerEvent::Name(text) => self.name = text,
            CreateLedgerEvent::Precision(text) => {
                self.precision = text;
                self.precision_typed = true;
            }
            CreateLedgerEvent::StepPrecision(step) => {
                self.list.close();
                self.focused = LedgerField::Precision;
                self.step_precision(step);
            }
            CreateLedgerEvent::LedgerName(text) => self.ledger_name = text,
            CreateLedgerEvent::Owner(text) => self.owner = text,
            CreateLedgerEvent::Kind(kind) => {
                self.list.close();
                self.pick_kind(kind);
            }
            CreateLedgerEvent::ToggleKind => self.toggle(LedgerField::Type),
            CreateLedgerEvent::Month(start) => {
                self.list.close();
                self.financial_year_start = start;
            }
            CreateLedgerEvent::ToggleMonth => self.toggle(LedgerField::FinancialYearStart),
            CreateLedgerEvent::Focus(field) => {
                self.list.close();
                self.focused = field;
            }
            CreateLedgerEvent::Step(LedgerStage::BaseUnit) => {
                if self.stage == LedgerStage::Ledger {
                    self.show(LedgerStage::BaseUnit);
                }
            }
            CreateLedgerEvent::Step(LedgerStage::Ledger) => {
                if self.stage == LedgerStage::BaseUnit {
                    self.confirm(router);
                }
            }
            CreateLedgerEvent::CloseList => {
                self.list.close();
            }
            CreateLedgerEvent::Confirm => self.confirm(router),
            CreateLedgerEvent::Back => self.back(router),
            CreateLedgerEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> CreateLedgerViewModel {
        let text = |label: String, text: &str, placeholder: String, field| TextFieldView {
            label,
            text: text.to_string(),
            placeholder,
            focused: self.focused == field,
        };
        let total = LedgerStage::ALL.len().to_string();
        let ledger_name_error = self.ledger_name_error();
        let mut hints = vec![Hint::new(
            self.stage.label(),
            msg::ledger_creation_stage_count(&self.stage.number().to_string(), &total),
        )];
        if self.stage == LedgerStage::Ledger
            && let Some(error) = &ledger_name_error
        {
            hints.push(Hint::new(String::new(), error.clone()));
        }
        CreateLedgerViewModel {
            frame: DialogFrame {
                title: msg::ledger_creation_title(),
                submit: match self.stage {
                    LedgerStage::BaseUnit => msg::ledger_creation_next(),
                    LedgerStage::Ledger => msg::ledger_creation_submit(),
                },
                can_confirm: self.is_valid(),
                destructive: false,
                hints: Vec::<DialogHint>::new(),
            },
            stage: self.stage,
            steps: LedgerStage::ALL
                .iter()
                .map(|stage| {
                    (
                        *stage,
                        msg::ledger_creation_step(&stage.number().to_string(), &stage.label()),
                    )
                })
                .collect(),
            back: msg::ledger_creation_back(),
            hints,
            code: text(
                lib_locale::msg::column_code(),
                &self.code,
                lib_units::msg::units_code_placeholder(),
                LedgerField::Code,
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
                focused: self.focused == LedgerField::Type,
            },
            name: text(
                lib_locale::msg::column_name(),
                &self.name,
                lib_units::msg::units_name_placeholder(),
                LedgerField::Name,
            ),
            precision: text(
                lib_units::msg::units_field_precision(),
                &self.precision,
                lib_units::msg::units_precision_placeholder(&Precision::MAX.to_string()),
                LedgerField::Precision,
            ),
            precision_help: lib_units::msg::units_precision_help(),
            help: msg::ledger_creation_base_unit_help(),
            ledger_help: msg::ledger_creation_ledger_help(),
            ledger_name: text(
                msg::ledger_creation_field_ledger_name(),
                &self.ledger_name,
                lib_locale::msg::ledger_name_fallback(),
                LedgerField::LedgerName,
            ),
            ledger_name_help: msg::ledger_creation_ledger_name_help(),
            ledger_name_error,
            owner: text(
                msg::ledger_creation_field_owner(),
                &self.owner,
                String::new(),
                LedgerField::Owner,
            ),
            owner_help: msg::ledger_creation_owner_help(),
            financial_year: SelectView {
                label: msg::ledger_creation_field_financial_year(),
                options: FinancialYearStart::ALL
                    .into_iter()
                    .map(|start| Choice {
                        value: start,
                        label: lib_locale::format::format_month(start.month()),
                    })
                    .collect(),
                chosen: Some(self.financial_year_start),
                focused: self.focused == LedgerField::FinancialYearStart,
            },
            financial_year_help: msg::ledger_creation_financial_year_help(),
            list: self.list,
        }
    }
}

impl DialogPresenter for CreateLedgerPresenter {
    type Outcome = LedgerCreated;

    fn input_event(input: DialogInput) -> CreateLedgerEvent {
        CreateLedgerEvent::Input(input)
    }

    fn confirm_event() -> CreateLedgerEvent {
        CreateLedgerEvent::Confirm
    }

    fn cancel_event() -> CreateLedgerEvent {
        CreateLedgerEvent::Cancel
    }

    /// Back on Ledger, a cancel on Base unit.
    fn back_event() -> CreateLedgerEvent {
        CreateLedgerEvent::Back
    }

    fn can_confirm(&self) -> bool {
        self.is_valid()
    }

    fn close_open_select(&mut self) -> bool {
        let open = self.list.is_open();
        self.list.close();
        open
    }
}

#[cfg(test)]
mod tests {
    use lib_toast::{Notice, Notify, ToastKind};

    use std::path::Path;

    use super::*;
    use crate::fake::InMemoryLedgerCreation;

    #[derive(Default)]
    struct Router {
        closed: Vec<Option<LedgerCreated>>,
        notices: Vec<Notice>,
    }

    impl Notify for Router {
        fn notify(&mut self, notice: Notice) {
            self.notices.push(notice);
        }
    }

    impl DialogRouter<LedgerCreated> for Router {
        fn close(&mut self, outcome: Option<LedgerCreated>) {
            self.closed.push(outcome);
        }
    }

    struct Dialog {
        presenter: CreateLedgerPresenter,
        port: Rc<RefCell<InMemoryLedgerCreation>>,
        router: Router,
    }

    impl Dialog {
        fn new(currency: Currency) -> Self {
            lib_locale::init_with_layers("en-US", &[lib_units::msg::LAYER, msg::LAYER]);
            let port = Rc::new(RefCell::new(InMemoryLedgerCreation::default()));
            let creation: SharedLedgerCreation = port.clone();
            Self {
                presenter: CreateLedgerPresenter::new(
                    creation,
                    PathBuf::from("/ledgers/teda.pldb"),
                    currency,
                    "Personal".into(),
                    crate::interactor::name_for(Path::new("/ledgers/teda.pldb")),
                    None,
                    FinancialYearStart::APRIL,
                ),
                port,
                router: Router::default(),
            }
        }

        fn send(&mut self, events: impl IntoIterator<Item = CreateLedgerEvent>) {
            for event in events {
                self.presenter.handle(event, &mut self.router);
            }
        }

        fn press(&mut self, inputs: impl IntoIterator<Item = DialogInput>) {
            self.send(inputs.into_iter().map(CreateLedgerEvent::Input));
        }
    }

    #[test]
    fn the_prefill_is_the_locales_currency_per_locale() {
        lib_locale::init_with_layers("en-US", &[lib_units::msg::LAYER, msg::LAYER]);
        for (locale, code, name) in [
            (lib_locale::Locale::EnUs, "USD", "US Dollar"),
            (lib_locale::Locale::EnGb, "GBP", "British Pound"),
            (lib_locale::Locale::EnAu, "AUD", "Australian Dollar"),
            (lib_locale::Locale::EnXa, "USD", "US Dollar"),
        ] {
            let dialog = Dialog::new(locale.currency());
            let view = dialog.presenter.view_model();
            assert_eq!(
                (view.code.text.as_str(), view.name.text.as_str()),
                (code, name),
                "{locale}"
            );
            assert_eq!(view.kind.chosen, Some(UnitKind::Currency));
            assert_eq!(view.precision.text, "2");
            assert!(view.frame.can_confirm);
        }
    }

    #[test]
    fn the_dialog_opens_on_the_base_unit_stage() {
        let dialog = Dialog::new(Currency::Aud);
        let view = dialog.presenter.view_model();
        assert_eq!(view.frame.title, "Create ledger");
        assert_eq!(
            view.frame.submit, "Next",
            "Create waits on the Ledger stage"
        );
        assert_eq!(view.stage, LedgerStage::BaseUnit);
    }

    #[test]
    fn create_is_off_while_a_field_is_invalid_and_confirming_writes_nothing() {
        let mut dialog = Dialog::new(Currency::Aud);
        for (event, label) in [
            (CreateLedgerEvent::Code("  ".into()), "code"),
            (CreateLedgerEvent::Name(String::new()), "name"),
            (CreateLedgerEvent::Precision("19".into()), "precision"),
        ] {
            let mut dialog_one = Dialog::new(Currency::Aud);
            dialog_one.send([event, CreateLedgerEvent::Confirm]);
            let view = dialog_one.presenter.view_model();
            assert!(!view.frame.can_confirm, "{label}");
            assert_eq!(
                view.stage,
                LedgerStage::BaseUnit,
                "Next is refused: {label}"
            );
            assert_eq!(dialog_one.port.borrow().calls, 0, "{label}");
            assert!(dialog_one.router.closed.is_empty(), "{label}");
        }
        dialog.send([CreateLedgerEvent::Precision("x".into())]);
        dialog.press([DialogInput::Submit, DialogInput::Submit]);
        assert_eq!(dialog.port.borrow().calls, 0);
        assert_eq!(dialog.presenter.view_model().stage, LedgerStage::BaseUnit);
    }

    #[test]
    fn esc_writes_nothing() {
        let mut dialog = Dialog::new(Currency::Usd);
        dialog.send([CreateLedgerEvent::Cancel]);

        assert_eq!(dialog.router.closed, [None]);
        assert_eq!(dialog.port.borrow().calls, 0);
    }

    #[test]
    fn create_calls_the_port_once_with_the_edited_base_unit_and_closes_with_it() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.send([
            CreateLedgerEvent::Code("BTC".into()),
            CreateLedgerEvent::Name("Bitcoin".into()),
            CreateLedgerEvent::Kind(UnitKind::Crypto),
        ]);
        assert_eq!(dialog.presenter.view_model().precision.text, "8");

        dialog.press([DialogInput::Submit]);
        assert_eq!(dialog.port.borrow().calls, 0, "Next writes nothing");
        dialog.press([DialogInput::Submit]);

        assert_eq!(dialog.port.borrow().calls, 1);
        let [Some(created)] = dialog.router.closed.as_slice() else {
            panic!("one confirmed close, not {:?}", dialog.router.closed);
        };
        let port = dialog.port.borrow();
        let ledger = &port.ledgers[&created.path];
        assert_eq!(ledger.base_unit.id, created.base_unit);
        assert_eq!(
            (ledger.base_unit.code.as_str(), ledger.base_unit.kind),
            ("BTC", UnitKind::Crypto)
        );
        assert_eq!(
            (
                ledger.name.as_str(),
                &ledger.owner,
                ledger.financial_year_start
            ),
            ("teda", &None, FinancialYearStart::APRIL),
            "the Ledger stage's prefill, unedited"
        );
    }

    #[test]
    fn enter_goes_next_then_creates_from_a_text_field_and_submit_from_the_type_select() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.press([DialogInput::Select]);
        assert_eq!(dialog.presenter.view_model().stage, LedgerStage::Ledger);
        assert_eq!(dialog.port.borrow().calls, 0);
        dialog.press([DialogInput::Select]);
        assert_eq!(dialog.port.borrow().calls, 1);

        let mut dialog = Dialog::new(Currency::Aud);
        dialog.press([DialogInput::NextField, DialogInput::Select]);
        assert!(
            dialog.presenter.view_model().list.is_open(),
            "select opens the list"
        );
        assert_eq!(dialog.port.borrow().calls, 0);
        dialog.press([DialogInput::Select, DialogInput::Submit]);
        assert_eq!(dialog.presenter.view_model().stage, LedgerStage::Ledger);
        dialog.press([DialogInput::Submit]);
        assert_eq!(dialog.port.borrow().calls, 1);
    }

    #[test]
    fn the_type_list_opens_steps_and_commits_without_creating() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.press([DialogInput::NextField, DialogInput::Select]);
        assert!(dialog.presenter.view_model().list.is_open());
        dialog.press([DialogInput::SelectNext, DialogInput::Select]);

        let view = dialog.presenter.view_model();
        assert!(!view.list.is_open());
        assert_ne!(view.kind.chosen, Some(UnitKind::Currency));
        assert_eq!(dialog.port.borrow().calls, 0);
    }

    #[test]
    fn a_typed_precision_stops_following_the_type() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.send([
            CreateLedgerEvent::Precision("4".into()),
            CreateLedgerEvent::Kind(UnitKind::Crypto),
        ]);
        assert_eq!(dialog.presenter.view_model().precision.text, "4");
    }

    #[test]
    fn the_arrow_keys_step_the_focused_precision_and_its_arrows_focus_and_step_it() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.press([DialogInput::PreviousField]);
        dialog.press([DialogInput::SelectPrevious, DialogInput::SelectPrevious]);
        assert_eq!(
            dialog.presenter.view_model().precision.text,
            "4",
            "up adds a place"
        );
        dialog.press([DialogInput::SelectNext]);
        assert_eq!(
            dialog.presenter.view_model().precision.text,
            "3",
            "down takes one"
        );

        dialog.send([CreateLedgerEvent::Kind(UnitKind::Crypto)]);
        assert_eq!(
            dialog.presenter.view_model().precision.text,
            "3",
            "a stepped Precision stops following the Type"
        );

        let mut dialog = Dialog::new(Currency::Aud);
        dialog.send([CreateLedgerEvent::StepPrecision(PrecisionStep::Down)]);
        let view = dialog.presenter.view_model();
        assert!(view.precision.focused, "an arrow focuses the field");
        assert_eq!(view.precision.text, "1");
        assert_eq!(dialog.port.borrow().calls, 0, "stepping writes nothing");
    }

    #[test]
    fn typing_edits_the_focused_field_and_tab_wraps() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.press([DialogInput::Backspace, DialogInput::Char('x')]);
        assert_eq!(
            dialog.presenter.view_model().code.text,
            "AUX",
            "the code types upper case"
        );
        dialog.press([DialogInput::PreviousField]);
        assert!(dialog.presenter.view_model().precision.focused);
        dialog.press([DialogInput::NextField]);
        assert!(dialog.presenter.view_model().code.focused);
    }

    #[test]
    fn a_failed_create_raises_an_error_toast_and_keeps_the_dialog_open() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.port.borrow_mut().broken = true;

        dialog.press([DialogInput::Submit, DialogInput::Submit]);

        assert!(dialog.router.closed.is_empty());
        assert_eq!(
            dialog.presenter.view_model().stage,
            LedgerStage::Ledger,
            "a refusal keeps the Ledger stage"
        );
        assert_eq!(
            dialog.router.notices,
            [Notice::Toast(
                ToastKind::Error,
                "Couldn't create ledger teda.pldb: it can't be written there".into()
            )]
        );
        assert!(dialog.port.borrow().ledgers.is_empty());
    }

    /// The Dialog on its Ledger stage, reached by Next.
    fn on_ledger() -> Dialog {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.press([DialogInput::Submit]);
        dialog
    }

    #[test]
    fn next_shows_the_ledger_stage_prefilled_and_focused_on_its_name() {
        let dialog = on_ledger();
        let view = dialog.presenter.view_model();

        assert_eq!(view.stage, LedgerStage::Ledger);
        assert_eq!(view.frame.submit, "Create");
        assert!(view.frame.can_confirm);
        assert!(view.ledger_name.focused, "its first field");
        assert_eq!(view.ledger_name.text, "teda", "the file name without .pldb");
        assert_eq!(view.owner.text, "");
        assert_eq!(view.financial_year.chosen, Some(FinancialYearStart::APRIL));
        let months: Vec<_> = view
            .financial_year
            .options
            .iter()
            .map(|choice| choice.label.as_str())
            .collect();
        assert_eq!(months.len(), 12);
        assert_eq!((months[0], months[11]), ("Jan", "Dec"));
        assert_eq!(view.ledger_name_error, None);
        assert_eq!(dialog.port.borrow().calls, 0);
    }

    #[test]
    fn the_steps_and_the_status_line_name_the_stage() {
        let mut dialog = Dialog::new(Currency::Aud);
        let view = dialog.presenter.view_model();
        assert_eq!(
            view.steps,
            [
                (LedgerStage::BaseUnit, "1 · Base unit".to_string()),
                (LedgerStage::Ledger, "2 · Ledger".to_string()),
            ]
        );
        assert_eq!(view.hints, [Hint::new("Base unit", "(1 of 2)")]);

        dialog.press([DialogInput::Submit]);
        assert_eq!(
            dialog.presenter.view_model().hints,
            [Hint::new("Ledger", "(2 of 2)")]
        );
    }

    #[test]
    fn create_is_refused_while_the_ledger_name_is_blank_or_too_long() {
        for (text, error) in [
            ("   ", "A Ledger needs a name"),
            (
                &"x".repeat(61) as &str,
                "A ledger name is at most 60 characters",
            ),
        ] {
            let mut dialog = on_ledger();
            dialog.send([CreateLedgerEvent::LedgerName(text.into())]);
            dialog.press([DialogInput::Submit]);
            dialog.send([CreateLedgerEvent::Confirm]);

            let view = dialog.presenter.view_model();
            assert!(!view.frame.can_confirm, "{error}");
            assert_eq!(view.ledger_name_error.as_deref(), Some(error));
            assert_eq!(
                view.hints[1],
                Hint::new("", error),
                "the status line says why"
            );
            assert_eq!(view.stage, LedgerStage::Ledger);
            assert_eq!(view.ledger_name.text, text, "the typed text is kept");
            assert_eq!(dialog.port.borrow().calls, 0);
            assert!(dialog.router.closed.is_empty());
        }
        let mut dialog = on_ledger();
        dialog.send([CreateLedgerEvent::LedgerName("x".repeat(60))]);
        assert!(dialog.presenter.view_model().frame.can_confirm, "60 fits");
    }

    #[test]
    fn create_writes_the_typed_name_owner_and_month() {
        let mut dialog = on_ledger();
        dialog.press([
            DialogInput::Backspace,
            DialogInput::Backspace,
            DialogInput::Char('x'),
        ]);
        dialog.press([DialogInput::NextField]);
        dialog.press("  Ian ".chars().map(DialogInput::Char));
        dialog.send([CreateLedgerEvent::Month(FinancialYearStart::JULY)]);

        dialog.press([DialogInput::Submit]);

        let [Some(created)] = dialog.router.closed.as_slice() else {
            panic!("one confirmed close, not {:?}", dialog.router.closed);
        };
        let port = dialog.port.borrow();
        let ledger = &port.ledgers[&created.path];
        assert_eq!(ledger.name.as_str(), "tex");
        assert_eq!(ledger.owner.as_ref().map(Owner::as_str), Some("Ian"));
        assert_eq!(ledger.financial_year_start, FinancialYearStart::JULY);
    }

    #[test]
    fn back_keeps_everything_typed_both_ways() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.send([CreateLedgerEvent::Code("BTC".into())]);
        dialog.press([DialogInput::Submit]);
        dialog.send([
            CreateLedgerEvent::LedgerName("Household".into()),
            CreateLedgerEvent::Owner("Ian".into()),
        ]);

        dialog.send([CreateLedgerEvent::Back]);
        let view = dialog.presenter.view_model();
        assert_eq!(view.stage, LedgerStage::BaseUnit);
        assert!(view.code.focused, "its first field");
        assert_eq!(view.code.text, "BTC");
        assert!(dialog.router.closed.is_empty(), "Back closes nothing");

        dialog.press([DialogInput::Submit]);
        let view = dialog.presenter.view_model();
        assert_eq!(
            (view.ledger_name.text.as_str(), view.owner.text.as_str()),
            ("Household", "Ian")
        );
        assert_eq!(dialog.port.borrow().calls, 0);
    }

    #[test]
    fn back_on_the_base_unit_cancels_and_the_cancel_event_always_does() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.send([CreateLedgerPresenter::back_event()]);
        assert_eq!(dialog.router.closed, [None]);

        let mut dialog = on_ledger();
        dialog.send([CreateLedgerPresenter::cancel_event()]);
        assert_eq!(dialog.router.closed, [None], "Cancel on Ledger cancels");
        assert_eq!(dialog.port.borrow().calls, 0);
    }

    #[test]
    fn tab_cycles_within_the_ledger_stage() {
        let mut dialog = on_ledger();
        let focused = |dialog: &Dialog| {
            let view = dialog.presenter.view_model();
            [
                view.ledger_name.focused,
                view.owner.focused,
                view.financial_year.focused,
            ]
        };
        dialog.press([DialogInput::NextField]);
        assert_eq!(focused(&dialog), [false, true, false]);
        dialog.press([DialogInput::NextField]);
        assert_eq!(focused(&dialog), [false, false, true]);
        dialog.press([DialogInput::NextField]);
        assert_eq!(
            focused(&dialog),
            [true, false, false],
            "wraps within the stage"
        );
        dialog.press([DialogInput::PreviousField]);
        assert_eq!(focused(&dialog), [false, false, true]);
    }

    #[test]
    fn the_month_list_opens_steps_and_commits_without_creating() {
        let mut dialog = on_ledger();
        dialog.press([DialogInput::PreviousField, DialogInput::Select]);
        assert!(dialog.presenter.view_model().list.is_open());
        assert!(
            dialog.presenter.close_open_select(),
            "back closes the list first"
        );
        dialog.send([CreateLedgerEvent::ToggleMonth]);
        dialog.press([DialogInput::SelectNext, DialogInput::Select]);

        let view = dialog.presenter.view_model();
        assert!(!view.list.is_open());
        assert_eq!(view.financial_year.chosen, FinancialYearStart::new(5).ok());
        assert_eq!(dialog.port.borrow().calls, 0);
    }

    #[test]
    fn a_step_click_goes_back_or_on_only_when_next_would() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.send([
            CreateLedgerEvent::Code(String::new()),
            CreateLedgerEvent::Step(LedgerStage::Ledger),
        ]);
        assert_eq!(dialog.presenter.view_model().stage, LedgerStage::BaseUnit);

        dialog.send([
            CreateLedgerEvent::Code("AUD".into()),
            CreateLedgerEvent::Step(LedgerStage::Ledger),
        ]);
        assert_eq!(dialog.presenter.view_model().stage, LedgerStage::Ledger);
        dialog.send([CreateLedgerEvent::Step(LedgerStage::Ledger)]);
        assert_eq!(
            dialog.port.borrow().calls,
            0,
            "the current step creates nothing"
        );

        dialog.send([CreateLedgerEvent::Step(LedgerStage::BaseUnit)]);
        assert_eq!(dialog.presenter.view_model().stage, LedgerStage::BaseUnit);
        dialog.send([CreateLedgerEvent::Step(LedgerStage::BaseUnit)]);
        assert!(
            dialog.router.closed.is_empty(),
            "the current step cancels nothing"
        );
    }
}
