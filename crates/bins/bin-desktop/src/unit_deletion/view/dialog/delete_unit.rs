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
use lib_toast::{Notice, Notify};
use lib_unit_deletion::SharedUnitDeletion;
use lib_unit_deletion::presenter::{DeleteUnitEvent, DeleteUnitPresenter, DeleteUnitViewModel};
use lib_units::UnitsRouter;
use lib_units::presenter::UnitsOutcome;
use lib_viper::view::SelectView;

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
use crate::units::{DesktopPorts, Landed, help, refusal};

/// The open Delete Dialog's clicks.
type Clicks = DialogClicks<DeleteUnitDialog>;

/// A select the Dialog draws, in the order it draws them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    NewBase,
    /// The Usual unit of the Institution at this index of the view model's selects.
    UsualUnit(usize),
}

/// The selects `view` draws, top to bottom.
fn fields(view: &DeleteUnitViewModel) -> Vec<Field> {
    let new_base = view.unit.new_base.iter().map(|_| Field::NewBase);
    let usual_units = view
        .usual_units
        .iter()
        .flat_map(|usual_units| (0..usual_units.selects.len()).map(Field::UsualUnit));
    new_base.chain(usual_units).collect()
}

/// The open Delete unit Dialog: its Presenter, the Units ports and Ledger read copy its confirm
/// brings in line, each select's widget state, and the Units page's landing.
pub struct DeleteUnitDialog {
    presenter: DeleteUnitPresenter,
    ports: DesktopPorts,
    ledger: Entity<LedgerStore>,
    /// Each select's open list and highlight, in [`fields`] order.
    selects: Vec<SelectState>,
    /// The select `Up`/`Down` drive, an index into [`fields`].
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

    /// The widget state of the select at `index` of [`fields`], made as it is first drawn.
    fn select_state(&mut self, index: usize) -> &mut SelectState {
        if self.selects.len() <= index {
            self.selects.resize_with(index + 1, SelectState::default);
        }
        &mut self.selects[index]
    }

    /// A select key on `field`, the select at `index`: a changed choice goes to the Presenter.
    fn select_key(
        &mut self,
        view: &DeleteUnitViewModel,
        index: usize,
        field: Field,
        key: DialogKey,
    ) {
        match field {
            Field::NewBase => {
                let Some(new_base) = &view.unit.new_base else {
                    return;
                };
                if let (_, Some(id)) = select_key(self.select_state(index), &new_base.select, key) {
                    self.send(DeleteUnitEvent::NewBase(id));
                }
            }
            Field::UsualUnit(n) => {
                let Some(named) = usual_unit(view, n) else {
                    return;
                };
                if let (_, Some(unit)) = select_key(self.select_state(index), &named.1, key) {
                    self.send(DeleteUnitEvent::UsualUnit {
                        institution: named.0,
                        unit,
                    });
                }
            }
        }
    }

    /// A click on the select at `index`: it takes the arrow keys, and its list opens or closes.
    fn click_select(&mut self, index: usize) {
        let view = self.view();
        let Some(&field) = fields(&view).get(index) else {
            return;
        };
        self.focus = index;
        match field {
            Field::NewBase => {
                if let Some(new_base) = &view.unit.new_base {
                    toggle_select(self.select_state(index), &new_base.select);
                }
            }
            Field::UsualUnit(n) => {
                if let Some((_, select)) = usual_unit(&view, n) {
                    toggle_select(self.select_state(index), &select);
                }
            }
        }
    }

    /// A click on row `option` of the open list of the select at `index`.
    fn click_option(&mut self, index: usize, option: usize) {
        let view = self.view();
        match fields(&view).get(index) {
            Some(Field::NewBase) => {
                let chosen = view.unit.new_base.as_ref().and_then(|new_base| {
                    choose_option(self.select_state(index), &new_base.select, option)
                });
                if let Some(id) = chosen {
                    self.send(DeleteUnitEvent::NewBase(id));
                }
            }
            Some(&Field::UsualUnit(n)) => {
                let Some((institution, select)) = usual_unit(&view, n) else {
                    return;
                };
                if let Some(unit) = choose_option(self.select_state(index), &select, option) {
                    self.send(DeleteUnitEvent::UsualUnit { institution, unit });
                }
            }
            None => {}
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

/// The Institution and select of the `n`th Usual unit select.
fn usual_unit(
    view: &DeleteUnitViewModel,
    n: usize,
) -> Option<(lib_core::InstitutionId, SelectView<Option<UnitId>>)> {
    let named = view.usual_units.as_ref()?.selects.get(n)?;
    Some((named.institution, named.select.clone()))
}

/// Where the Delete Dialog's Presenter asks to go: closing, which the Dialog reads, and Notices,
/// which `Root` drains once the key or click returns. It opens no Dialog.
#[derive(Default)]
struct DialogRouter {
    /// `Some` once the Presenter closed the Dialog, with what its confirm did.
    closed: Option<Option<UnitsOutcome>>,
}

impl UnitsRouter for DialogRouter {
    fn open_dialog(&mut self, _dialog: lib_units::UnitsDialog) -> bool {
        false
    }

    fn close_dialog(&mut self, outcome: Option<UnitsOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

impl EventEmitter<DialogClosed> for DeleteUnitDialog {}

impl HostedDialog for DeleteUnitDialog {
    /// `Tab`/`Shift-Tab` move between the selects, `Up`/`Down` (and `Enter` on an open list) drive
    /// the one focused, and every other key types into the typed-back code.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        let view = self.view();
        let fields = fields(&view);
        if matches!(key, DialogKey::Tab | DialogKey::BackTab) {
            if !fields.is_empty() {
                self.close_open_select();
                let count = fields.len();
                self.focus = if key == DialogKey::Tab {
                    (self.focus + 1) % count
                } else {
                    (self.focus + count - 1) % count
                };
            }
            return DialogOutcome::Handled;
        }
        if let Some(&field) = fields.get(self.focus) {
            let select_key_now = match key {
                DialogKey::Up | DialogKey::Down => true,
                DialogKey::Enter => self.select_state(self.focus).is_open(),
                _ => false,
            };
            if select_key_now {
                self.select_key(&view, self.focus, field, key);
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
        let focused = match fields(&view).get(self.focus) {
            Some(Field::NewBase) => unit
                .new_base
                .as_ref()
                .map(|new_base| new_base.select.label.clone()),
            Some(&Field::UsualUnit(n)) => usual_unit(&view, n).map(|(_, select)| select.label),
            None => None,
        };
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
        let fields = fields(&view);
        if self.selects.len() < fields.len() {
            self.selects.resize_with(fields.len(), SelectState::default);
        }
        render(
            &view,
            &fields,
            &self.selects,
            self.focus,
            &Clicks::new(cx),
            cx,
        )
    }
}

fn render(
    view: &DeleteUnitViewModel,
    fields: &[Field],
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
    let ring = fields.len() > 1;
    for (index, field) in fields.iter().enumerate() {
        let state = selects.get(index).cloned().unwrap_or_default();
        let on_field_click = actions.click(move |dialog: &mut DeleteUnitDialog| {
            dialog.click_select(index);
        });
        let on_option_click = actions.option(move |dialog: &mut DeleteUnitDialog, option| {
            dialog.click_option(index, option);
        });
        let focused = ring && index == focus;
        match *field {
            Field::NewBase => {
                let Some(new_base) = &unit.new_base else {
                    continue;
                };
                let options = labels(&new_base.select);
                body.push(
                    div()
                        .child(select_field::render(
                            SelectFieldProps {
                                id: "delete-unit-new-base".into(),
                                label: new_base.select.label.clone().into(),
                                options: &options,
                                state: &synced(&state, &new_base.select),
                                focused,
                                accent: false,
                                read_only: None,
                                on_field_click,
                                on_option_click,
                            },
                            cx,
                        ))
                        .child(help(new_base.help.clone(), cx))
                        .into_any_element(),
                );
            }
            Field::UsualUnit(n) => {
                let Some((_, select)) = usual_unit(view, n) else {
                    continue;
                };
                if n == 0
                    && let Some(usual_units) = &view.usual_units
                {
                    body.push(
                        div()
                            .debug_selector(|| "delete-unit-usual-units-help".to_string())
                            .text_size(px(11.5))
                            .text_color(color::muted(cx))
                            .child(usual_units.help.clone())
                            .into_any_element(),
                    );
                }
                let options = labels(&select);
                body.push(select_field::render(
                    SelectFieldProps {
                        id: format!("delete-unit-usual-unit-{n}").into(),
                        label: select.label.clone().into(),
                        options: &options,
                        state: &synced(&state, &select),
                        focused,
                        accent: false,
                        read_only: None,
                        on_field_click,
                        on_option_click,
                    },
                    cx,
                ));
            }
        }
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
