//! The Add, Edit and Delete price source Dialogs' live form state -- pure, `gpui`-free; the Dialog
//! host holds it inside [`PriceSourceDialog`](super::dialogs::sources::PriceSourceDialog) and
//! `dialogs/sources/` draws it.
//!
//! A Custom source is a whole [`Endpoint`], so its form (16z·custom) has every field. A built-in's
//! endpoint is code (ADR-0043), so its form (16z·edit) holds only the name and Updates a Ledger
//! stores for it. The API key field is a stub: it is validated here and never leaves the form,
//! since where a Client keeps a key is still to be decided.

use lib_units::{CustomSource, Endpoint, KeyPlacement, PriceSourceAuth, Updates};

use crate::{
    chrome::dialog_host::{DialogKey, DialogOutcome},
    form::field::TextField,
};

/// The Updates control's options, in order.
pub const UPDATES_OPTIONS: [Updates; 4] = [
    Updates::Every15Minutes,
    Updates::Hourly,
    Updates::Daily,
    Updates::ManualOnly,
];

/// The Authentication control's options, in order.
pub const AUTH_OPTIONS: [AuthChoice; 2] = [AuthChoice::None, AuthChoice::ApiKey];

/// The Authentication control's value: whether the source needs a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthChoice {
    #[default]
    None,
    ApiKey,
}

/// A field of the price source forms, in `Tab` order. The built-in form has Name and Updates only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceField {
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

impl SourceField {
    const CUSTOM_ORDER: [SourceField; 8] = [
        SourceField::Name,
        SourceField::Updates,
        SourceField::Address,
        SourceField::Request,
        SourceField::PricePath,
        SourceField::Auth,
        SourceField::KeyParam,
        SourceField::ApiKey,
    ];

    const BUILT_IN_ORDER: [SourceField; 2] = [SourceField::Name, SourceField::Updates];

    /// Whether the field is a segmented control, which takes `Left`/`Right` rather than typing.
    fn is_control(self) -> bool {
        matches!(self, Self::Updates | Self::Auth)
    }
}

/// The Add and Edit price source form for a Custom source (16z·custom).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomSourceForm {
    pub name: TextField,
    pub updates: Updates,
    pub address: TextField,
    pub request: TextField,
    pub price_path: TextField,
    pub auth: AuthChoice,
    pub key_param: TextField,
    /// Typed so the form can ask for it, never stored: nothing reads it past [`Self::is_valid`].
    pub api_key: TextField,
    pub focused: SourceField,
    /// Where the key travels. The form names only a parameter, so a new source puts it there and
    /// an edited one keeps what it had.
    placement: KeyPlacement,
    /// Whether this edits a source, whose key may be left blank since none is stored to re-enter.
    editing: bool,
}

impl Default for CustomSourceForm {
    fn default() -> Self {
        Self {
            name: TextField::default(),
            updates: Updates::Daily,
            address: TextField::default(),
            request: TextField::default(),
            price_path: TextField::default(),
            auth: AuthChoice::None,
            key_param: TextField::default(),
            api_key: TextField::default(),
            focused: SourceField::Name,
            placement: KeyPlacement::Param,
            editing: false,
        }
    }
}

impl CustomSourceForm {
    /// A form editing `source`, every field pre-filled but the key, which was never stored.
    pub fn from_source(source: &CustomSource) -> Self {
        let endpoint = &source.endpoint;
        let (auth, key_param, placement) = match &endpoint.auth {
            PriceSourceAuth::None => (AuthChoice::None, TextField::default(), KeyPlacement::Param),
            PriceSourceAuth::ApiKey { placement, name } => (
                AuthChoice::ApiKey,
                TextField::new(name.as_ref()),
                *placement,
            ),
        };
        Self {
            name: TextField::new(source.name.as_str()),
            updates: source.updates,
            address: TextField::new(endpoint.address.as_ref()),
            request: TextField::new(endpoint.request.as_ref()),
            price_path: TextField::new(endpoint.price_path.as_ref()),
            auth,
            key_param,
            placement,
            editing: true,
            ..Self::default()
        }
    }

    /// Name, Address, Request and Price path are required; with API key authentication so are
    /// the Key parameter and, on a new source, the key itself.
    pub fn is_valid(&self) -> bool {
        let required = [&self.name, &self.address, &self.request, &self.price_path];
        if required.iter().any(|field| field.is_blank()) {
            return false;
        }
        match self.auth {
            AuthChoice::None => true,
            AuthChoice::ApiKey => {
                !self.key_param.is_blank() && (self.editing || !self.api_key.is_blank())
            }
        }
    }

    /// The trimmed name the source is saved under.
    pub fn name(&self) -> String {
        self.name.text().trim().to_string()
    }

    /// The endpoint the fields describe. The API key is not part of it.
    pub fn endpoint(&self) -> Endpoint {
        let trimmed = |field: &TextField| field.text().trim().to_string().into();
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

    /// The text field typing edits, `None` while a control is focused.
    pub fn focused_mut(&mut self) -> Option<&mut TextField> {
        Some(match self.focused {
            SourceField::Name => &mut self.name,
            SourceField::Address => &mut self.address,
            SourceField::Request => &mut self.request,
            SourceField::PricePath => &mut self.price_path,
            SourceField::KeyParam => &mut self.key_param,
            SourceField::ApiKey => &mut self.api_key,
            SourceField::Updates | SourceField::Auth => return None,
        })
    }

    /// Whether `field` is drawn: the key fields only while API key authentication is chosen.
    pub fn shows(&self, field: SourceField) -> bool {
        !matches!(field, SourceField::KeyParam | SourceField::ApiKey)
            || self.auth == AuthChoice::ApiKey
    }

    pub fn set_auth(&mut self, auth: AuthChoice) {
        self.auth = auth;
        if !self.shows(self.focused) {
            self.focused = SourceField::Auth;
        }
    }

    /// `Tab` / `Shift-Tab`, skipping the key fields while they are hidden.
    pub fn cycle_focus(&mut self, backward: bool) {
        let order = SourceField::CUSTOM_ORDER;
        let mut next = self.focused;
        loop {
            next = cycle(&order, next, backward);
            if self.shows(next) {
                break;
            }
        }
        self.focused = next;
    }

    /// `Left`/`Right` (`delta` -1/1) on the focused control; stops at either end.
    fn step_focused(&mut self, delta: isize) {
        match self.focused {
            SourceField::Updates => self.updates = step(&UPDATES_OPTIONS, self.updates, delta),
            SourceField::Auth => self.set_auth(step(&AUTH_OPTIONS, self.auth, delta)),
            _ => {}
        }
    }

    /// Keys this form takes before the shared typing: `Left`/`Right` step the focused control and
    /// `Shift-Tab` goes back a field. A control swallows typing.
    pub fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let control = self.focused.is_control();
        match key {
            DialogKey::Left | DialogKey::Right if control => {
                self.step_focused(if key == DialogKey::Left { -1 } else { 1 });
            }
            DialogKey::BackTab => self.cycle_focus(true),
            DialogKey::Char(_) | DialogKey::Backspace if control => {}
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }
}

/// The Edit price source form for a built-in (16z·edit): its name and Updates, all a Ledger stores
/// of one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltInSourceForm {
    pub name: TextField,
    pub updates: Updates,
    pub focused: SourceField,
}

impl BuiltInSourceForm {
    pub fn new(name: &str, updates: Updates) -> Self {
        Self {
            name: TextField::new(name),
            updates,
            focused: SourceField::Name,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.name.is_blank()
    }

    pub fn name(&self) -> String {
        self.name.text().trim().to_string()
    }

    pub fn focused_mut(&mut self) -> Option<&mut TextField> {
        (self.focused == SourceField::Name).then_some(&mut self.name)
    }

    pub fn cycle_focus(&mut self, backward: bool) {
        self.focused = cycle(&SourceField::BUILT_IN_ORDER, self.focused, backward);
    }

    pub fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let focused = self.focused;
        match key {
            DialogKey::Left | DialogKey::Right if focused == SourceField::Updates => {
                let delta = if key == DialogKey::Left { -1 } else { 1 };
                self.updates = step(&UPDATES_OPTIONS, self.updates, delta);
            }
            DialogKey::BackTab => self.cycle_focus(true),
            DialogKey::Char(_) | DialogKey::Backspace if focused.is_control() => {}
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }
}

/// The Delete price source form (16z·del): the name to type back, copied in when it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteSourceForm {
    pub name: String,
    pub confirm_input: TextField,
}

impl DeleteSourceForm {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            confirm_input: TextField::default(),
        }
    }

    /// Enabled only on an exact, case-sensitive match, as Delete unit is.
    pub fn is_valid(&self) -> bool {
        self.confirm_input.text() == self.name
    }
}

fn cycle(order: &[SourceField], current: SourceField, backward: bool) -> SourceField {
    let count = order.len();
    let index = order
        .iter()
        .position(|field| *field == current)
        .unwrap_or(0);
    let next = if backward {
        (index + count - 1) % count
    } else {
        (index + 1) % count
    };
    order[next]
}

fn step<T: Copy + PartialEq>(options: &[T], current: T, delta: isize) -> T {
    let index = options
        .iter()
        .position(|option| *option == current)
        .unwrap_or(0);
    let next = index
        .saturating_add_signed(delta)
        .min(options.len().saturating_sub(1));
    options.get(next).copied().unwrap_or(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_core::PriceSourceId;

    fn type_into(form: &mut CustomSourceForm, text: &str) {
        for ch in text.chars() {
            form.focused_mut()
                .expect("a text field is focused")
                .push(ch);
        }
    }

    fn filled() -> CustomSourceForm {
        let mut form = CustomSourceForm::default();
        type_into(&mut form, " Alpha Vantage ");
        form.focused = SourceField::Address;
        type_into(&mut form, "https://www.alphavantage.co");
        form.focused = SourceField::Request;
        type_into(&mut form, "/query?symbol={symbol}");
        form.focused = SourceField::PricePath;
        type_into(&mut form, "/Global Quote/05. price");
        form
    }

    #[test]
    fn a_custom_source_needs_its_name_address_request_and_price_path() {
        let mut form = CustomSourceForm::default();
        assert!(!form.is_valid());
        form = filled();
        assert!(form.is_valid());
        form.price_path = TextField::new("  ");
        assert!(!form.is_valid());
    }

    #[test]
    fn api_key_authentication_needs_a_key_parameter_and_a_key_on_a_new_source() {
        let mut form = filled();
        form.set_auth(AuthChoice::ApiKey);
        assert!(!form.is_valid());
        form.key_param = TextField::new("apikey");
        assert!(!form.is_valid(), "a new source needs its key typed");
        form.api_key = TextField::new("secret");
        assert!(form.is_valid());
    }

    #[test]
    fn the_endpoint_is_trimmed_and_carries_the_key_parameter_but_never_the_key() {
        let mut form = filled();
        form.set_auth(AuthChoice::ApiKey);
        form.key_param = TextField::new(" apikey ");
        form.api_key = TextField::new("secret");
        assert_eq!(form.name(), "Alpha Vantage");
        let endpoint = form.endpoint();
        assert_eq!(endpoint.address, "https://www.alphavantage.co");
        assert_eq!(
            endpoint.auth,
            PriceSourceAuth::ApiKey {
                placement: KeyPlacement::Param,
                name: "apikey".to_string().into(),
            }
        );
        assert!(!format!("{endpoint:?}").contains("secret"));
    }

    #[test]
    fn editing_prefills_every_field_but_the_key_and_keeps_the_placement() {
        let source = CustomSource {
            id: PriceSourceId::new(),
            name: "Broker".into(),
            endpoint: Endpoint {
                address: "https://broker.example".to_string().into(),
                request: "/{symbol}".to_string().into(),
                price_path: "/price".to_string().into(),
                auth: PriceSourceAuth::ApiKey {
                    placement: KeyPlacement::Header,
                    name: "x-api-key".to_string().into(),
                },
            },
            updates: Updates::Hourly,
        };
        let form = CustomSourceForm::from_source(&source);
        assert_eq!(form.name.text(), "Broker");
        assert_eq!(form.updates, Updates::Hourly);
        assert_eq!(form.auth, AuthChoice::ApiKey);
        assert_eq!(form.api_key.text(), "");
        assert!(form.is_valid(), "an edit may leave the key blank");
        assert_eq!(form.endpoint(), source.endpoint);
    }

    #[test]
    fn tab_skips_the_key_fields_until_api_key_is_chosen() {
        let mut form = CustomSourceForm {
            focused: SourceField::Auth,
            ..CustomSourceForm::default()
        };
        form.cycle_focus(false);
        assert_eq!(form.focused, SourceField::Name);
        form.cycle_focus(true);
        assert_eq!(form.focused, SourceField::Auth);

        form.handle_own_key(DialogKey::Right);
        assert_eq!(form.auth, AuthChoice::ApiKey);
        form.cycle_focus(false);
        assert_eq!(form.focused, SourceField::KeyParam);
        form.cycle_focus(false);
        assert_eq!(form.focused, SourceField::ApiKey);

        form.set_auth(AuthChoice::None);
        assert_eq!(
            form.focused,
            SourceField::Auth,
            "a hidden field gives up focus"
        );
    }

    #[test]
    fn left_and_right_step_updates_and_stop_at_either_end_while_typing_is_swallowed() {
        let mut form = CustomSourceForm {
            focused: SourceField::Updates,
            ..CustomSourceForm::default()
        };
        assert_eq!(form.updates, Updates::Daily);
        form.handle_own_key(DialogKey::Right);
        form.handle_own_key(DialogKey::Right);
        assert_eq!(form.updates, Updates::ManualOnly);
        for _ in 0..5 {
            form.handle_own_key(DialogKey::Left);
        }
        assert_eq!(form.updates, Updates::Every15Minutes);
        assert_eq!(
            form.handle_own_key(DialogKey::Char('x')),
            Some(DialogOutcome::Handled)
        );
        assert!(form.focused_mut().is_none());
    }

    #[test]
    fn a_built_in_form_edits_only_its_name_and_updates() {
        let mut form = BuiltInSourceForm::new("CoinGecko", Updates::Every15Minutes);
        assert!(form.is_valid());
        form.cycle_focus(false);
        assert_eq!(form.focused, SourceField::Updates);
        form.handle_own_key(DialogKey::Right);
        assert_eq!(form.updates, Updates::Hourly);
        form.cycle_focus(false);
        assert_eq!(form.focused, SourceField::Name);
        form.name = TextField::new(" ");
        assert!(!form.is_valid());
    }

    #[test]
    fn delete_confirms_only_on_the_exact_name() {
        let mut form = DeleteSourceForm::new("Broker");
        assert!(!form.is_valid());
        form.confirm_input = TextField::new("broker");
        assert!(!form.is_valid());
        form.confirm_input = TextField::new("Broker");
        assert!(form.is_valid());
    }
}
