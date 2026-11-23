//! The Add and Edit price source Dialogs. A Custom source is a whole [`Endpoint`], so its form
//! (16z·custom) has every field. A built-in's endpoint is code (ADR-0043), so its form (16z·edit)
//! holds only the name and Updates a Ledger stores for it, and shows the endpoint read-only.
//!
//! The API key field is always masked, a Custom source's and an optional-keyed built-in's (CoinGecko)
//! alike. A saved key shows only as the field's placeholder; blank keeps it, typed text replaces it
//! and Remove key clears it, written in the Save's commit through the [`crate::PriceSourceKeys`]
//! port, which `lib-database` encrypts (ADR-0043).

use lib_core::PriceSourceId;
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{Choice, DialogFrame, Hint, SelectView, TextFieldView, count};
use secrecy::{ExposeSecret as _, SecretString};

use super::{UnitsOutcome, cycle, step, updates_choices};
use crate::interactor::{AddPriceSource, EditPriceSource, KeyChange, SavePriceSourceKey};
use crate::protocols::SharedPorts;
use crate::{
    Endpoint, Error, KeyPlacement, KeyState, PriceSourceAuth, RequestHeader, SourceOrigin, Updates,
    msg,
};

/// The Authentication control's value: whether the source needs a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthChoice {
    #[default]
    None,
    ApiKey,
}

/// A field of the price source forms, in `Tab` order. The built-in form has Name and Updates only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PriceSourceField {
    #[default]
    Name,
    Updates,
    Address,
    Request,
    PricePath,
    /// A Custom source's fixed request headers, `Name: value` pairs with `|` between them.
    Headers,
    Auth,
    KeyParam,
    ApiKey,
    /// The Remove key button beside the API key, while a saved key can be removed.
    RemoveKey,
}

impl PriceSourceField {
    const CUSTOM_ORDER: [Self; 10] = [
        Self::Name,
        Self::Updates,
        Self::Address,
        Self::Request,
        Self::PricePath,
        Self::Headers,
        Self::Auth,
        Self::KeyParam,
        Self::ApiKey,
        Self::RemoveKey,
    ];
    const BUILT_IN_ORDER: [Self; 4] = [Self::Name, Self::Updates, Self::ApiKey, Self::RemoveKey];
}

/// Not `PartialEq`: [`Self::ApiKey`] holds a secret, which can't be compared or printed.
#[derive(Debug, Clone)]
pub enum PriceSourceFormEvent {
    /// A Dialog command or typed key, as the Desktop sends them. What it means is the focused
    /// field's: `select` and `submit` confirm a valid form; `Left`/`Right` step Updates and
    /// Authentication, which swallow typing; typing edits a text field, the API key included, held
    /// as a secret; the field keys move focus, wrapping and skipping hidden fields.
    Input(DialogInput),
    /// A text field's whole new text, as its View's input holds it. Ignored for a control and for
    /// the API key, which comes as [`Self::ApiKey`].
    Text(PriceSourceField, String),
    /// The API key field's whole new text.
    ApiKey(SecretString),
    /// Remove key: the saved key goes on Save, and the field empties.
    RemoveKey,
    Updates(Updates),
    Auth(AuthChoice),
    /// `Left`/`Right` on the focused control (`-1`/`1`), stopping at either end; with
    /// `NextField` and `PrevField`, the TUI's, which reads its own keys until its redesign (#805).
    Step(isize),
    NextField,
    PrevField,
    Focus(PriceSourceField),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceSourceFormViewModel {
    pub frame: DialogFrame,
    pub name: TextFieldView,
    pub name_help: String,
    pub updates: SelectView<Updates>,
    /// A Custom source's endpoint fields; `None` for a built-in.
    pub endpoint: Option<EndpointFields>,
    /// A built-in's endpoint as read-only facts, `(label, value)`; empty for a Custom source.
    pub built_in_facts: Vec<(String, String)>,
    /// A built-in's note that its address isn't edited here.
    pub built_in_help: Option<String>,
    /// For an edit: `Values 1 unit: btc.`
    pub usage: Option<String>,
    /// The API key field: a Custom source's with API key authentication, or a built-in's that takes
    /// a key.
    pub api_key: Option<ApiKeyView>,
    /// The Test button's label, and the note that testing isn't built yet.
    pub test: String,
    pub test_note: String,
    /// The fields on screen, in `Tab` order.
    pub fields: Vec<PriceSourceField>,
    pub refusal: Option<String>,
    /// The status line's legend while the Dialog is open.
    pub hints: Vec<Hint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointFields {
    pub address: TextFieldView,
    pub request: TextFieldView,
    pub request_help: String,
    pub price_path: TextFieldView,
    pub price_path_help: String,
    /// The fixed headers as one line, `Accept: application/json | X-Client: ledger`.
    pub headers: TextFieldView,
    pub headers_help: String,
    pub auth: SelectView<AuthChoice>,
    /// The key parameter's name, shown only with API key authentication; the key itself is the
    /// form's [`PriceSourceFormViewModel::api_key`].
    pub key_param: Option<TextFieldView>,
}

/// The masked API key field, which never shows a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiKeyView {
    /// Its text is the typed key masked, one `•` per character; with nothing typed over a saved
    /// key, its placeholder says *Key saved · leave blank to keep it*.
    pub field: TextFieldView,
    /// Off on a Ledger whose keys another device's API key secret saved: it takes no typing.
    pub enabled: bool,
    /// Remove key's label, while there is a saved key to remove.
    pub remove: Option<String>,
    pub remove_focused: bool,
    /// Where the key is kept, or why the field is off.
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    AddCustom,
    EditCustom(PriceSourceId),
    EditBuiltIn(PriceSourceId),
}

pub struct PriceSourceFormPresenter {
    ports: SharedPorts,
    keys: DialogKeys,
    mode: Mode,
    name: String,
    updates: Updates,
    address: String,
    request: String,
    price_path: String,
    headers: String,
    auth: AuthChoice,
    key_param: String,
    /// Typed over any saved key, stored on Save only when not blank.
    api_key: SecretString,
    /// What the Ledger held for the source's key when the form opened.
    saved_key: KeyState,
    /// Remove key was pressed: the saved key goes on Save.
    remove_key: bool,
    /// Whether this device may write the Ledger's keys (one secret per Ledger, ADR-0043).
    keys_writable: bool,
    /// Where the key travels. The form names only a parameter, so a new source puts it there and
    /// an edited one keeps what it had.
    placement: KeyPlacement,
    focused: PriceSourceField,
    refusal: Option<String>,
}

impl PriceSourceFormPresenter {
    /// An empty Add price source form, for a Custom source: the built-ins are always listed, so
    /// there is none to add.
    /// `keys` are the Dialog keymap's chords, which its hints show.
    pub fn add(ports: SharedPorts, keys: DialogKeys) -> Self {
        let keys_writable = ports.borrow().keys().can_write();
        Self {
            ports,
            keys,
            mode: Mode::AddCustom,
            name: String::new(),
            updates: Updates::Daily,
            address: String::new(),
            request: String::new(),
            price_path: String::new(),
            headers: String::new(),
            auth: AuthChoice::None,
            key_param: String::new(),
            api_key: SecretString::default(),
            saved_key: KeyState::NotSet,
            remove_key: false,
            keys_writable,
            placement: KeyPlacement::Param,
            focused: PriceSourceField::Name,
            refusal: None,
        }
    }

    /// The Edit form for source `id`: a built-in's name, Updates and any key it takes, or a Custom
    /// source's every field, its key only as whether one is saved. `None` for Manual entry, which
    /// has nothing to edit, or a source that is gone.
    pub fn edit(ports: SharedPorts, id: PriceSourceId, keys: DialogKeys) -> Option<Self> {
        let mut form = Self::add(ports.clone(), keys);
        let ports = ports.borrow();
        let catalogue = ports.source_catalogue();
        let source = catalogue.source(id)?;
        form.name = source.name.to_string();
        form.updates = source.updates;
        form.saved_key = ports.keys().key_state(id);
        match source.origin {
            SourceOrigin::ManualEntry => return None,
            SourceOrigin::BuiltIn(_) => form.mode = Mode::EditBuiltIn(id),
            SourceOrigin::Custom(custom) => {
                let endpoint = &custom.endpoint;
                form.mode = Mode::EditCustom(id);
                form.address = endpoint.address.to_string();
                form.request = endpoint.request.to_string();
                form.price_path = endpoint.price_path.to_string();
                form.headers = headers_text(&endpoint.headers);
                if let PriceSourceAuth::ApiKey {
                    placement, name, ..
                } = &endpoint.auth
                {
                    form.auth = AuthChoice::ApiKey;
                    form.key_param = name.to_string();
                    form.placement = *placement;
                }
            }
        }
        drop(ports);
        Some(form)
    }
}

impl Presenter for PriceSourceFormPresenter {
    type Event = PriceSourceFormEvent;
    type ViewModel = PriceSourceFormViewModel;
    type Router<'r> = dyn DialogRouter<UnitsOutcome> + 'r;

    fn handle(&mut self, event: PriceSourceFormEvent, router: &mut Self::Router<'_>) {
        match event {
            PriceSourceFormEvent::Input(input) => self.input(input, router),
            PriceSourceFormEvent::Text(field, text) => {
                if let Some(slot) = self.text_mut(field) {
                    *slot = text;
                }
            }
            PriceSourceFormEvent::ApiKey(key) => {
                if self.focusable(PriceSourceField::ApiKey) {
                    self.api_key = key;
                }
            }
            PriceSourceFormEvent::RemoveKey => self.remove_key(),
            PriceSourceFormEvent::Updates(updates) => self.updates = updates,
            PriceSourceFormEvent::Auth(auth) if self.is_custom() => self.set_auth(auth),
            PriceSourceFormEvent::Auth(_) => {}
            PriceSourceFormEvent::Step(delta) => self.step_focused(delta),
            PriceSourceFormEvent::NextField => self.cycle_focus(false),
            PriceSourceFormEvent::PrevField => self.cycle_focus(true),
            PriceSourceFormEvent::Focus(field) => {
                if self.focusable(field) {
                    self.focused = field;
                }
            }
            PriceSourceFormEvent::Confirm => self.save(router),
            PriceSourceFormEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> PriceSourceFormViewModel {
        let text = |label, text: &str, placeholder, field| TextFieldView {
            label,
            text: text.to_string(),
            placeholder,
            focused: self.focused == field,
        };
        let (title, submit) = match self.mode {
            Mode::AddCustom => (
                msg::units_price_source_add_title(),
                msg::units_price_source_add_submit(),
            ),
            Mode::EditCustom(_) | Mode::EditBuiltIn(_) => (
                msg::units_price_source_edit_title(&self.name),
                msg::units_price_source_edit_submit(),
            ),
        };
        let keyed = self.auth == AuthChoice::ApiKey;
        let api_key = self.keyed().then(|| self.api_key_view());
        let endpoint = self.is_custom().then(|| EndpointFields {
            address: text(
                msg::units_price_source_field_address(),
                &self.address,
                msg::units_price_source_address_placeholder(),
                PriceSourceField::Address,
            ),
            request: text(
                msg::units_price_source_field_request(),
                &self.request,
                msg::units_price_source_request_placeholder(),
                PriceSourceField::Request,
            ),
            request_help: msg::units_price_source_request_help(),
            price_path: text(
                msg::units_price_source_field_price_path(),
                &self.price_path,
                msg::units_price_source_price_path_placeholder(),
                PriceSourceField::PricePath,
            ),
            price_path_help: msg::units_price_source_price_path_help(),
            headers: text(
                msg::units_price_source_field_headers(),
                &self.headers,
                msg::units_price_source_headers_placeholder(),
                PriceSourceField::Headers,
            ),
            headers_help: msg::units_price_source_headers_help(),
            auth: SelectView {
                label: msg::units_price_source_field_auth(),
                options: vec![
                    Choice {
                        value: AuthChoice::None,
                        label: msg::units_price_source_auth_none(),
                    },
                    Choice {
                        value: AuthChoice::ApiKey,
                        label: msg::units_price_source_auth_api_key(),
                    },
                ],
                chosen: Some(self.auth),
                focused: self.focused == PriceSourceField::Auth,
            },
            key_param: keyed.then(|| {
                text(
                    msg::units_price_source_field_key_param(),
                    &self.key_param,
                    msg::units_price_source_key_param_placeholder(),
                    PriceSourceField::KeyParam,
                )
            }),
        });
        let (built_in_facts, built_in_help) = match self.mode {
            Mode::EditBuiltIn(id) => (
                self.built_in_facts(id),
                Some(msg::units_price_source_built_in_help()),
            ),
            Mode::AddCustom | Mode::EditCustom(_) => (Vec::new(), None),
        };
        PriceSourceFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.is_valid(),
                destructive: false,
                hints: super::form_hints(),
            },
            hints: super::dialog_legend(&self.keys, &super::form_hints()),
            name: text(
                lib_locale::msg::column_name(),
                &self.name,
                msg::units_price_source_name_placeholder(),
                PriceSourceField::Name,
            ),
            name_help: msg::units_price_source_name_help(),
            updates: SelectView {
                label: msg::units_price_source_field_updates(),
                options: updates_choices(),
                chosen: Some(self.updates),
                focused: self.focused == PriceSourceField::Updates,
            },
            endpoint,
            built_in_facts,
            built_in_help,
            usage: self.usage(),
            api_key,
            test: if self.is_custom() {
                msg::units_price_source_test_with_symbol()
            } else {
                msg::units_price_source_test()
            },
            test_note: msg::units_price_source_test_not_built(),
            fields: self
                .order()
                .iter()
                .copied()
                .filter(|field| self.focusable(*field))
                .collect(),
            refusal: self.refusal.clone(),
        }
    }
}

impl DialogPresenter for PriceSourceFormPresenter {
    type Outcome = UnitsOutcome;

    fn input_event(input: DialogInput) -> PriceSourceFormEvent {
        PriceSourceFormEvent::Input(input)
    }

    fn confirm_event() -> PriceSourceFormEvent {
        PriceSourceFormEvent::Confirm
    }

    fn cancel_event() -> PriceSourceFormEvent {
        PriceSourceFormEvent::Cancel
    }

    fn can_confirm(&self) -> bool {
        self.is_valid()
    }
}

impl PriceSourceFormPresenter {
    /// Reads `input` for the focused field, as the Desktop sends it.
    fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<UnitsOutcome>) {
        match (input, self.focused) {
            (DialogInput::NextField, _) => self.cycle_focus(false),
            (DialogInput::PreviousField, _) => self.cycle_focus(true),
            (DialogInput::Select, PriceSourceField::RemoveKey) => self.remove_key(),
            (DialogInput::Select | DialogInput::Submit, _) => self.save(router),
            (DialogInput::Left, _) => self.step_focused(-1),
            (DialogInput::Right, _) => self.step_focused(1),
            (DialogInput::Char(ch), PriceSourceField::ApiKey) => {
                let mut key = self.api_key.expose_secret().to_string();
                key.push(ch);
                self.api_key = SecretString::from(key);
            }
            (DialogInput::Backspace, PriceSourceField::ApiKey) => {
                let mut key = self.api_key.expose_secret().to_string();
                key.pop();
                self.api_key = SecretString::from(key);
            }
            // A control swallows typing, so it never reaches a text field.
            (DialogInput::Char(ch), field) => {
                if let Some(text) = self.text_mut(field) {
                    text.push(ch);
                }
            }
            (DialogInput::Backspace, field) => {
                if let Some(text) = self.text_mut(field) {
                    text.pop();
                }
            }
            _ => {}
        }
    }

    /// `delta` on the focused control, stopping at either end.
    fn step_focused(&mut self, delta: isize) {
        match self.focused {
            PriceSourceField::Updates => {
                let options: Vec<_> = updates_choices().iter().map(|c| c.value).collect();
                self.updates = step(&options, self.updates, delta);
            }
            PriceSourceField::Auth => {
                let auth = step(&[AuthChoice::None, AuthChoice::ApiKey], self.auth, delta);
                self.set_auth(auth);
            }
            _ => {}
        }
    }

    fn cycle_focus(&mut self, backward: bool) {
        self.focused = cycle(self.order(), self.focused, backward, |f| self.focusable(f));
    }

    fn is_custom(&self) -> bool {
        !matches!(self.mode, Mode::EditBuiltIn(_))
    }

    fn order(&self) -> &'static [PriceSourceField] {
        if self.is_custom() {
            &PriceSourceField::CUSTOM_ORDER
        } else {
            &PriceSourceField::BUILT_IN_ORDER
        }
    }

    /// Whether the source takes a key: a Custom source with API key authentication chosen, or a
    /// built-in whose endpoint names one (CoinGecko's optional Demo key).
    fn keyed(&self) -> bool {
        match self.mode {
            Mode::AddCustom | Mode::EditCustom(_) => self.auth == AuthChoice::ApiKey,
            Mode::EditBuiltIn(id) => crate::built_in(id).is_some_and(|source| {
                matches!(source.endpoint.auth, PriceSourceAuth::ApiKey { .. })
            }),
        }
    }

    /// Whether a saved key would still be there after Save, as the form stands.
    fn key_kept(&self) -> bool {
        self.saved_key != KeyState::NotSet && !self.remove_key
    }

    /// Whether `field` is drawn: the key parameter only while a Custom source's API key
    /// authentication is chosen, the key while the source takes one, and Remove key while a saved
    /// key can be removed.
    fn shows(&self, field: PriceSourceField) -> bool {
        if !self.order().contains(&field) {
            return false;
        }
        match field {
            PriceSourceField::KeyParam => self.auth == AuthChoice::ApiKey,
            PriceSourceField::ApiKey => self.keyed(),
            PriceSourceField::RemoveKey => self.keyed() && self.key_kept(),
            _ => true,
        }
    }

    /// Whether `field` takes focus: drawn, and for the key and Remove key, writable here.
    fn focusable(&self, field: PriceSourceField) -> bool {
        self.shows(field)
            && (self.keys_writable
                || !matches!(
                    field,
                    PriceSourceField::ApiKey | PriceSourceField::RemoveKey
                ))
    }

    fn set_auth(&mut self, auth: AuthChoice) {
        self.auth = auth;
        if !self.focusable(self.focused) {
            self.focused = PriceSourceField::Auth;
        }
    }

    /// Marks the saved key to go on Save and empties the field, focus moving onto it.
    fn remove_key(&mut self) {
        if !self.focusable(PriceSourceField::RemoveKey) {
            return;
        }
        self.remove_key = true;
        self.api_key = SecretString::default();
        self.focused = PriceSourceField::ApiKey;
    }

    fn api_key_view(&self) -> ApiKeyView {
        let note = if !self.keys_writable {
            msg::units_price_source_key_locked()
        } else if let Mode::EditBuiltIn(id) = self.mode {
            msg::units_price_source_key_optional_note(
                crate::built_in(id).map_or("", |source| source.name),
            )
        } else {
            msg::units_price_source_key_note()
        };
        ApiKeyView {
            field: TextFieldView {
                label: msg::units_price_source_field_api_key(),
                text: "\u{2022}".repeat(self.api_key.expose_secret().chars().count()),
                placeholder: if self.key_kept() {
                    msg::units_price_source_key_saved_placeholder()
                } else {
                    String::new()
                },
                focused: self.focused == PriceSourceField::ApiKey,
            },
            enabled: self.keys_writable,
            remove: self
                .focusable(PriceSourceField::RemoveKey)
                .then(msg::units_price_source_remove_key),
            remove_focused: self.focused == PriceSourceField::RemoveKey,
            note,
        }
    }

    /// What Save does to the key: none taken removes any saved one (a Custom source switched to
    /// no authentication), typed text replaces it, Remove key removes it, and blank keeps it.
    fn key_change(&self) -> KeyChange {
        let typed = self.api_key.expose_secret().trim();
        if !self.keyed() {
            KeyChange::Remove
        } else if !typed.is_empty() && self.keys_writable {
            KeyChange::Replace(SecretString::from(typed.to_string()))
        } else if self.remove_key {
            KeyChange::Remove
        } else {
            KeyChange::Keep
        }
    }

    fn text_mut(&mut self, field: PriceSourceField) -> Option<&mut String> {
        if !self.shows(field) {
            return None;
        }
        Some(match field {
            PriceSourceField::Name => &mut self.name,
            PriceSourceField::Address => &mut self.address,
            PriceSourceField::Request => &mut self.request,
            PriceSourceField::PricePath => &mut self.price_path,
            PriceSourceField::Headers => &mut self.headers,
            PriceSourceField::KeyParam => &mut self.key_param,
            PriceSourceField::ApiKey
            | PriceSourceField::RemoveKey
            | PriceSourceField::Updates
            | PriceSourceField::Auth => {
                return None;
            }
        })
    }

    /// Name is required; a Custom source also needs its Address, Request and Price path, any
    /// Headers well formed, and with API key authentication the Key parameter and, on a new source
    /// on a Ledger this device can write keys to, the key itself.
    fn is_valid(&self) -> bool {
        let filled = |text: &String| !text.trim().is_empty();
        if !filled(&self.name) {
            return false;
        }
        if !self.is_custom() {
            return true;
        }
        if ![&self.address, &self.request, &self.price_path]
            .into_iter()
            .all(filled)
        {
            return false;
        }
        if parse_headers(&self.headers).is_none() {
            return false;
        }
        match self.auth {
            AuthChoice::None => true,
            AuthChoice::ApiKey => {
                filled(&self.key_param)
                    && (matches!(self.mode, Mode::EditCustom(_))
                        || !self.keys_writable
                        || !self.api_key.expose_secret().trim().is_empty())
            }
        }
    }

    /// The endpoint the fields describe. The API key is not part of it.
    fn endpoint(&self) -> Endpoint {
        let trimmed = |text: &String| text.trim().to_string().into();
        Endpoint {
            address: trimmed(&self.address),
            request: trimmed(&self.request),
            price_path: trimmed(&self.price_path),
            auth: match self.auth {
                AuthChoice::None => PriceSourceAuth::None,
                AuthChoice::ApiKey => PriceSourceAuth::ApiKey {
                    placement: self.placement,
                    name: trimmed(&self.key_param),
                    optional: false,
                },
            },
            // Saving checks they parse first.
            headers: parse_headers(&self.headers).unwrap_or_default().into(),
        }
    }

    fn built_in_facts(&self, id: PriceSourceId) -> Vec<(String, String)> {
        let Some(source) = crate::built_in(id) else {
            return Vec::new();
        };
        let endpoint = &source.endpoint;
        vec![
            (
                msg::units_column_source(),
                msg::units_price_source_built_in(source.name),
            ),
            (
                msg::units_price_source_field_address(),
                endpoint.address.to_string(),
            ),
            (
                msg::units_price_source_field_request(),
                endpoint.request.to_string(),
            ),
            (
                msg::units_price_source_field_price_path(),
                endpoint.price_path.to_string(),
            ),
        ]
    }

    /// The Units an edited source values, by code.
    fn usage(&self) -> Option<String> {
        let id = match self.mode {
            Mode::AddCustom => return None,
            Mode::EditCustom(id) | Mode::EditBuiltIn(id) => id,
        };
        let codes = super::linked_codes(&*self.ports.borrow(), id);
        Some(msg::units_price_source_usage(
            count(codes.len()),
            &codes.join(", "),
        ))
    }

    /// Saves the source, then its key in the same commit. A key the port refuses leaves the Dialog
    /// open on the saved source, as an Edit, so trying again doesn't add the source twice.
    fn save(&mut self, router: &mut dyn DialogRouter<UnitsOutcome>) {
        if !self.is_valid() {
            return;
        }
        let name = self.name.trim().to_string();
        let change = self.key_change();
        let saved = {
            let mut ports = self.ports.borrow_mut();
            let price_sources = ports.split_mut().price_sources;
            let source = match self.mode {
                Mode::AddCustom => {
                    let added = AddPriceSource {
                        name: name.clone(),
                        endpoint: self.endpoint(),
                        updates: self.updates,
                    }
                    .run(price_sources);
                    self.mode = Mode::EditCustom(added.id);
                    Ok(added.id)
                }
                Mode::EditCustom(id) | Mode::EditBuiltIn(id) => EditPriceSource {
                    id,
                    name: name.clone(),
                    updates: self.updates,
                    endpoint: self.is_custom().then(|| self.endpoint()),
                }
                .run(price_sources)
                .map(|_| id),
            };
            source.and_then(|source| {
                SavePriceSourceKey { source, change }
                    .run(ports.keys_mut())
                    .map(|stored| stored.then_some(source))
            })
        };
        self.refusal = match saved {
            Ok(key_stored) => {
                router.close(Some(UnitsOutcome::SourceSaved { name, key_stored }));
                None
            }
            Err(Error::ChangeManualEntry) => Some(msg::units_refusal_manual_entry()),
            Err(Error::BuiltInEndpoint) => Some(msg::units_refusal_built_in_endpoint()),
            Err(Error::PriceSourceGone) => Some(msg::units_refusal_price_source_gone()),
            Err(Error::KeysLocked) => Some(msg::units_refusal_keys_locked()),
            Err(Error::KeyNotSaved) => Some(msg::units_refusal_key_not_saved()),
            Err(
                error @ (Error::UnitGone
                | Error::QuoteGone
                | Error::OnlyUnit
                | Error::UnitInUse(_)
                | Error::BaseUnitNeeded
                | Error::NewBaseGone
                | Error::InactiveBase
                | Error::QuotedBy(_)
                | Error::DeactivateBase
                | Error::DeleteBuiltIn
                | Error::PricedBySource
                | Error::NotFetched
                | Error::KeyMissing
                | Error::AlreadyFetching
                | Error::FutureWeek),
            ) => {
                debug_assert!(false, "saving a price source doesn't return {error:?}");
                tracing::error!(
                    error = ?error,
                    "Saving a price source was refused with an error it never returns"
                );
                Some(msg::units_refusal_price_source_gone())
            }
        };
    }
}

/// Between two headers in the Headers field: rarely in a header's value, unlike `;` and `,`, which
/// a `User-Agent` or `Accept` holds.
const HEADER_SEPARATOR: char = '|';

/// The Headers field's text as headers, or `None` when a pair has no `:` or its name is blank or
/// holds a space. Blank text is no headers.
fn parse_headers(text: &str) -> Option<Vec<RequestHeader>> {
    text.split(HEADER_SEPARATOR)
        .map(str::trim)
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once(':')?;
            let name = name.trim();
            (!name.is_empty() && !name.contains(char::is_whitespace)).then(|| RequestHeader {
                name: name.to_string().into(),
                value: value.trim().to_string().into(),
            })
        })
        .collect()
}

/// `headers` as the Headers field shows them.
fn headers_text(headers: &[RequestHeader]) -> String {
    headers
        .iter()
        .map(|header| format!("{}: {}", header.name, header.value))
        .collect::<Vec<_>>()
        .join(&format!(" {HEADER_SEPARATOR} "))
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{hints, pairs, sample};
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::fake::{InMemoryPorts, RecordingRouter, RouterCall};
    use crate::protocols::PriceSourceKeys as _;
    use crate::{BuiltInOverride, COINGECKO, CustomSource, MANUAL_ENTRY, PriceSourceRecord};

    fn send(
        form: &mut PriceSourceFormPresenter,
        events: Vec<PriceSourceFormEvent>,
    ) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            form.handle(event, &mut router);
        }
        router.calls
    }

    fn typed(field: PriceSourceField, text: &str) -> PriceSourceFormEvent {
        PriceSourceFormEvent::Text(field, text.to_string())
    }

    fn filled() -> Vec<PriceSourceFormEvent> {
        vec![
            typed(PriceSourceField::Name, " Alpha Vantage "),
            typed(PriceSourceField::Address, "https://www.alphavantage.co"),
            typed(PriceSourceField::Request, "/query?symbol={symbol}"),
            typed(PriceSourceField::PricePath, "/Global Quote/05. price"),
        ]
    }

    fn custom_records(ports: &InMemoryPorts) -> Vec<CustomSource> {
        ports
            .price_sources
            .records
            .iter()
            .filter_map(|record| match record {
                PriceSourceRecord::Custom(source) => Some(source.clone()),
                PriceSourceRecord::BuiltIn(_) => None,
            })
            .collect()
    }

    #[test]
    fn a_custom_source_needs_its_name_address_request_and_price_path_and_saves_trimmed() {
        let (ports, shared) = sample();
        let mut form = PriceSourceFormPresenter::add(shared, DialogKeys::default());
        assert!(!form.view_model().frame.can_confirm);
        assert_eq!(form.view_model().frame.title, "Add price source");

        send(&mut form, filled()[..3].to_vec());
        assert!(
            !form.view_model().frame.can_confirm,
            "Price path is still blank"
        );
        let mut events = filled();
        events.push(PriceSourceFormEvent::Confirm);
        let calls = send(&mut form, events);

        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::SourceSaved {
                name: "Alpha Vantage".into(),
                key_stored: None,
            }))]
        );
        let saved = custom_records(&ports.borrow());
        assert_eq!(saved[0].name, "Alpha Vantage");
        assert_eq!(saved[0].endpoint.auth, PriceSourceAuth::None);
    }

    #[test]
    fn a_custom_sources_headers_are_saved_as_entered_and_prefill_its_edit_form() {
        let (ports, shared) = sample();
        let mut form = PriceSourceFormPresenter::add(shared.clone(), DialogKeys::default());
        let mut events = filled();
        events.push(typed(
            PriceSourceField::Headers,
            " Accept: application/json | X-Client: ledger; v1 ",
        ));
        events.push(PriceSourceFormEvent::Confirm);
        let calls = send(&mut form, events);

        assert_eq!(calls.len(), 1, "saved");
        let saved = custom_records(&ports.borrow());
        let header = |name: &str, value: &str| RequestHeader {
            name: name.to_string().into(),
            value: value.to_string().into(),
        };
        assert_eq!(
            saved[0].endpoint.headers.as_ref(),
            [
                header("Accept", "application/json"),
                header("X-Client", "ledger; v1")
            ]
        );

        let edit = PriceSourceFormPresenter::edit(shared, saved[0].id, DialogKeys::default())
            .expect("a Custom source");
        assert_eq!(
            edit.view_model().endpoint.expect("custom").headers.text,
            "Accept: application/json | X-Client: ledger; v1"
        );
    }

    #[test]
    fn a_header_with_no_name_or_no_colon_keeps_the_form_from_saving() {
        let (_, shared) = sample();
        let mut form = PriceSourceFormPresenter::add(shared, DialogKeys::default());
        send(&mut form, filled());
        assert!(form.view_model().frame.can_confirm, "headers are optional");

        for bad in ["Accept", ": json", "Bad Name: x", "Accept: json | nope"] {
            send(&mut form, vec![typed(PriceSourceField::Headers, bad)]);
            assert!(!form.view_model().frame.can_confirm, "{bad}");
        }
    }

    #[test]
    fn api_key_authentication_shows_the_key_fields_and_needs_them_on_a_new_source() {
        let (ports, shared) = sample();
        let mut form = PriceSourceFormPresenter::add(shared, DialogKeys::default());
        send(&mut form, filled());
        assert!(form.view_model().endpoint.unwrap().key_param.is_none());

        send(
            &mut form,
            vec![
                PriceSourceFormEvent::Focus(PriceSourceField::Auth),
                PriceSourceFormEvent::Step(1),
                typed(PriceSourceField::KeyParam, "apikey"),
            ],
        );
        let view = form.view_model();
        assert!(!view.frame.can_confirm, "a new source needs the key itself");
        assert!(view.fields.contains(&PriceSourceField::ApiKey));

        send(
            &mut form,
            vec![
                PriceSourceFormEvent::ApiKey("secret".into()),
                PriceSourceFormEvent::Confirm,
            ],
        );
        assert_eq!(form.view_model().api_key.unwrap().field.text, "••••••");
        let saved = custom_records(&ports.borrow());
        assert_eq!(
            ports
                .borrow()
                .keys
                .exposed()
                .get(&saved[0].id)
                .map(String::as_str),
            Some("secret"),
            "the key goes through the keys port, in the same Save"
        );
        assert_eq!(
            saved[0].endpoint.auth,
            PriceSourceAuth::ApiKey {
                placement: KeyPlacement::Param,
                name: "apikey".into(),
                optional: false,
            },
            "only the key's parameter name is stored"
        );
    }

    #[test]
    fn tab_skips_the_hidden_key_fields_and_choosing_none_moves_focus_off_them() {
        let (_, shared) = sample();
        let mut form = PriceSourceFormPresenter::add(shared, DialogKeys::default());
        send(
            &mut form,
            vec![PriceSourceFormEvent::Focus(PriceSourceField::Auth)],
        );
        send(&mut form, vec![PriceSourceFormEvent::NextField]);
        assert!(
            form.view_model().name.focused,
            "the hidden key fields are skipped"
        );

        send(
            &mut form,
            vec![
                PriceSourceFormEvent::Auth(AuthChoice::ApiKey),
                PriceSourceFormEvent::Focus(PriceSourceField::ApiKey),
                PriceSourceFormEvent::Auth(AuthChoice::None),
            ],
        );
        assert!(form.view_model().endpoint.unwrap().auth.focused);
    }

    #[test]
    fn editing_a_built_in_saves_only_its_name_and_updates_and_shows_its_endpoint_read_only() {
        let (ports, shared) = sample();
        ports.borrow_mut().units.units[1].link.source_id = COINGECKO;
        let mut form =
            PriceSourceFormPresenter::edit(shared, COINGECKO, DialogKeys::default()).unwrap();
        let view = form.view_model();
        assert_eq!(view.frame.title, "Edit price source — CoinGecko");
        assert_eq!(view.endpoint, None);
        assert_eq!(
            view.fields,
            [
                PriceSourceField::Name,
                PriceSourceField::Updates,
                PriceSourceField::ApiKey
            ],
            "and its optional Demo key"
        );
        assert_eq!(view.built_in_facts[0].1, "CoinGecko · built-in");
        assert_eq!(view.usage.as_deref(), Some("Values 1 unit: BTC."));

        send(
            &mut form,
            vec![
                typed(PriceSourceField::Name, "Coins"),
                PriceSourceFormEvent::Focus(PriceSourceField::Updates),
                PriceSourceFormEvent::Step(-1),
                PriceSourceFormEvent::Step(-1),
                PriceSourceFormEvent::Confirm,
            ],
        );

        assert_eq!(
            ports.borrow().price_sources.records,
            [PriceSourceRecord::BuiltIn(BuiltInOverride {
                id: COINGECKO,
                name: "Coins".into(),
                updates: Updates::Every15Minutes,
            })],
            "Every 15 min is the first option, so stepping stops there"
        );
    }

    #[test]
    fn editing_a_keyed_custom_source_shows_its_saved_key_only_as_the_placeholder_and_blank_keeps_it()
     {
        let (ports, shared) = sample();
        let mut add = PriceSourceFormPresenter::add(shared.clone(), DialogKeys::default());
        let mut events = filled();
        events.extend([
            PriceSourceFormEvent::Auth(AuthChoice::ApiKey),
            typed(PriceSourceField::KeyParam, "apikey"),
            PriceSourceFormEvent::ApiKey("secret".into()),
            PriceSourceFormEvent::Confirm,
        ]);
        send(&mut add, events);
        let id = custom_records(&ports.borrow())[0].id;

        let mut edit = PriceSourceFormPresenter::edit(shared, id, DialogKeys::default()).unwrap();
        let view = edit.view_model();
        let endpoint = view.endpoint.unwrap();
        assert_eq!(endpoint.address.text, "https://www.alphavantage.co");
        assert_eq!(endpoint.key_param.unwrap().text, "apikey");
        let key = view.api_key.expect("a keyed source");
        assert_eq!(key.field.text, "", "the saved key is never shown");
        assert_eq!(key.field.placeholder, "Key saved · leave blank to keep it");
        assert_eq!(key.remove.as_deref(), Some("Remove key"));
        assert!(view.frame.can_confirm);

        let calls = send(
            &mut edit,
            vec![
                typed(PriceSourceField::Name, "AV"),
                PriceSourceFormEvent::Confirm,
            ],
        );
        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::SourceSaved {
                name: "AV".into(),
                key_stored: None,
            }))]
        );
        assert_eq!(
            ports.borrow().keys.exposed().get(&id).map(String::as_str),
            Some("secret"),
            "blank keeps the saved key"
        );
    }

    /// A Custom source on API key authentication with key `k3y` saved, and its id.
    fn keyed_source(ports: &Rc<RefCell<InMemoryPorts>>, shared: &SharedPorts) -> PriceSourceId {
        let mut add = PriceSourceFormPresenter::add(shared.clone(), DialogKeys::default());
        let mut events = filled();
        events.extend([
            PriceSourceFormEvent::Auth(AuthChoice::ApiKey),
            typed(PriceSourceField::KeyParam, "apikey"),
            PriceSourceFormEvent::ApiKey("k3y".into()),
            PriceSourceFormEvent::Confirm,
        ]);
        send(&mut add, events);
        custom_records(&ports.borrow())[0].id
    }

    #[test]
    fn typed_text_replaces_a_saved_key_on_save_and_says_a_key_was_stored() {
        let (ports, shared) = sample();
        let id = keyed_source(&ports, &shared);
        let mut edit = PriceSourceFormPresenter::edit(shared, id, DialogKeys::default()).unwrap();

        let calls = send(
            &mut edit,
            vec![
                PriceSourceFormEvent::ApiKey(" n3w ".into()),
                PriceSourceFormEvent::Confirm,
            ],
        );

        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::SourceSaved {
                name: "Alpha Vantage".into(),
                key_stored: Some(id),
            }))]
        );
        assert_eq!(ports.borrow().keys.exposed()[&id], "n3w");
    }

    #[test]
    fn remove_key_empties_the_field_and_save_removes_the_key() {
        let (ports, shared) = sample();
        let id = keyed_source(&ports, &shared);
        let mut edit = PriceSourceFormPresenter::edit(shared, id, DialogKeys::default()).unwrap();
        send(
            &mut edit,
            vec![
                PriceSourceFormEvent::Focus(PriceSourceField::RemoveKey),
                PriceSourceFormEvent::Input(DialogInput::Select),
            ],
        );

        let key = edit.view_model().api_key.unwrap();
        assert_eq!(key.field.placeholder, "", "no key is kept now");
        assert_eq!(key.remove, None);
        assert!(key.field.focused, "focus moves onto the emptied field");
        assert_eq!(
            ports.borrow().keys.exposed().len(),
            1,
            "nothing is removed before Save"
        );

        send(&mut edit, vec![PriceSourceFormEvent::Confirm]);
        assert!(ports.borrow().keys.keys.is_empty());
    }

    #[test]
    fn switching_a_custom_source_to_no_authentication_removes_its_key_on_save() {
        let (ports, shared) = sample();
        let id = keyed_source(&ports, &shared);
        let mut edit = PriceSourceFormPresenter::edit(shared, id, DialogKeys::default()).unwrap();

        send(
            &mut edit,
            vec![
                PriceSourceFormEvent::Auth(AuthChoice::None),
                PriceSourceFormEvent::Confirm,
            ],
        );

        assert!(ports.borrow().keys.keys.is_empty());
        assert_eq!(
            custom_records(&ports.borrow())[0].endpoint.auth,
            PriceSourceAuth::None
        );
    }

    #[test]
    fn coingeckos_edit_dialog_takes_its_optional_demo_key() {
        let (ports, shared) = sample();
        let mut form =
            PriceSourceFormPresenter::edit(shared.clone(), COINGECKO, DialogKeys::default())
                .unwrap();
        let view = form.view_model();
        assert_eq!(
            view.fields,
            [
                PriceSourceField::Name,
                PriceSourceField::Updates,
                PriceSourceField::ApiKey
            ]
        );
        let key = view.api_key.expect("CoinGecko takes a key");
        assert_eq!(key.field.placeholder, "");
        assert_eq!(
            key.note,
            "Optional. Without a key CoinGecko is asked keyless, at a lower call limit."
        );

        let mut events = vec![PriceSourceFormEvent::Focus(PriceSourceField::ApiKey)];
        events.extend(
            "CG-1"
                .chars()
                .map(|ch| PriceSourceFormEvent::Input(DialogInput::Char(ch))),
        );
        events.push(PriceSourceFormEvent::Input(DialogInput::Submit));
        let calls = send(&mut form, events);

        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::SourceSaved {
                name: "CoinGecko".into(),
                key_stored: Some(COINGECKO),
            }))]
        );
        assert_eq!(ports.borrow().keys.exposed()[&COINGECKO], "CG-1");
        let reopened = PriceSourceFormPresenter::edit(shared, COINGECKO, DialogKeys::default())
            .unwrap()
            .view_model();
        assert_eq!(
            reopened.api_key.unwrap().field.placeholder,
            "Key saved · leave blank to keep it"
        );
        assert!(
            PriceSourceFormPresenter::edit(sample().1, crate::YAHOO_FINANCE, DialogKeys::default())
                .unwrap()
                .view_model()
                .api_key
                .is_none(),
            "a built-in that takes no key has no key field"
        );
    }

    #[test]
    fn on_a_ledger_whose_keys_another_secret_saved_the_key_field_is_off_and_says_so() {
        let (ports, shared) = sample();
        ports.borrow_mut().keys.locked = true;
        ports
            .borrow_mut()
            .keys
            .keys
            .insert(COINGECKO, crate::fake::InMemoryKey::Unreadable);
        let mut form =
            PriceSourceFormPresenter::edit(shared, COINGECKO, DialogKeys::default()).unwrap();

        send(
            &mut form,
            vec![
                PriceSourceFormEvent::Focus(PriceSourceField::ApiKey),
                PriceSourceFormEvent::ApiKey("mine".into()),
            ],
        );
        let view = form.view_model();
        let key = view.api_key.unwrap();
        assert!(!key.enabled);
        assert!(!key.field.focused, "it takes no focus");
        assert_eq!(key.field.text, "", "nor typing");
        assert_eq!(key.remove, None);
        assert!(
            key.note
                .starts_with("This Ledger's API keys were saved with another device's")
        );
        assert!(!view.fields.contains(&PriceSourceField::ApiKey));

        send(&mut form, vec![PriceSourceFormEvent::Confirm]);
        assert_eq!(
            ports.borrow().keys.key_state(COINGECKO),
            KeyState::Unreadable,
            "the other device's key is kept"
        );
    }

    #[test]
    fn a_key_the_port_refuses_keeps_the_dialog_open_on_the_saved_source_as_an_edit() {
        let (ports, shared) = sample();
        ports.borrow_mut().keys.unsaved = true;
        let mut add = PriceSourceFormPresenter::add(shared, DialogKeys::default());
        let mut events = filled();
        events.extend([
            PriceSourceFormEvent::Auth(AuthChoice::ApiKey),
            typed(PriceSourceField::KeyParam, "apikey"),
            PriceSourceFormEvent::ApiKey("k3y".into()),
            PriceSourceFormEvent::Confirm,
        ]);

        assert_eq!(send(&mut add, events), []);
        assert_eq!(
            add.view_model().refusal.as_deref(),
            Some(
                "The key wasn't saved: no API key secret could be kept in this device's configuration."
            )
        );

        ports.borrow_mut().keys.unsaved = false;
        let calls = send(&mut add, vec![PriceSourceFormEvent::Confirm]);
        assert_eq!(calls.len(), 1, "saved");
        assert_eq!(
            custom_records(&ports.borrow()).len(),
            1,
            "trying again edits the source rather than adding it twice"
        );
    }

    #[test]
    fn manual_entry_has_no_edit_form_and_a_source_removed_under_one_is_refused_as_text() {
        let (ports, shared) = sample();
        assert!(
            PriceSourceFormPresenter::edit(shared.clone(), MANUAL_ENTRY, DialogKeys::default())
                .is_none()
        );

        let mut add = PriceSourceFormPresenter::add(shared.clone(), DialogKeys::default());
        let mut events = filled();
        events.push(PriceSourceFormEvent::Confirm);
        send(&mut add, events);
        let id = custom_records(&ports.borrow())[0].id;
        let mut edit = PriceSourceFormPresenter::edit(shared, id, DialogKeys::default()).unwrap();
        ports.borrow_mut().price_sources.records.clear();

        assert_eq!(send(&mut edit, vec![PriceSourceFormEvent::Confirm]), []);
        assert_eq!(
            edit.view_model().refusal.as_deref(),
            Some("That price source was removed while the dialog was open.")
        );
    }

    #[test]
    fn the_form_takes_next_field_choose_and_cancel() {
        let (_, shared) = sample();
        let view = PriceSourceFormPresenter::add(shared, DialogKeys::default()).view_model();

        assert_eq!(
            hints(&view.frame),
            pairs(&[
                ("next_field", "next field"),
                ("select_previous/select_next", "choose"),
                ("back", "cancel")
            ])
        );
    }

    #[test]
    fn dialog_input_types_into_a_text_field_and_left_right_step_updates_swallowing_typing() {
        let (_, shared) = sample();
        let mut form = PriceSourceFormPresenter::add(shared, DialogKeys::default());
        send(
            &mut form,
            vec![
                PriceSourceFormEvent::Input(DialogInput::Char('F')),
                PriceSourceFormEvent::Input(DialogInput::Char('x')),
                PriceSourceFormEvent::Focus(PriceSourceField::Updates),
            ],
        );
        let before = form.view_model().updates.chosen;
        send(
            &mut form,
            vec![
                PriceSourceFormEvent::Input(DialogInput::Char('z')),
                PriceSourceFormEvent::Input(DialogInput::Left),
            ],
        );
        let view = form.view_model();
        assert_eq!(view.name.text, "Fx", "a control swallows typing");
        assert_ne!(view.updates.chosen, before);
    }

    #[test]
    fn a_typed_api_key_is_held_and_saved_but_never_shown() {
        let (ports, shared) = sample();
        let mut form = PriceSourceFormPresenter::add(shared, DialogKeys::default());
        let mut events = filled();
        events.push(PriceSourceFormEvent::Auth(AuthChoice::ApiKey));
        events.push(typed(PriceSourceField::KeyParam, "apikey"));
        events.push(PriceSourceFormEvent::Focus(PriceSourceField::ApiKey));
        events.extend(
            "k3y"
                .chars()
                .map(|ch| PriceSourceFormEvent::Input(DialogInput::Char(ch))),
        );
        events.push(PriceSourceFormEvent::Input(DialogInput::Submit));
        let calls = send(&mut form, events);

        assert!(matches!(
            calls.as_slice(),
            [RouterCall::CloseDialog(Some(
                UnitsOutcome::SourceSaved { .. }
            ))]
        ));
        assert!(!format!("{:?}", form.view_model()).contains("k3y"));
        let saved = custom_records(&ports.borrow());
        assert!(matches!(
            saved[0].endpoint.auth,
            PriceSourceAuth::ApiKey { .. }
        ));
    }
}
