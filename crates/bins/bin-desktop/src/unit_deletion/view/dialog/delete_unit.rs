//! The Delete unit Dialog (16d) in the red treatment, an Entity in the Dialog host
//! (`shell::view::dialog_entity`), with the sections that apply: the Unit that takes over as base
//! when it is the base, and a Usual unit select for each Institution that names it. A Unit
//! something else names opens as the Deactivate unit Dialog instead. Every key and click becomes a
//! `lib-unit-deletion` Presenter event, and this draws the view model and nothing else.
//!
//! Keys: the typed-back code always holds the typing, so the selects are driven by `Up`/`Down`
//! (`Enter` commits an open list) on the select `Tab`/`Shift-Tab` move to, and by the mouse.
//! Confirm and Cancel go to the Presenter, which closes the Dialog through `lib-units`' Router; a
//! confirmed outcome is handed to the Units page, and the host closes on [`DialogClosed`].

use std::rc::Rc;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, Render, Window, div, prelude::*, px};
use lib_core::UnitId;
use lib_unit_deletion::SharedUnitDeletion;
use lib_unit_deletion::presenter::{
    DeleteUnitEvent, DeleteUnitPresenter, DeleteUnitViewModel, UsualUnitSelect,
};
use lib_units::presenter::NewBaseView;

use crate::dialog;
use crate::form::fields::text_field;
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{
    choose_option, chosen_label, enter, labels, select_key, synced, toggle_select, typed,
};
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use crate::theme::color;
use crate::units::{DesktopPorts, DialogRouter, Landed, help, refusal};

/// The open Delete Dialog's clicks.
type Clicks = DialogClicks<DeleteUnitDialog>;

/// A select the Dialog draws, found by its position: the New base unit first when the Unit is the
/// base, then the Usual unit of each Institution naming it, in the Ledger's order.
enum Select<'a> {
    NewBase(&'a NewBaseView),
    /// The `n`th Usual unit select.
    UsualUnit(usize, &'a UsualUnitSelect),
}

/// The select at `index`, top to bottom.
fn select(view: &DeleteUnitViewModel, index: usize) -> Option<Select<'_>> {
    let new_base = view.unit.new_base.as_ref();
    let usual_units = view
        .usual_units
        .as_ref()
        .map(|usual_units| &usual_units.selects[..]);
    match (new_base, index) {
        (Some(new_base), 0) => Some(Select::NewBase(new_base)),
        (new_base, _) => {
            let n = index - usize::from(new_base.is_some());
            Some(Select::UsualUnit(n, usual_units?.get(n)?))
        }
    }
}

/// How many selects `view` draws.
fn select_count(view: &DeleteUnitViewModel) -> usize {
    let usual_units = view
        .usual_units
        .as_ref()
        .map_or(0, |usual_units| usual_units.selects.len());
    usize::from(view.unit.new_base.is_some()) + usual_units
}

/// The event for a new Usual unit on the Institution `named` selects for.
fn usual_unit_chosen(named: &UsualUnitSelect, unit: Option<UnitId>) -> DeleteUnitEvent {
    DeleteUnitEvent::UsualUnit {
        institution: named.institution,
        unit,
    }
}

impl Select<'_> {
    fn label(&self) -> &str {
        match self {
            Self::NewBase(new_base) => &new_base.select.label,
            Self::UsualUnit(_, named) => &named.select.label,
        }
    }

    fn options(&self) -> Vec<String> {
        match self {
            Self::NewBase(new_base) => labels(&new_base.select),
            Self::UsualUnit(_, named) => labels(&named.select),
        }
    }

    fn synced(&self, state: &SelectState) -> SelectState {
        match self {
            Self::NewBase(new_base) => synced(state, &new_base.select),
            Self::UsualUnit(_, named) => synced(state, &named.select),
        }
    }

    fn toggle(&self, state: &mut SelectState) {
        match self {
            Self::NewBase(new_base) => toggle_select(state, &new_base.select),
            Self::UsualUnit(_, named) => toggle_select(state, &named.select),
        }
    }

    /// The event a key on the select sends, when it changed the choice.
    fn key(&self, state: &mut SelectState, key: DialogKey) -> Option<DeleteUnitEvent> {
        match self {
            Self::NewBase(new_base) => select_key(state, &new_base.select, key)
                .1
                .map(DeleteUnitEvent::NewBase),
            Self::UsualUnit(_, named) => {
                let unit = select_key(state, &named.select, key).1?;
                Some(usual_unit_chosen(named, unit))
            }
        }
    }

    /// The event a click on row `option` of the open list sends.
    fn choose(&self, state: &mut SelectState, option: usize) -> Option<DeleteUnitEvent> {
        match self {
            Self::NewBase(new_base) => {
                choose_option(state, &new_base.select, option).map(DeleteUnitEvent::NewBase)
            }
            Self::UsualUnit(_, named) => {
                let unit = choose_option(state, &named.select, option)?;
                Some(usual_unit_chosen(named, unit))
            }
        }
    }
}

/// The open Delete unit Dialog: its Presenter, the Units ports and Ledger read copy its confirm
/// brings in line, each select's widget state, and the Units page's landing.
pub struct DeleteUnitDialog {
    presenter: DeleteUnitPresenter,
    ports: DesktopPorts,
    ledger: Entity<LedgerStore>,
    /// Each select's open list and highlight by position.
    selects: Vec<SelectState>,
    /// The select `Up`/`Down` drive, a position in [`select`].
    focus: usize,
    landed: Landed,
}

impl DeleteUnitDialog {
    /// The Dialog for Unit `id`, or `None` when it is gone. Usage is copied from the Ledger first,
    /// so the Presenter sees what names the Unit now.
    pub(crate) fn open(
        id: UnitId,
        ports: &DesktopPorts,
        deletion: &SharedUnitDeletion,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
        cx: &App,
    ) -> Option<Self> {
        crate::units::ports::copy_usage(ports, ledger.read(cx).ledger());
        let presenter = DeleteUnitPresenter::new(ports.clone(), deletion.clone(), id)?;
        Some(Self {
            presenter,
            ports: ports.clone(),
            ledger: ledger.clone(),
            selects: Vec::new(),
            focus: 0,
            landed,
        })
    }

    fn view(&self) -> DeleteUnitViewModel {
        self.presenter.view_model()
    }

    /// Sends `event` to the Presenter, returning what its Router was asked.
    fn send(&mut self, event: DeleteUnitEvent) -> DialogRouter {
        let mut router = DialogRouter::default();
        self.presenter.handle(event, &mut router);
        router
    }

    /// The widget state of the select at `index`, made as it is first drawn.
    fn select_state(&mut self, index: usize) -> &mut SelectState {
        if self.selects.len() <= index {
            self.selects.resize_with(index + 1, SelectState::default);
        }
        &mut self.selects[index]
    }

    /// A select key on the select at `index`: a changed choice goes to the Presenter.
    fn select_key(&mut self, view: &DeleteUnitViewModel, index: usize, key: DialogKey) {
        let Some(select) = select(view, index) else {
            return;
        };
        if let Some(event) = select.key(self.select_state(index), key) {
            self.send(event);
        }
    }

    /// A click on the select at `index`: it takes the arrow keys, and its list opens or closes.
    fn click_select(&mut self, index: usize) {
        let view = self.view();
        let Some(select) = select(&view, index) else {
            return;
        };
        self.focus = index;
        select.toggle(self.select_state(index));
    }

    /// A click on row `option` of the open list of the select at `index`.
    fn click_option(&mut self, index: usize, option: usize) {
        let view = self.view();
        let Some(select) = select(&view, index) else {
            return;
        };
        if let Some(event) = select.choose(self.select_state(index), option) {
            self.send(event);
        }
    }

    /// Hands a confirmed outcome to the page and tells the host, once the Presenter has closed
    /// the Dialog.
    fn closed(&self, router: DialogRouter, cx: &mut Context<'_, Self>) {
        let Some(outcome) = router.closed else {
            return;
        };
        if let Some(outcome) = outcome {
            (self.landed)(outcome, cx);
        }
        cx.emit(DialogClosed);
    }
}

impl EventEmitter<DialogClosed> for DeleteUnitDialog {}

impl HostedDialog for DeleteUnitDialog {
    /// `Tab`/`Shift-Tab` move between the selects, `Up`/`Down` (and `Enter` on an open list) drive
    /// the one focused, and every other key types into the typed-back code.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        let view = self.view();
        let count = select_count(&view);
        if matches!(key, DialogKey::Tab | DialogKey::BackTab) {
            if count > 0 {
                self.close_open_select();
                self.focus = if key == DialogKey::Tab {
                    (self.focus + 1) % count
                } else {
                    (self.focus + count - 1) % count
                };
            }
            return DialogOutcome::Handled;
        }
        if self.focus < count {
            let select_key_now = match key {
                DialogKey::Up | DialogKey::Down => true,
                DialogKey::Enter => self.select_state(self.focus).is_open(),
                _ => false,
            };
            if select_key_now {
                self.select_key(&view, self.focus, key);
                return DialogOutcome::Handled;
            }
        }
        match key {
            DialogKey::Enter => enter(view.unit.frame.can_confirm),
            _ => match view
                .unit
                .confirm
                .as_ref()
                .and_then(|confirm| typed(&confirm.text, key))
            {
                Some(text) => {
                    self.send(DeleteUnitEvent::ConfirmText(text));
                    DialogOutcome::Handled
                }
                None => DialogOutcome::Ignored,
            },
        }
    }

    fn can_confirm(&self) -> bool {
        self.view().unit.frame.can_confirm
    }

    /// Sends Confirm to the Presenter, which deletes or deactivates, with the Ledger's read copies
    /// in line either side. A refusal stays on the view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        crate::units::ports::refresh(&self.ports, &self.ledger, cx);
        let router = self.send(DeleteUnitEvent::Confirm);
        crate::unit_deletion::refresh(&self.ports, &self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled delete has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.send(DeleteUnitEvent::Cancel);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        let mut closed = false;
        for select in &mut self.selects {
            closed |= select.is_open();
            select.cancel();
        }
        closed
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.view();
        let unit = &view.unit;
        let usual_units = view.usual_units.as_ref();
        let focused = select(&view, self.focus).map(|select| select.label().to_string());
        DialogReadback {
            kind: "DeleteUnit",
            values: vec![
                ("title", unit.frame.title.clone()),
                (
                    "confirm",
                    unit.confirm
                        .as_ref()
                        .map(|confirm| confirm.text.clone())
                        .unwrap_or_default(),
                ),
                ("in_use", unit.in_use.is_some().to_string()),
                (
                    "in_use_text",
                    unit.in_use
                        .as_ref()
                        .map(|in_use| in_use.text.clone())
                        .unwrap_or_default(),
                ),
                (
                    "new_base",
                    unit.new_base
                        .as_ref()
                        .and_then(|new_base| chosen_label(&new_base.select))
                        .unwrap_or_default(),
                ),
                (
                    "new_base_options",
                    unit.new_base
                        .as_ref()
                        .map(|new_base| labels(&new_base.select).join("|"))
                        .unwrap_or_default(),
                ),
                (
                    "usual_units_help",
                    usual_units
                        .map(|usual_units| usual_units.help.clone())
                        .unwrap_or_default(),
                ),
                // `Coinspot=None|Kraken=vas — Vanguard Aus Shares`, in the Ledger's order.
                (
                    "usual_units",
                    usual_units
                        .map(|usual_units| {
                            usual_units
                                .selects
                                .iter()
                                .map(|named| {
                                    let chosen = chosen_label(&named.select).unwrap_or_default();
                                    format!("{}={chosen}", named.select.label)
                                })
                                .collect::<Vec<_>>()
                                .join("|")
                        })
                        .unwrap_or_default(),
                ),
                (
                    "usual_unit_options",
                    usual_units
                        .and_then(|usual_units| usual_units.selects.first())
                        .map(|named| labels(&named.select).join("|"))
                        .unwrap_or_default(),
                ),
                ("focused", focused.unwrap_or_default()),
                (
                    "select_open",
                    self.selects.iter().any(SelectState::is_open).to_string(),
                ),
                ("refusal", unit.refusal.clone().unwrap_or_default()),
                ("can_confirm", unit.frame.can_confirm.to_string()),
            ],
        }
    }
}

impl Render for DeleteUnitDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let view = self.view();
        let count = select_count(&view);
        if self.selects.len() < count {
            self.selects.resize_with(count, SelectState::default);
        }
        render(&view, &self.selects, self.focus, &Clicks::new(cx), cx)
    }
}

fn render(
    view: &DeleteUnitViewModel,
    selects: &[SelectState],
    focus: usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let unit = &view.unit;
    let note = |text: String| {
        div()
            .text_size(px(11.5))
            .text_color(color::muted(cx))
            .child(text)
            .into_any_element()
    };
    let mut body = Vec::new();
    if let Some(in_use) = &unit.in_use {
        body.push(
            div()
                .debug_selector(|| "delete-unit-in-use".to_string())
                .text_size(px(13.0))
                .child(in_use.text.clone())
                .into_any_element(),
        );
        body.push(note(in_use.help.clone()));
        body.extend(in_use.base.clone().map(|base| {
            div()
                .debug_selector(|| "delete-unit-base-note".to_string())
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(base)
                .into_any_element()
        }));
    }
    // The focus ring shows which select the arrow keys drive, needed only when there are two.
    let ring = select_count(view) > 1;
    for index in 0..select_count(view) {
        let Some(select) = select(view, index) else {
            continue;
        };
        let state = selects.get(index).cloned().unwrap_or_default();
        let on_field_click = actions.click(move |dialog: &mut DeleteUnitDialog| {
            dialog.click_select(index);
        });
        let on_option_click = actions.option(move |dialog: &mut DeleteUnitDialog, option| {
            dialog.click_option(index, option);
        });
        let focused = ring && index == focus;
        let options = select.options();
        let state = select.synced(&state);
        let (id, help_line) = match &select {
            Select::NewBase(new_base) => (
                "delete-unit-new-base".to_string(),
                Some(help(new_base.help.clone(), cx)),
            ),
            Select::UsualUnit(n, _) => (format!("delete-unit-usual-unit-{n}"), None),
        };
        // The Usual units' help sits once above the first of their selects.
        if let (Select::UsualUnit(0, _), Some(usual_units)) = (&select, &view.usual_units) {
            body.push(
                div()
                    .debug_selector(|| "delete-unit-usual-units-help".to_string())
                    .text_size(px(11.5))
                    .text_color(color::muted(cx))
                    .child(usual_units.help.clone())
                    .into_any_element(),
            );
        }
        let field = select_field::render(
            SelectFieldProps {
                id: id.into(),
                label: select.label().to_string().into(),
                options: &options,
                state: &state,
                focused,
                accent: false,
                read_only: None,
                on_field_click,
                on_option_click,
            },
            cx,
        );
        body.push(match help_line {
            Some(help_line) => div().child(field).child(help_line).into_any_element(),
            None => field,
        });
    }
    if let Some(confirm) = &unit.confirm {
        body.push(
            text_field(
                "delete-unit-confirm-input",
                confirm.label.clone(),
                &confirm.text,
                &confirm.placeholder,
                confirm.focused,
                Rc::new(|_window: &mut Window, _cx: &mut App| {}),
                cx,
            )
            .into_any_element(),
        );
    }
    body.extend(refusal(unit.refusal.clone(), cx));

    let destructive = unit.frame.destructive;
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(unit.frame.title.clone(), destructive, cx))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-unit-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-unit-confirm",
                    unit.frame.submit.clone(),
                    unit.frame.can_confirm,
                    destructive,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, destructive, card, cx)
}
