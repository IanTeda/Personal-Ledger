//! The Units Dialogs' View: whichever `lib-units` Dialog Presenter is open, the keys that drive
//! it and the floating window that draws its view model. Every rule and every piece of text is the
//! Presenter's; what is the TUI's is the widget mechanics (a text field appends and backspaces, a
//! select steps with `←`/`→`) and the typed API key, which the view model only shows as dots.
//!
//! Keys: a key is looked up first in `lib-shell`'s Dialog keymap, so `[keybindings]` moves it (by
//! default `Tab`/`Shift-Tab` move between fields, `Enter` confirms, `Esc` cancels and `↑`/`↓`
//! step a select). Then printable keys and `Backspace` edit the focused text field, and `←`/`→`
//! step the focused select (or a Dialog's only select, where its Presenter keeps no focus). The
//! footer names the keys the Presenter says its Dialog takes, each chord read from that keymap.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_shell::DialogCommand;
use lib_units::presenter::{
    DeletePriceSourceEvent, DeletePriceSourcePresenter, DeleteUnitEvent, DeleteUnitPresenter,
    PriceSourceField, PriceSourceFormEvent, PriceSourceFormPresenter, SetBaseUnitEvent,
    SetBaseUnitPresenter, SetPriceEvent, SetPricePresenter, UnitFormEvent, UnitFormPresenter,
};
use lib_units::{SharedPorts, UnitsDialog as Which, UnitsRouter};
use lib_viper::keymap::{Keymap, SELECT};
use lib_viper::view::{DialogFrame, SelectView, TextFieldView};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use secrecy::{ExposeSecret as _, SecretString};

use crate::colours::Colours;
use crate::popup::REFERENCE_TERMINAL_WIDTH;
use crate::units::view::units::chord;

/// The window's width: the ~88% of the reference terminal the unit forms have always taken
/// (`docs/ux/tui-mockups/07-units/README.md` "The forms").
const POPUP_WIDTH: u16 = (REFERENCE_TERMINAL_WIDTH as u32 * 88 / 100) as u16;

/// The field label column, wide enough for the longest English label with a gap.
const LABEL_WIDTH: u16 = 18;

/// What separates one footer key from the next.
const FOOTER_SEPARATOR: &str = " \u{b7} ";

/// The open Units Dialog: its Presenter, and for the Price Source form the API key typed so far.
pub enum Dialog {
    UnitForm(UnitFormPresenter),
    DeleteUnit(DeleteUnitPresenter),
    SetBaseUnit(SetBaseUnitPresenter),
    SetPrice(SetPricePresenter),
    PriceSourceForm(PriceSourceFormPresenter, SecretString),
    DeletePriceSource(DeletePriceSourcePresenter),
}

/// One row of a Dialog as drawn, flattened from its view model.
enum Item {
    Text(TextFieldView),
    Select {
        label: String,
        value: String,
        focused: bool,
    },
    Heading(String),
    Note(String),
    Fact(String, String),
    Refusal(String),
}

impl Dialog {
    /// `which`'s Presenter on `ports`, or `None` when the row it names is gone.
    pub fn open(which: Which, ports: &SharedPorts) -> Option<Self> {
        let ports = ports.clone();
        Some(match which {
            Which::AddUnit => Self::UnitForm(UnitFormPresenter::add(ports)),
            Which::EditUnit(id) => Self::UnitForm(UnitFormPresenter::edit(ports, id)?),
            Which::DeleteUnit(id) => Self::DeleteUnit(DeleteUnitPresenter::new(ports, id)?),
            Which::SetBaseUnit => Self::SetBaseUnit(SetBaseUnitPresenter::new(ports)),
            Which::SetPrice(id) => Self::SetPrice(SetPricePresenter::new(
                ports,
                std::rc::Rc::new(chrono::Utc::now),
                id,
            )?),
            Which::AddPriceSource => Self::PriceSourceForm(
                PriceSourceFormPresenter::add(ports),
                SecretString::from(String::new()),
            ),
            Which::EditPriceSource(id) => Self::PriceSourceForm(
                PriceSourceFormPresenter::edit(ports, id)?,
                SecretString::from(String::new()),
            ),
            Which::DeletePriceSource(id) => {
                Self::DeletePriceSource(DeletePriceSourcePresenter::new(ports, id)?)
            }
        })
    }

    /// Turns `key` into the open Presenter's event, if it means one, and sends it. `keymap` is
    /// the Dialog keymap, which a key is looked up in before it types or steps.
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        keymap: &Keymap<DialogCommand>,
        router: &mut dyn UnitsRouter,
    ) {
        let form = chord(key)
            .and_then(|chord| keymap.event(&chord))
            .map(FormKey::from);
        match self {
            Self::UnitForm(presenter) => {
                if let Some(event) = unit_form_event(&presenter.view_model(), form, key) {
                    presenter.handle(event, router);
                }
            }
            Self::DeleteUnit(presenter) => {
                let view = presenter.view_model();
                let event = match form {
                    Some(FormKey::Cancel) => Some(DeleteUnitEvent::Cancel),
                    Some(FormKey::Confirm) => Some(DeleteUnitEvent::Confirm),
                    Some(FormKey::Step(delta)) => view
                        .new_base
                        .as_ref()
                        .and_then(|new_base| stepped(&new_base.select, delta))
                        .map(DeleteUnitEvent::NewBase),
                    Some(FormKey::Next | FormKey::Prev) => None,
                    None => view
                        .confirm
                        .as_ref()
                        .and_then(|field| edited(&field.text, key))
                        .map(DeleteUnitEvent::ConfirmText)
                        .or_else(|| {
                            let select = &view.new_base.as_ref()?.select;
                            stepped(select, arrow(key)?).map(DeleteUnitEvent::NewBase)
                        }),
                };
                if let Some(event) = event {
                    presenter.handle(event, router);
                }
            }
            Self::SetBaseUnit(presenter) => {
                let event = match form {
                    Some(FormKey::Cancel) => Some(SetBaseUnitEvent::Cancel),
                    Some(FormKey::Confirm) => Some(SetBaseUnitEvent::Confirm),
                    Some(FormKey::Next | FormKey::Prev) => None,
                    step => form_step(step, key)
                        .and_then(|delta| stepped(&presenter.view_model().base, delta))
                        .map(SetBaseUnitEvent::Choose),
                };
                if let Some(event) = event {
                    presenter.handle(event, router);
                }
            }
            Self::SetPrice(presenter) => {
                let view = presenter.view_model();
                let event = match form {
                    Some(FormKey::Cancel) => Some(SetPriceEvent::Cancel),
                    Some(FormKey::Confirm) => Some(SetPriceEvent::Confirm),
                    Some(FormKey::Next) => Some(SetPriceEvent::NextField),
                    Some(FormKey::Prev) => Some(SetPriceEvent::PrevField),
                    Some(FormKey::Step(_)) => None,
                    None if view.week.focused => {
                        edited(&view.week.text, key).map(SetPriceEvent::Week)
                    }
                    None => edited(&view.close.text, key).map(SetPriceEvent::Close),
                };
                if let Some(event) = event {
                    presenter.handle(event, router);
                }
            }
            Self::PriceSourceForm(presenter, api_key) => {
                if let Some(event) = price_source_event(presenter, api_key, form, key) {
                    presenter.handle(event, router);
                }
            }
            Self::DeletePriceSource(presenter) => {
                let event = match form {
                    Some(FormKey::Cancel) => Some(DeletePriceSourceEvent::Cancel),
                    Some(FormKey::Confirm) => Some(DeletePriceSourceEvent::Confirm),
                    Some(_) => None,
                    None => edited(&presenter.view_model().confirm.text, key)
                        .map(DeletePriceSourceEvent::ConfirmText),
                };
                if let Some(event) = event {
                    presenter.handle(event, router);
                }
            }
        }
    }

    /// The Dialog as a floating window over `area`, the whole frame, its footer's chords read from
    /// `keymap`, the Dialog keymap.
    pub fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        keymap: &Keymap<DialogCommand>,
        c: &Colours,
    ) {
        let (dialog, items) = self.items();
        let lines: Vec<Line<'_>> = items.iter().map(|item| item_line(item, c)).collect();
        // A long note wraps, so the body is as tall as its lines wrap to inside the border.
        let wrap_width = usize::from(POPUP_WIDTH.min(area.width).saturating_sub(2)).max(1);
        let body_rows: usize = lines
            .iter()
            .map(|line| line.width().div_ceil(wrap_width).max(1))
            .sum();
        // Title and its rule, the body, then the footer's rule and the footer.
        let content = u16::try_from(body_rows)
            .unwrap_or(u16::MAX)
            .saturating_add(4);
        let window = popup_rect(area, content);
        frame.render_widget(Clear, window);
        let block = Block::bordered();
        let inner = block.inner(window);
        frame.render_widget(block, window);

        let [title, rule, body, footer_rule, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);
        let bold = Style::default().add_modifier(Modifier::BOLD);
        frame.render_widget(
            Paragraph::new(Span::styled(dialog.title.clone(), bold)),
            title,
        );
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rule);
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), footer_rule);
        frame.render_widget(Paragraph::new(footer_line(&dialog, keymap, c)), footer);
    }

    fn items(&self) -> (DialogFrame, Vec<Item>) {
        let mut items = Vec::new();
        let (dialog, refusal) = match self {
            Self::UnitForm(presenter) => {
                let view = presenter.view_model();
                items.push(Item::Text(view.code));
                items.push(select(&view.kind));
                items.push(Item::Text(view.name));
                items.push(Item::Text(view.precision));
                items.push(Item::Note(view.precision_help));
                items.push(Item::Heading(view.source_group));
                items.push(select(&view.source));
                items.push(Item::Note(view.source_help));
                items.extend(view.symbol.map(Item::Text));
                items.extend(view.symbol_help.map(Item::Note));
                items.extend(view.quote.as_ref().map(select));
                items.extend(view.usage.map(Item::Note));
                (view.frame, view.refusal)
            }
            Self::DeleteUnit(presenter) => {
                let view = presenter.view_model();
                if let Some(in_use) = view.in_use {
                    items.push(Item::Note(in_use.text));
                    items.push(Item::Note(in_use.help));
                    items.extend(in_use.base.map(Item::Note));
                }
                items.extend(view.confirm.map(Item::Text));
                if let Some(new_base) = view.new_base {
                    items.push(select(&new_base.select));
                    items.push(Item::Note(new_base.help));
                }
                (view.frame, view.refusal)
            }
            Self::SetBaseUnit(presenter) => {
                let view = presenter.view_model();
                items.push(select(&view.base));
                (view.frame, view.refusal)
            }
            Self::SetPrice(presenter) => {
                let view = presenter.view_model();
                items.push(Item::Text(view.week));
                items.push(Item::Note(view.week_help));
                items.push(Item::Text(view.close));
                items.push(Item::Note(view.close_help));
                (view.frame, view.refusal)
            }
            Self::PriceSourceForm(presenter, _) => {
                let view = presenter.view_model();
                items.push(Item::Text(view.name));
                items.push(Item::Note(view.name_help));
                items.push(select(&view.updates));
                if let Some(endpoint) = view.endpoint {
                    items.push(Item::Text(endpoint.address));
                    items.push(Item::Text(endpoint.request));
                    items.push(Item::Note(endpoint.request_help));
                    items.push(Item::Text(endpoint.price_path));
                    items.push(Item::Note(endpoint.price_path_help));
                    items.push(select(&endpoint.auth));
                    items.extend(endpoint.key_param.map(Item::Text));
                    items.extend(endpoint.api_key.map(Item::Text));
                    items.extend(endpoint.key_note.map(Item::Note));
                }
                items.extend(
                    view.built_in_facts
                        .into_iter()
                        .map(|(label, value)| Item::Fact(label, value)),
                );
                items.extend(view.built_in_help.map(Item::Note));
                items.extend(view.usage.map(Item::Note));
                (view.frame, view.refusal)
            }
            Self::DeletePriceSource(presenter) => {
                let view = presenter.view_model();
                items.push(Item::Note(view.warning));
                items.extend(view.fallback.map(Item::Note));
                items.push(Item::Text(view.confirm));
                (view.frame, view.refusal)
            }
        };
        items.extend(refusal.map(Item::Refusal));
        (dialog, items)
    }
}

fn unit_form_event(
    view: &lib_units::presenter::UnitFormViewModel,
    form: Option<FormKey>,
    key: KeyEvent,
) -> Option<UnitFormEvent> {
    let delta = match form {
        Some(FormKey::Cancel) => return Some(UnitFormEvent::Cancel),
        Some(FormKey::Confirm) => return Some(UnitFormEvent::Confirm),
        Some(FormKey::Next) => return Some(UnitFormEvent::NextField),
        Some(FormKey::Prev) => return Some(UnitFormEvent::PrevField),
        Some(FormKey::Step(delta)) => delta,
        None => {
            if view.code.focused {
                return edited(&view.code.text, key).map(UnitFormEvent::Code);
            }
            if view.name.focused {
                return edited(&view.name.text, key).map(UnitFormEvent::Name);
            }
            if view.precision.focused {
                return edited(&view.precision.text, key).map(UnitFormEvent::Precision);
            }
            if let Some(symbol) = view.symbol.as_ref().filter(|field| field.focused) {
                return edited(&symbol.text, key).map(UnitFormEvent::Symbol);
            }
            arrow(key)?
        }
    };
    if view.kind.focused {
        return stepped(&view.kind, delta).map(UnitFormEvent::Kind);
    }
    if view.source.focused {
        return stepped(&view.source, delta).map(UnitFormEvent::Source);
    }
    let quote = view.quote.as_ref().filter(|select| select.focused)?;
    stepped(quote, delta).map(UnitFormEvent::Quote)
}

fn price_source_event(
    presenter: &PriceSourceFormPresenter,
    api_key: &mut SecretString,
    form: Option<FormKey>,
    key: KeyEvent,
) -> Option<PriceSourceFormEvent> {
    // The selects (Updates, Authentication) step through the Presenter, which knows the focus.
    match form {
        Some(FormKey::Cancel) => return Some(PriceSourceFormEvent::Cancel),
        Some(FormKey::Confirm) => return Some(PriceSourceFormEvent::Confirm),
        Some(FormKey::Next) => return Some(PriceSourceFormEvent::NextField),
        Some(FormKey::Prev) => return Some(PriceSourceFormEvent::PrevField),
        Some(FormKey::Step(delta)) => return Some(PriceSourceFormEvent::Step(delta)),
        None => {}
    }
    match key.code {
        KeyCode::Left => return Some(PriceSourceFormEvent::Step(-1)),
        KeyCode::Right => return Some(PriceSourceFormEvent::Step(1)),
        _ => {}
    }
    let view = presenter.view_model();
    if view.name.focused {
        return edited(&view.name.text, key)
            .map(|text| PriceSourceFormEvent::Text(PriceSourceField::Name, text));
    }
    let endpoint = view.endpoint?;
    if endpoint.api_key.as_ref().is_some_and(|field| field.focused) {
        let typed = edited(api_key.expose_secret(), key)?;
        *api_key = SecretString::from(typed);
        return Some(PriceSourceFormEvent::ApiKey(api_key.clone()));
    }
    let fields = [
        (Some(endpoint.address), PriceSourceField::Address),
        (Some(endpoint.request), PriceSourceField::Request),
        (Some(endpoint.price_path), PriceSourceField::PricePath),
        (endpoint.key_param, PriceSourceField::KeyParam),
    ];
    let (text, field) = fields
        .into_iter()
        .find_map(|(view, field)| view.filter(|view| view.focused).map(|view| (view, field)))?;
    edited(&text.text, key).map(|text| PriceSourceFormEvent::Text(field, text))
}

/// A Dialog keymap command as these Dialogs take it. Their selects step in place rather than
/// open a list, so `select` confirms as `submit` does.
#[derive(Clone, Copy)]
enum FormKey {
    Cancel,
    Confirm,
    Next,
    Prev,
    Step(isize),
}

impl From<DialogCommand> for FormKey {
    fn from(command: DialogCommand) -> Self {
        match command {
            DialogCommand::Back => Self::Cancel,
            DialogCommand::Select | DialogCommand::Submit => Self::Confirm,
            DialogCommand::NextField => Self::Next,
            DialogCommand::PreviousField => Self::Prev,
            DialogCommand::SelectNext => Self::Step(1),
            DialogCommand::SelectPrevious => Self::Step(-1),
        }
    }
}

/// How far a select steps: by the Dialog keymap's command, or for a key it doesn't bind, by an
/// arrow.
fn form_step(form: Option<FormKey>, key: KeyEvent) -> Option<isize> {
    match form {
        Some(FormKey::Step(delta)) => Some(delta),
        Some(_) => None,
        None => arrow(key),
    }
}

/// `←` (`↑`) steps a select back and `→` (`↓`) forward.
fn arrow(key: KeyEvent) -> Option<isize> {
    match key.code {
        KeyCode::Left | KeyCode::Up => Some(-1),
        KeyCode::Right | KeyCode::Down => Some(1),
        _ => None,
    }
}

/// `text` after `key`: a printable character appended, or the last one removed by `Backspace`.
pub(crate) fn edited(text: &str, key: KeyEvent) -> Option<String> {
    let mut text = text.to_string();
    match key.code {
        KeyCode::Char(ch)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            text.push(ch);
        }
        KeyCode::Backspace => {
            text.pop()?;
        }
        _ => return None,
    }
    Some(text)
}

/// The option `delta` steps `select` to, stopping at either end.
fn stepped<T: Copy + PartialEq>(select: &SelectView<T>, delta: isize) -> Option<T> {
    let current = select
        .options
        .iter()
        .position(|choice| Some(choice.value) == select.chosen);
    let next = match current {
        Some(index) => index
            .saturating_add_signed(delta)
            .min(select.options.len().saturating_sub(1)),
        None => 0,
    };
    select.options.get(next).map(|choice| choice.value)
}

fn select<T: PartialEq>(view: &SelectView<T>) -> Item {
    Item::Select {
        label: view.label.clone(),
        value: view
            .options
            .iter()
            .find(|choice| Some(&choice.value) == view.chosen.as_ref())
            .map(|choice| choice.label.clone())
            .unwrap_or_default(),
        focused: view.focused,
    }
}

fn item_line<'a>(item: &'a Item, c: &Colours) -> Line<'a> {
    let label = |text: &str, focused: bool| {
        let style = if focused {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            c.muted()
        };
        Span::styled(
            format!("{text:<width$}", width = usize::from(LABEL_WIDTH)),
            style,
        )
    };
    match item {
        Item::Text(field) => {
            let value = if field.text.is_empty() {
                Span::styled(field.placeholder.as_str(), c.muted())
            } else {
                Span::raw(field.text.as_str())
            };
            let mut spans = vec![label(&field.label, field.focused)];
            if field.focused && field.text.is_empty() {
                // The caret sits before the placeholder, which typing replaces.
                spans.push(Span::styled(" ", c.cursor()));
                spans.push(value);
            } else {
                spans.push(value);
                if field.focused {
                    spans.push(Span::styled(" ", c.cursor()));
                }
            }
            Line::from(spans)
        }
        Item::Select {
            label: text,
            value,
            focused,
        } => {
            let value = if *focused {
                Span::styled(format!("\u{2039} {value} \u{203a}"), c.selection())
            } else {
                Span::raw(value.as_str())
            };
            Line::from(vec![label(text, *focused), value])
        }
        Item::Heading(text) => Line::from(Span::styled(text.as_str(), c.muted())),
        Item::Note(text) => Line::from(Span::styled(text.as_str(), c.muted())),
        Item::Fact(name, value) => Line::from(vec![label(name, false), Span::raw(value.as_str())]),
        Item::Refusal(text) => Line::from(Span::styled(text.as_str(), c.negative())),
    }
}

/// `enter Add · tab next field · up/down choose · esc cancel`: the submit, dimmed while it can't
/// confirm and in the danger colour when it destroys something, then the keys the Presenter says
/// the Dialog takes, each chord read from `keymap`.
fn footer_line<'a>(dialog: &DialogFrame, keymap: &Keymap<DialogCommand>, c: &Colours) -> Line<'a> {
    let submit = if !dialog.can_confirm {
        c.muted()
    } else if dialog.destructive {
        c.negative().add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    let confirm = keymap.chord(SELECT).unwrap_or_default();
    let mut spans = vec![Span::styled(format!("{confirm} {}", dialog.submit), submit)];
    for (chord, action) in dialog.legend(keymap) {
        spans.push(Span::styled(
            format!("{FOOTER_SEPARATOR}{chord} {action}"),
            c.muted(),
        ));
    }
    Line::from(spans)
}

/// A window `content_rows` tall plus its border, centred across `area` and anchored in its top
/// third, shrunk to fit a short terminal.
fn popup_rect(area: Rect, content_rows: u16) -> Rect {
    let width = POPUP_WIDTH.min(area.width);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let height = content_rows.saturating_add(2).min(area.height);
    let anchor_y = area.y + (area.height / 6).max(1);
    let y = anchor_y.min(area.y + area.height.saturating_sub(height));
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use lib_viper::view::Choice;

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn a_text_field_appends_and_backspaces() {
        assert_eq!(
            edited("ab", key(KeyCode::Char('c'))).as_deref(),
            Some("abc")
        );
        assert_eq!(edited("ab", key(KeyCode::Backspace)).as_deref(), Some("a"));
        assert_eq!(edited("", key(KeyCode::Backspace)), None);
        let ctrl = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(edited("ab", ctrl), None);
    }

    #[test]
    fn a_select_steps_and_stops_at_either_end() {
        let select = SelectView {
            label: String::new(),
            options: (1..=3)
                .map(|value| Choice {
                    value,
                    label: value.to_string(),
                })
                .collect(),
            chosen: Some(1),
            focused: true,
        };
        assert_eq!(stepped(&select, 1), Some(2));
        assert_eq!(stepped(&select, -1), Some(1));
        assert_eq!(stepped(&select, 5), Some(3));
    }

    #[test]
    fn the_window_stays_inside_a_short_terminal() {
        let area = Rect::new(0, 0, 60, 10);
        let window = popup_rect(area, 20);
        assert!(window.width <= area.width && window.height <= area.height);
        assert!(window.y + window.height <= area.height);
    }
}
