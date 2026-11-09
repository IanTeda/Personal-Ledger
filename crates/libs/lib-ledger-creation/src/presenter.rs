//! The Create ledger Dialog's Presenter: the base Unit as Add unit asks for it, prefilled from the
//! Locale's Currency, over the file name the person chose. Nothing is written until Create, which
//! runs the Interactor once; a cancel writes nothing. The Price Source is Manual entry, so Add
//! unit's Source, Symbol and Quoted in fields, which only matter to a Unit priced from elsewhere
//! and quoted in another, are not asked.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use lib_core::UnitId;
use lib_locale::Currency;
use lib_locale::Label as _;
use lib_units::interactor::AddUnit;
use lib_units::{Precision, PriceLink, UnitKind};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{
    Choice, DialogFrame, DialogHint, SelectList, SelectMove, SelectView, TextFieldView,
};

use crate::interactor::{CreateLedger, LedgerCreated, base_unit_for};
use crate::msg;
use crate::protocols::LedgerCreation;

/// The creation port a Dialog shares with its Client.
pub type SharedLedgerCreation = Rc<RefCell<dyn LedgerCreation>>;

/// The form's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LedgerField {
    #[default]
    Code,
    Type,
    Name,
    Precision,
}

impl LedgerField {
    const ORDER: [Self; 4] = [Self::Code, Self::Type, Self::Name, Self::Precision];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateLedgerEvent {
    /// A Dialog command or typed key: the field keys move focus, `select` and `submit` confirm a
    /// valid form (`select` opens and commits the Type list on that field), and typing edits the
    /// focused text field.
    Input(DialogInput),
    /// A text field's whole new text.
    Code(String),
    Name(String),
    Precision(String),
    /// A Type picked, closing its list.
    Kind(UnitKind),
    /// A click on the Type select: focuses it and opens or closes its list.
    ToggleKind,
    /// A click on a text field, which closes an open list.
    Focus(LedgerField),
    /// Closes the open list without picking: the Desktop host's `back`.
    CloseList,
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateLedgerViewModel {
    pub frame: DialogFrame,
    /// The chosen file's name, shown read-only.
    pub file: TextFieldView,
    pub code: TextFieldView,
    pub kind: SelectView<UnitKind>,
    pub name: TextFieldView,
    pub precision: TextFieldView,
    pub precision_help: String,
    /// Why the base Unit matters.
    pub help: String,
    pub list: SelectList,
}

pub struct CreateLedgerPresenter {
    creation: SharedLedgerCreation,
    path: PathBuf,
    personal: String,
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
    /// Personal Institution named `personal`.
    pub fn new(
        creation: SharedLedgerCreation,
        path: PathBuf,
        currency: Currency,
        personal: String,
    ) -> Self {
        let base = base_unit_for(currency);
        Self {
            creation,
            path,
            personal,
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
                let count = LedgerField::ORDER.len();
                let at = LedgerField::ORDER
                    .iter()
                    .position(|field| *field == self.focused)
                    .unwrap_or(0);
                let to = if input == DialogInput::NextField {
                    (at + 1) % count
                } else {
                    (at + count - 1) % count
                };
                self.focused = LedgerField::ORDER[to];
            }
            DialogInput::Submit => self.confirm(router),
            // A select swallows typing, so it never reaches a text field.
            _ if self.focused == LedgerField::Type => self.kind_input(input),
            DialogInput::Select => self.confirm(router),
            DialogInput::Char(ch) => {
                if let Some(text) = self.text_mut() {
                    text.push(ch);
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

    /// `input` on the Type select: `select` opens a closed list and commits an open one, the
    /// arrows step the value or the highlight, and `submit` alone confirms from here.
    fn kind_input(&mut self, input: DialogInput) {
        let view = self.view_model();
        if let SelectMove::Chose(kind) =
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
            LedgerField::Type => None,
        }
    }

    fn typed(&mut self) {
        if self.focused == LedgerField::Precision {
            self.precision_typed = true;
        }
    }

    fn pick_kind(&mut self, kind: UnitKind) {
        self.kind = kind;
        if !self.precision_typed {
            self.precision = Precision::of_kind(kind).to_string();
        }
    }

    /// Code, Name and a Precision in range are required, as Add unit requires them.
    fn is_valid(&self) -> bool {
        !self.code.trim().is_empty()
            && !self.name.trim().is_empty()
            && Precision::parse(&self.precision).is_some()
    }

    fn confirm(&mut self, router: &mut dyn DialogRouter<LedgerCreated>) {
        let Some(precision) = Precision::parse(&self.precision).filter(|_| self.is_valid()) else {
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
        }
        .run(&mut *self.creation.borrow_mut(), router);
        // A refusal has raised its Error Toast; the Dialog stays for another try or a cancel.
        if let Ok(created) = created {
            router.close(Some(created));
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
            CreateLedgerEvent::Code(text) => self.code = text,
            CreateLedgerEvent::Name(text) => self.name = text,
            CreateLedgerEvent::Precision(text) => {
                self.precision = text;
                self.precision_typed = true;
            }
            CreateLedgerEvent::Kind(kind) => {
                self.list.close();
                self.pick_kind(kind);
            }
            CreateLedgerEvent::ToggleKind => {
                if self.focused != LedgerField::Type {
                    self.list.close();
                    self.focused = LedgerField::Type;
                }
                let view = self.view_model();
                self.list
                    .toggle(&view.kind.options, view.kind.chosen.as_ref());
            }
            CreateLedgerEvent::Focus(field) => {
                self.list.close();
                self.focused = field;
            }
            CreateLedgerEvent::CloseList => {
                self.list.close();
            }
            CreateLedgerEvent::Confirm => self.confirm(router),
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
        CreateLedgerViewModel {
            frame: DialogFrame {
                title: msg::ledger_creation_title(),
                submit: msg::ledger_creation_submit(),
                can_confirm: self.is_valid(),
                destructive: false,
                hints: Vec::<DialogHint>::new(),
            },
            file: TextFieldView {
                label: msg::ledger_creation_field_file(),
                text: self
                    .path
                    .file_name()
                    .unwrap_or(self.path.as_os_str())
                    .to_string_lossy()
                    .into_owned(),
                placeholder: String::new(),
                focused: false,
            },
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
            (lib_locale::Locale::EnUs, "usd", "US Dollar"),
            (lib_locale::Locale::EnGb, "gbp", "British Pound"),
            (lib_locale::Locale::EnAu, "aud", "Australian Dollar"),
            (lib_locale::Locale::EnXa, "usd", "US Dollar"),
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
    fn the_dialog_shows_the_chosen_file_name() {
        let dialog = Dialog::new(Currency::Aud);
        let view = dialog.presenter.view_model();
        assert_eq!(view.frame.title, "Create ledger");
        assert_eq!(view.frame.submit, "Create");
        assert_eq!(view.file.text, "teda.pldb");
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
            assert!(
                !dialog_one.presenter.view_model().frame.can_confirm,
                "{label}"
            );
            assert_eq!(dialog_one.port.borrow().calls, 0, "{label}");
            assert!(dialog_one.router.closed.is_empty(), "{label}");
        }
        dialog.send([CreateLedgerEvent::Precision("x".into())]);
        dialog.press([DialogInput::Submit]);
        assert_eq!(dialog.port.borrow().calls, 0);
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
            CreateLedgerEvent::Code("btc".into()),
            CreateLedgerEvent::Name("Bitcoin".into()),
            CreateLedgerEvent::Kind(UnitKind::Crypto),
        ]);
        assert_eq!(dialog.presenter.view_model().precision.text, "8");

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
            ("btc", UnitKind::Crypto)
        );
    }

    #[test]
    fn enter_creates_from_a_text_field_and_submit_from_the_type_select() {
        let mut dialog = Dialog::new(Currency::Aud);
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
    fn typing_edits_the_focused_field_and_tab_wraps() {
        let mut dialog = Dialog::new(Currency::Aud);
        dialog.press([DialogInput::Backspace, DialogInput::Char('x')]);
        assert_eq!(
            dialog.presenter.view_model().code.text,
            "auX".to_lowercase()
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

        dialog.press([DialogInput::Submit]);

        assert!(dialog.router.closed.is_empty());
        assert_eq!(
            dialog.router.notices,
            [Notice::Toast(
                ToastKind::Error,
                "Couldn't create ledger teda.pldb: it can't be written there".into()
            )]
        );
        assert!(dialog.port.borrow().ledgers.is_empty());
    }
}
