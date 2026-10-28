//! The Delete institution Dialog's View on the shared `crate::dialog` frame's destructive variant:
//! the typed-back name, which holds every typed key, or, when an Account names the Institution, the
//! refusal with the Accounts' names and only a Close button.

use gpui::{
    AnyElement, App, Context, IntoElement, Render, SharedString, Window, div, prelude::*, px,
};
use lib_institutions::presenter::{
    DeleteInstitutionEvent, DeleteInstitutionPresenter, DeleteInstitutionViewModel,
};
use lib_shell::DialogCommand;
use lib_viper::keymap::{BACK, Keymap, SELECT};

use super::{DialogRouter, Landed};
use crate::dialog;
use crate::form::viper::{enter, typed};
use crate::institutions::ports::{self, DesktopPorts};
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use crate::shell::view::statusline::Hint;
use crate::theme::color;

const WIDTH: gpui::Pixels = px(440.0);

/// The open Delete institution Dialog: its Presenter.
pub struct DeleteInstitutionDialog {
    /// The Ledger read copy a confirm brings in line with what it wrote.
    ledger: gpui::Entity<LedgerStore>,
    presenter: DeleteInstitutionPresenter,
    landed: Landed,
}

impl DeleteInstitutionDialog {
    /// The Dialog for Institution `id` on `ports`, or `None` when it is gone. A confirm hands its
    /// outcome to `landed`.
    pub(crate) fn open(
        id: lib_core::InstitutionId,
        ports: &DesktopPorts,
        ledger: &gpui::Entity<LedgerStore>,
        landed: Landed,
    ) -> Option<Self> {
        Some(Self {
            ledger: ledger.clone(),
            presenter: DeleteInstitutionPresenter::new(ports.clone(), id)?,
            landed,
        })
    }

    fn send(&mut self, event: DeleteInstitutionEvent) -> DialogRouter {
        let mut router = DialogRouter::default();
        self.presenter.handle(event, &mut router);
        router
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

impl gpui::EventEmitter<DialogClosed> for DeleteInstitutionDialog {}

impl HostedDialog for DeleteInstitutionDialog {
    /// Typing fills the confirmation; `Enter` deletes once it is typed back, or closes a refusal.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        let view = self.presenter.view_model();
        let Some(confirm) = view.confirm else {
            return match key {
                DialogKey::Enter => DialogOutcome::Confirm,
                _ => DialogOutcome::Handled,
            };
        };
        match key {
            DialogKey::Enter => enter(view.frame.can_confirm),
            DialogKey::Tab | DialogKey::BackTab | DialogKey::Up | DialogKey::Down => {
                DialogOutcome::Handled
            }
            _ => match typed(&confirm.text, key) {
                Some(text) => {
                    self.send(DeleteInstitutionEvent::ConfirmText(text));
                    DialogOutcome::Handled
                }
                None => DialogOutcome::Ignored,
            },
        }
    }

    fn can_confirm(&self) -> bool {
        self.presenter.view_model().frame.can_confirm
    }

    /// Sends Confirm to the Presenter, which runs its Interactor, with the Ledger read copy in
    /// line either side. A refusal turns the Dialog into the refusal and keeps it open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ledger, cx);
        let router = self.send(DeleteInstitutionEvent::Confirm);
        ports::refresh(&self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.send(DeleteInstitutionEvent::Cancel);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        false
    }

    /// Cancel, and delete while the name can be typed back.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        let mut hints = vec![(key(BACK), crate::msg::desktop_hint_cancel())];
        if self.presenter.view_model().confirm.is_some() {
            hints.push((key(SELECT), crate::msg::desktop_hint_confirm()));
        }
        Some(hints)
    }

    /// The title, the typed-back name, what the Dialog says and the Accounts it lists, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.presenter.view_model();
        DialogReadback {
            kind: "DeleteInstitution",
            values: vec![
                ("title", view.frame.title.clone()),
                (
                    "confirm",
                    view.confirm
                        .as_ref()
                        .map(|field| field.text.clone())
                        .unwrap_or_default(),
                ),
                ("notice", view.notice.clone()),
                ("accounts", view.accounts.join(", ")),
                ("more", view.more.clone().unwrap_or_default()),
                ("refused", view.confirm.is_none().to_string()),
                ("can_confirm", view.frame.can_confirm.to_string()),
            ],
        }
    }
}

impl Render for DeleteInstitutionDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = DialogClicks::new(cx);
        render_view(&self.presenter.view_model(), &actions, cx)
    }
}

fn render_view(
    view: &DeleteInstitutionViewModel,
    actions: &DialogClicks<DeleteInstitutionDialog>,
    cx: &App,
) -> AnyElement {
    let mut fields = vec![
        div()
            .text_size(px(13.0))
            .child(view.notice.clone())
            .into_any_element(),
    ];
    if !view.accounts.is_empty() {
        fields.push(accounts_list(view, cx));
    }
    fields.extend(
        view.hint
            .clone()
            .map(|hint| dialog::info_panel(hint, cx).into_any_element()),
    );
    if let Some(confirm) = &view.confirm {
        fields.push(confirm_field(
            &confirm.label,
            &confirm.text,
            &confirm.placeholder,
            cx,
        ));
    }
    let buttons = match view.confirm {
        Some(_) => vec![
            dialog::cancel_button("delete-institution-cancel", actions.cancel.clone(), cx)
                .into_any_element(),
            dialog::confirm_button(
                "delete-institution-confirm",
                view.frame.submit.clone(),
                view.frame.can_confirm,
                true,
                actions.confirm.clone(),
                cx,
            )
            .into_any_element(),
        ],
        // A refusal offers Close and nothing destructive.
        None => vec![
            dialog::confirm_button(
                "delete-institution-close",
                view.frame.submit.clone(),
                true,
                false,
                actions.cancel.clone(),
                cx,
            )
            .into_any_element(),
        ],
    };
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            view.frame.title.clone(),
            view.frame.destructive,
            cx,
        ))
        .child(dialog::body(fields))
        .child(dialog::action_row(buttons, cx));
    dialog::overlay(WIDTH, view.frame.destructive, card, cx)
}

/// The Accounts naming the Institution, one to a line, then `and N more`.
fn accounts_list(view: &DeleteInstitutionViewModel, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .text_size(px(13.0))
        .children(view.accounts.iter().enumerate().map(|(index, name)| {
            let id = SharedString::from(format!("delete-institution-account-{index}"));
            div()
                .debug_selector({
                    let id = id.clone();
                    move || id.to_string()
                })
                .id(id)
                .font_weight(gpui::FontWeight::BOLD)
                .child(name.clone())
        }))
        .children(view.more.clone().map(|more| {
            div()
                .debug_selector(|| "delete-institution-more".to_string())
                .id("delete-institution-more")
                .text_color(color::muted(cx))
                .child(more)
        }))
        .into_any_element()
}

/// The typed-back name, its placeholder the name itself.
fn confirm_field(label: &str, text: &str, placeholder: &str, cx: &App) -> AnyElement {
    let (shown, text_color) = if text.is_empty() {
        (placeholder.to_string(), color::muted(cx))
    } else {
        (text.to_string(), color::foreground(cx))
    };
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(label.to_string()),
        )
        .child(
            div()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(color::border(cx))
                .text_size(px(13.0))
                .text_color(text_color)
                .child(SharedString::from(shown)),
        )
        .into_any_element()
}
