//! The Add and Edit price source Dialogs. A Custom source is a whole [`Endpoint`], so its form
//! (16z·custom) has every field. A built-in's endpoint is code (ADR-0043), so its form (16z·edit)
//! holds only the name and Updates a Ledger stores for it, and shows the endpoint read-only.
//!
//! The API key field is asked for but never leaves the form: where a Client keeps a key is the OS
//! keychain's, off this module (ADR-0043).

use lib_core::PriceSourceId;
use secrecy::{ExposeSecret as _, SecretString};

use super::{
    Choice, DialogFrame, SelectView, TextFieldView, UnitsOutcome, count, cycle, step,
    updates_choices,
};
use crate::interactor::{AddPriceSource, EditPriceSource, EditPriceSourceRefusal};
use crate::protocols::{SharedPorts, UnitsRouter};
use crate::{Endpoint, KeyPlacement, PriceSourceAuth, SourceOrigin, Updates, msg, price_source};

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
    Auth,
    KeyParam,
    ApiKey,
}

impl PriceSourceField {
    const CUSTOM_ORDER: [Self; 8] = [
        Self::Name,
        Self::Updates,
        Self::Address,
        Self::Request,
        Self::PricePath,
        Self::Auth,
        Self::KeyParam,
        Self::ApiKey,
    ];
    const BUILT_IN_ORDER: [Self; 2] = [Self::Name, Self::Updates];
}

/// Not `PartialEq`: [`Self::ApiKey`] holds a secret, which can't be compared or printed.
#[derive(Debug, Clone)]
pub enum PriceSourceFormEvent {
    /// A text field's whole new text, as its View's input holds it. Ignored for a control and for
    /// the API key, which comes as [`Self::ApiKey`].
    Text(PriceSourceField, String),
    /// The API key field's whole new text.
    ApiKey(SecretString),
    Updates(Updates),
    Auth(AuthChoice),
    /// `Left`/`Right` on the focused control (`-1`/`1`), stopping at either end.
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
    /// The Test button's label, and the note that testing isn't built yet.
    pub test: String,
    pub test_note: String,
    /// The fields on screen, in `Tab` order.
    pub fields: Vec<PriceSourceField>,
    pub refusal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointFields {
    pub address: TextFieldView,
    pub request: TextFieldView,
    pub request_help: String,
    pub price_path: TextFieldView,
    pub price_path_help: String,
    pub auth: SelectView<AuthChoice>,
    /// The key fields, shown only with API key authentication.
    pub key_param: Option<TextFieldView>,
    /// Its text is the typed key masked, one `•` per character.
    pub api_key: Option<TextFieldView>,
    pub key_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    AddCustom,
    EditCustom(PriceSourceId),
    EditBuiltIn(PriceSourceId),
}

pub struct PriceSourceFormPresenter {
    ports: SharedPorts,
    mode: Mode,
    name: String,
    updates: Updates,
    address: String,
    request: String,
    price_path: String,
    auth: AuthChoice,
    key_param: String,
    /// Typed so the form can ask for it, never stored: nothing reads it past validity and the
    /// mask's length.
    api_key: SecretString,
    /// Where the key travels. The form names only a parameter, so a new source puts it there and
    /// an edited one keeps what it had.
    placement: KeyPlacement,
    focused: PriceSourceField,
    refusal: Option<String>,
}

impl PriceSourceFormPresenter {
    /// An empty Add price source form, for a Custom source: the built-ins are always listed, so
    /// there is none to add.
    pub fn add(ports: SharedPorts) -> Self {
        Self {
            ports,
            mode: Mode::AddCustom,
            name: String::new(),
            updates: Updates::Daily,
            address: String::new(),
            request: String::new(),
            price_path: String::new(),
            auth: AuthChoice::None,
            key_param: String::new(),
            api_key: SecretString::default(),
            placement: KeyPlacement::Param,
            focused: PriceSourceField::Name,
            refusal: None,
        }
    }

    /// The Edit form for source `id`: a built-in's name and Updates, or a Custom source's every
    /// field but the key, which was never stored. `None` for Manual entry, which has nothing to
    /// edit, or a source that is gone.
    pub fn edit(ports: SharedPorts, id: PriceSourceId) -> Option<Self> {
        let mut form = Self::add(ports.clone());
        let ports = ports.borrow();
        let records = ports.price_sources().price_source_records();
        let manual = lib_locale::msg::price_source_manual_entry();
        let source = price_source(&records, &manual, id)?;
        form.name = source.name.to_string();
        form.updates = source.updates;
        match source.origin {
            SourceOrigin::ManualEntry => return None,
            SourceOrigin::BuiltIn(_) => form.mode = Mode::EditBuiltIn(id),
            SourceOrigin::Custom(custom) => {
                let endpoint = &custom.endpoint;
                form.mode = Mode::EditCustom(id);
                form.address = endpoint.address.to_string();
                form.request = endpoint.request.to_string();
                form.price_path = endpoint.price_path.to_string();
                if let PriceSourceAuth::ApiKey { placement, name } = &endpoint.auth {
                    form.auth = AuthChoice::ApiKey;
                    form.key_param = name.to_string();
                    form.placement = *placement;
                }
            }
        }
        drop(ports);
        Some(form)
    }

    pub fn handle(&mut self, event: PriceSourceFormEvent, router: &mut dyn UnitsRouter) {
        match event {
            PriceSourceFormEvent::Text(field, text) => {
                if let Some(slot) = self.text_mut(field) {
                    *slot = text;
                }
            }
            PriceSourceFormEvent::ApiKey(key) => {
                if self.shows(PriceSourceField::ApiKey) {
                    self.api_key = key;
                }
            }
            PriceSourceFormEvent::Updates(updates) => self.updates = updates,
            PriceSourceFormEvent::Auth(auth) if self.is_custom() => self.set_auth(auth),
            PriceSourceFormEvent::Auth(_) => {}
            PriceSourceFormEvent::Step(delta) => match self.focused {
                PriceSourceField::Updates => {
                    let options: Vec<_> = updates_choices().iter().map(|c| c.value).collect();
                    self.updates = step(&options, self.updates, delta);
                }
                PriceSourceField::Auth => {
                    let auth = step(&[AuthChoice::None, AuthChoice::ApiKey], self.auth, delta);
                    self.set_auth(auth);
                }
                _ => {}
            },
            PriceSourceFormEvent::NextField | PriceSourceFormEvent::PrevField => {
                let backward = matches!(event, PriceSourceFormEvent::PrevField);
                self.focused = cycle(self.order(), self.focused, backward, |f| self.shows(f));
            }
            PriceSourceFormEvent::Focus(field) => {
                if self.shows(field) {
                    self.focused = field;
                }
            }
            PriceSourceFormEvent::Confirm => self.confirm(router),
            PriceSourceFormEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> PriceSourceFormViewModel {
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
            api_key: keyed.then(|| {
                text(
                    msg::units_price_source_field_api_key(),
                    &"\u{2022}".repeat(self.api_key.expose_secret().chars().count()),
                    String::new(),
                    PriceSourceField::ApiKey,
                )
            }),
            key_note: keyed.then(msg::units_price_source_key_note),
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
            },
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
                .filter(|field| self.shows(*field))
                .collect(),
            refusal: self.refusal.clone(),
        }
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

    /// Whether `field` is drawn: the key fields only while API key authentication is chosen.
    fn shows(&self, field: PriceSourceField) -> bool {
        self.order().contains(&field)
            && (!matches!(field, PriceSourceField::KeyParam | PriceSourceField::ApiKey)
                || self.auth == AuthChoice::ApiKey)
    }

    fn set_auth(&mut self, auth: AuthChoice) {
        self.auth = auth;
        if !self.shows(self.focused) {
            self.focused = PriceSourceField::Auth;
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
            PriceSourceField::KeyParam => &mut self.key_param,
            PriceSourceField::ApiKey | PriceSourceField::Updates | PriceSourceField::Auth => {
                return None;
            }
        })
    }

    /// Name is required; a Custom source also needs its Address, Request and Price path, and with
    /// API key authentication the Key parameter and, on a new source, the key itself.
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
        match self.auth {
            AuthChoice::None => true,
            AuthChoice::ApiKey => {
                filled(&self.key_param)
                    && (matches!(self.mode, Mode::EditCustom(_))
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
                },
            },
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

    fn confirm(&mut self, router: &mut dyn UnitsRouter) {
        if !self.is_valid() {
            return;
        }
        let name = self.name.trim().to_string();
        let saved = {
            let mut ports = self.ports.borrow_mut();
            let price_sources = ports.split_mut().price_sources;
            match self.mode {
                Mode::AddCustom => {
                    AddPriceSource {
                        name: name.clone(),
                        endpoint: self.endpoint(),
                        updates: self.updates,
                    }
                    .run(price_sources);
                    Ok(())
                }
                Mode::EditCustom(id) | Mode::EditBuiltIn(id) => EditPriceSource {
                    id,
                    name: name.clone(),
                    updates: self.updates,
                    endpoint: self.is_custom().then(|| self.endpoint()),
                }
                .run(price_sources)
                .map(|_| ()),
            }
        };
        self.refusal = match saved {
            Ok(()) => {
                router.close_dialog(Some(UnitsOutcome::SourceSaved { name }));
                None
            }
            Err(EditPriceSourceRefusal::Gone) => Some(msg::units_refusal_price_source_gone()),
            Err(EditPriceSourceRefusal::ManualEntry) => Some(msg::units_refusal_manual_entry()),
            Err(EditPriceSourceRefusal::BuiltInEndpoint) => {
                Some(msg::units_refusal_built_in_endpoint())
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
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

    fn custom_records(ports: &crate::fake::InMemoryPorts) -> Vec<CustomSource> {
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
        let (ports, shared) = stub();
        let mut form = PriceSourceFormPresenter::add(shared);
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
                name: "Alpha Vantage".into()
            }))]
        );
        let saved = custom_records(&ports.borrow());
        assert_eq!(saved[0].name, "Alpha Vantage");
        assert_eq!(saved[0].endpoint.auth, PriceSourceAuth::None);
    }

    #[test]
    fn api_key_authentication_shows_the_key_fields_and_needs_them_on_a_new_source() {
        let (ports, shared) = stub();
        let mut form = PriceSourceFormPresenter::add(shared);
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
        let endpoint = form.view_model();
        assert_eq!(endpoint.endpoint.unwrap().api_key.unwrap().text, "••••••");
        assert_eq!(
            custom_records(&ports.borrow())[0].endpoint.auth,
            PriceSourceAuth::ApiKey {
                placement: KeyPlacement::Param,
                name: "apikey".into()
            },
            "only the key's parameter name is stored"
        );
    }

    #[test]
    fn tab_skips_the_hidden_key_fields_and_choosing_none_moves_focus_off_them() {
        let (_, shared) = stub();
        let mut form = PriceSourceFormPresenter::add(shared);
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
        let (ports, shared) = stub();
        ports.borrow_mut().units.units[1].link.source_id = COINGECKO;
        let mut form = PriceSourceFormPresenter::edit(shared, COINGECKO).unwrap();
        let view = form.view_model();
        assert_eq!(view.frame.title, "Edit price source — CoinGecko");
        assert_eq!(view.endpoint, None);
        assert_eq!(
            view.fields,
            [PriceSourceField::Name, PriceSourceField::Updates]
        );
        assert_eq!(view.built_in_facts[0].1, "CoinGecko · built-in");
        assert_eq!(view.usage.as_deref(), Some("Values 1 unit: btc."));

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
    fn editing_a_custom_source_prefills_all_but_the_key_and_needs_no_key_retyped() {
        let (ports, shared) = stub();
        let mut add = PriceSourceFormPresenter::add(shared.clone());
        let mut events = filled();
        events.extend([
            PriceSourceFormEvent::Auth(AuthChoice::ApiKey),
            typed(PriceSourceField::KeyParam, "apikey"),
            PriceSourceFormEvent::ApiKey("secret".into()),
            PriceSourceFormEvent::Confirm,
        ]);
        send(&mut add, events);
        let id = custom_records(&ports.borrow())[0].id;

        let mut edit = PriceSourceFormPresenter::edit(shared, id).unwrap();
        let view = edit.view_model();
        let endpoint = view.endpoint.unwrap();
        assert_eq!(endpoint.address.text, "https://www.alphavantage.co");
        assert_eq!(endpoint.key_param.unwrap().text, "apikey");
        assert_eq!(
            endpoint.api_key.unwrap().text,
            "",
            "the key was never stored"
        );
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
                name: "AV".into()
            }))]
        );
    }

    #[test]
    fn manual_entry_has_no_edit_form_and_a_source_removed_under_one_is_refused_as_text() {
        let (ports, shared) = stub();
        assert!(PriceSourceFormPresenter::edit(shared.clone(), MANUAL_ENTRY).is_none());

        let mut add = PriceSourceFormPresenter::add(shared.clone());
        let mut events = filled();
        events.push(PriceSourceFormEvent::Confirm);
        send(&mut add, events);
        let id = custom_records(&ports.borrow())[0].id;
        let mut edit = PriceSourceFormPresenter::edit(shared, id).unwrap();
        ports.borrow_mut().price_sources.records.clear();

        assert_eq!(send(&mut edit, vec![PriceSourceFormEvent::Confirm]), []);
        assert_eq!(
            edit.view_model().refusal.as_deref(),
            Some("That price source was removed while the dialog was open.")
        );
    }
}
