//! `Render for Shell`, the page props and the View switch of `Shell`. An `impl Shell` block for the
//! concern, not a real module (`view/` holds the Views themselves).

use super::Shell;
use crate::settings::documents::dialogs::{
    DocumentTypesDialog, DocumentTypesDialogHandlers,
    add_edit::OnFieldClick as DocumentTypeFieldClick,
};
use crate::settings::payees::{
    dialogs::{OnFieldClick, OnOptionClick, PayeeDialogHandlers, PayeesDialog},
    form,
};
use lib_documents::types::{RemindLead, TracksDate};
use std::rc::Rc;

use gpui_component::scroll::{Scrollbar, ScrollbarShow};

use crate::settings::inventory::dialogs::{InventoryDialog, InventoryDialogHandlers};

use gpui::{
    Context, ExternalPaths, KeyDownEvent, MouseDownEvent, ScrollHandle, Window, div, prelude::*, px,
};

use crate::{
    bills::{self},
    budgets,
    chrome::dialog_host::{Dialog, DialogActions},
    chrome::rail::{self as chrome_rail, context::ContextRail, primary::PrimaryRail},
    chrome::statusline::{self, StatusLine},
    chrome::toast::history as toast_history_view,
    chrome::topbar::{self, TopBar},
    chrome::window_size::WindowSize,
    documents::{self},
    navigation::explorer::{self as explorer_view},
    navigation::nav::{FocusZone, InputMode},
    period::Period,
    settings::tracing::dialogs::TracingDialog,
    theme::{color, type_scale},
    view::{
        active::{ActiveViewEntity, ShellView},
        documents::{self as documents_view},
        help::{self as help_view, HelpDialog},
        placeholder,
    },
};

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let window_size = WindowSize::of(window);
        // A `b` override lasts only until the window crosses a Breakpoint (ADR-0042); dropped
        // here, so coming back to the Window Size it was set at draws that size's default.
        if self
            .chrome
            .rail_override
            .is_some_and(|rail_override| rail_override.set_at != window_size)
        {
            self.chrome.rail_override = None;
        }
        let rail_mode = self.primary_rail(window_size, cx);
        let focus = self.nav.focus();
        let active_entity = self.active_view_entity(cx);
        // The "1d" spec: "The shell behind the palette drops to 30% opacity" -- the "1e" file
        // explorer reuses the same dimming pattern (Implementation note 10). Applied to the top
        // bar and content row only, not the status line -- that same spec separately describes
        // the status line's own COMMAND-mode content (the live query, "esc close command
        // window"), which stays meaningful precisely because it stays legible; only the
        // navigational chrome the palette/explorer visually floats over goes dim.
        let content_opacity =
            if self.chrome.palette.is_some() || self.file_explorer.is_some() || self.help_open() {
                0.3
            } else {
                1.0
            };

        // Both closures go through an `Entity` handle rather than `cx.listener`:
        // `on_click`/`on_hover`'s own signatures are `Fn(_, &mut Window, &mut App)`, with no
        // `&mut Shell` parameter for `cx.listener` to supply, and the collapsed rail's
        // `on_row_hover` closure additionally needs to close over each row's own `Noun` --
        // `PrimaryRail` curries that in per-row from the single `Rc` given here.
        let entity = cx.entity();
        let on_rail_toggle: topbar::OnRailToggle = {
            let entity = entity.clone();
            Rc::new(move |_event, window, cx| {
                let window_size = WindowSize::of(window);
                entity.update(cx, |shell, cx| shell.handle_toggle_rail(window_size, cx));
            })
        };
        let on_topbar_action: topbar::OnTopBarAction = {
            let entity = entity.clone();
            Rc::new(move |action, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_topbar_action(action, cx));
            })
        };
        let on_row_hover: chrome_rail::primary::OnRowHover = {
            let entity = entity.clone();
            Rc::new(move |noun, hovered, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_rail_hover(noun, hovered, cx));
            })
        };
        let on_row_click: chrome_rail::primary::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |noun, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_rail_click(noun, cx));
            })
        };
        let on_explorer_entry_click: explorer_view::OnEntryClick = {
            let entity = entity.clone();
            Rc::new(move |path, click_count, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_explorer_entry_click(path, click_count, cx)
                });
            })
        };
        let on_explorer_breadcrumb_click: explorer_view::OnBreadcrumbClick = {
            let entity = entity.clone();
            Rc::new(move |path, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_explorer_breadcrumb_click(path, cx)
                });
            })
        };
        let on_explorer_filter_toggle: explorer_view::OnFilterToggle = {
            let entity = entity.clone();
            Rc::new(move |filter, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_explorer_filter_toggle(filter, cx)
                });
            })
        };
        let on_hint: statusline::OnHint = {
            let entity = entity.clone();
            Rc::new(move |action, window, cx| {
                let window_size = WindowSize::of(window);
                entity.update(cx, |shell, cx| {
                    shell.handle_hint_click(action, window_size, cx)
                });
            })
        };
        let on_toast_dismiss: crate::chrome::toast::OnDismiss = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.chrome.toasts.dismiss_visible(index);
                    cx.notify();
                });
            })
        };
        let on_toast_hover: crate::chrome::toast::OnHover = {
            let entity = entity.clone();
            Rc::new(move |hovered, _window, cx| {
                entity.update(cx, |shell, _cx| shell.chrome.toasts_hovered = hovered);
            })
        };
        // Hidden while the history is open, which lists them in full.
        let toast_history_open = self.toast_history_open();
        let toast_layer = if toast_history_open {
            None
        } else {
            crate::chrome::toast::render(
                &self.chrome.toasts,
                on_toast_dismiss,
                on_toast_hover,
                window.viewport_size().width,
                cx,
            )
        };
        let on_toast_history_close: toast_history_view::OnClose = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.close_dialog();
                    cx.notify();
                });
            })
        };
        let on_help_close: help_view::OnClose = on_toast_history_close.clone();
        let on_explorer_cancel: explorer_view::OnCancel = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_explorer_cancel(cx));
            })
        };
        let on_explorer_open: explorer_view::OnOpen = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_explorer_open(cx));
            })
        };
        let on_settings_dialog_cancel: crate::dialog::OnClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_dialog_cancel(cx));
            })
        };
        let on_settings_dialog_confirm: crate::dialog::OnClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_dialog_confirm(cx));
            })
        };
        // The hosted Dialogs' clicks: an edit runs on the open Dialog and raises the Toast it
        // returns; cancel and confirm run the host's cycle.
        let dialog_actions = DialogActions {
            edit: {
                let entity = entity.clone();
                Rc::new(move |cx, change| {
                    entity.update(cx, |shell, cx| {
                        let toast = shell
                            .chrome
                            .dialog
                            .as_mut()
                            .and_then(|dialog| change(dialog, cx));
                        if let Some((kind, message)) = toast {
                            shell.raise_toast(kind, message);
                        }
                        cx.notify();
                    });
                })
            },
            cancel: on_settings_dialog_cancel.clone(),
            confirm: on_settings_dialog_confirm.clone(),
        };
        let plain_payees = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let payees_dialog_handlers = {
            let plain = plain_payees;
            let form_edit = |change: fn(&mut form::PayeeForm, usize)| {
                let entity = entity.clone();
                let on_click: OnOptionClick = Rc::new(move |index, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut PayeesDialog| {
                            if let Some(form) = dialog.form_mut() {
                                change(form, index);
                            }
                        });
                    });
                });
                on_click
            };
            let on_field_click: OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut PayeesDialog| {
                            if let Some(form) = dialog.form_mut() {
                                if field == form::PayeeField::DefaultCategory {
                                    form.click_select();
                                } else {
                                    form.focus(field);
                                }
                            }
                        });
                    });
                })
            };
            let add_rule = |shell: &mut Shell, cx: &mut Context<'_, Shell>| {
                shell.edit_dialog(cx, |dialog: &mut PayeesDialog| {
                    if let Some(form) = dialog.form_mut() {
                        form.focus(form::PayeeField::Rule);
                        form.add_rule();
                    }
                });
            };
            PayeeDialogHandlers {
                on_field_click,
                on_option_click: form_edit(form::PayeeForm::choose_category),
                on_add_rule: plain(add_rule),
                on_remove_rule: form_edit(form::PayeeForm::remove_rule),
                on_cancel: plain(Shell::handle_dialog_cancel),
                on_confirm: plain(Shell::handle_dialog_confirm),
            }
        };
        let documents_dialog_handlers = {
            let on_field_click: DocumentTypeFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut DocumentTypesDialog| {
                            dialog.click_field(field)
                        });
                    });
                })
            };
            let on_tracks_click: crate::form::fields::OnSegmentClick<Option<TracksDate>> = {
                let entity = entity.clone();
                Rc::new(move |option, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut DocumentTypesDialog| {
                            dialog.click_tracks(option)
                        });
                    });
                })
            };
            let on_remind_click: crate::form::fields::OnSegmentClick<Option<RemindLead>> = {
                let entity = entity.clone();
                Rc::new(move |option, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut DocumentTypesDialog| {
                            dialog.click_remind(option)
                        });
                    });
                })
            };
            let on_destination_field_click: crate::dialog::OnClick = {
                let entity = entity.clone();
                Rc::new(move |_window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut DocumentTypesDialog| {
                            dialog.click_destination_field()
                        });
                    });
                })
            };
            let on_destination_option_click: crate::form::select_field::OnOptionClick = {
                let entity = entity.clone();
                Rc::new(move |index, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut DocumentTypesDialog| {
                            dialog.click_destination_option(index)
                        });
                    });
                })
            };
            DocumentTypesDialogHandlers {
                on_field_click,
                on_tracks_click,
                on_remind_click,
                on_destination_field_click,
                on_destination_option_click,
                on_cancel: on_settings_dialog_cancel.clone(),
                on_confirm: on_settings_dialog_confirm.clone(),
            }
        };
        // The default Budget's current month: the rail badge and the Dashboard's budget list.
        let default_budget_figures = self.budgets(cx).default_budget().map(|budget| {
            budgets::period_figures(
                budget,
                &self.budgets_ledger(cx),
                Period::of(self.today),
                self.today,
            )
        });
        let inventory_dialog_handlers = {
            // A click that runs `change` on the open Inventory dialog.
            let edit = |change: fn(&mut InventoryDialog)| {
                let entity = entity.clone();
                let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                    entity.update(cx, |shell, cx| shell.edit_dialog(cx, change));
                });
                on_click
            };
            // As `edit`, for a click on an option of one of its selects.
            let choose = |change: fn(&mut InventoryDialog, usize)| {
                let entity = entity.clone();
                let on_click: crate::form::select_field::OnOptionClick =
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.edit_dialog(cx, |dialog: &mut InventoryDialog| {
                                change(dialog, index)
                            });
                        });
                    });
                on_click
            };
            let on_field_click: crate::settings::inventory::dialogs::location::OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut InventoryDialog| {
                            dialog.click_field(field)
                        });
                    });
                })
            };
            InventoryDialogHandlers {
                on_field_click,
                on_unit_field_click: edit(InventoryDialog::click_unit_field),
                on_unit_option_click: choose(InventoryDialog::click_unit_option),
                // The Area form has one field, already focused: a click only repaints.
                on_area_name_click: edit(|_| {}),
                on_destination_field_click: edit(InventoryDialog::click_destination_field),
                on_destination_option_click: choose(InventoryDialog::click_destination_option),
                on_cancel: on_settings_dialog_cancel.clone(),
                on_confirm: on_settings_dialog_confirm.clone(),
            }
        };

        // The active View's own element, told whether the View zone has focus before the status
        // line reads its page status.
        let active_element = active_entity.element(
            focus == FocusZone::View,
            self.nav.ledger_open(),
            window_size,
            cx,
        );
        let view_chrome = self.view_chrome(cx);
        let active_view = self.active_view(cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(color::background(cx))
            .text_color(color::foreground(cx))
            .font_family(type_scale::FONT_FAMILY)
            .text_size(type_scale::BODY)
            .track_focus(&self.focus_handle)
            .group(documents_view::drop_overlay::GROUP)
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, cx| {
                this.drop_documents(paths.paths(), cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.handle_key_down(event, WindowSize::of(window), cx) {
                    cx.notify();
                }
                this.run_pending_effects(cx);
            }))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .opacity(content_opacity)
                    .child(TopBar::new(
                        on_rail_toggle,
                        on_topbar_action,
                        self.nav.can_go_back(),
                        self.nav.can_go_forward(),
                        self.chrome.undo_menu_open,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.0))
                            .flex()
                            .child(
                                PrimaryRail::new(
                                    self.nav.primary_highlight(),
                                    focus == FocusZone::PrimaryRail,
                                    rail_mode,
                                    self.chrome.collapsed_rail_tooltip,
                                    on_row_hover,
                                    on_row_click,
                                )
                                .bill_attention(
                                    bills::attention_entries(
                                        self.bill_plans(cx),
                                        self.bill_entries(cx),
                                        self.today,
                                    )
                                    .len(),
                                )
                                .budget_over(
                                    default_budget_figures
                                        .as_ref()
                                        .map_or(0, |figures| figures.over_count),
                                )
                                .documents_inbox(documents::inbox_count(self.documents(cx))),
                            )
                            .when(
                                self.nav.noun().has_context_entities() && self.nav.ledger_open(),
                                |this| {
                                    this.child(ContextRail::new(
                                        self.nav.noun(),
                                        self.nav.context(),
                                        focus == FocusZone::ContextRail,
                                    ))
                                },
                            )
                            .child(match active_entity {
                                ActiveViewEntity::Shell(view) => {
                                    render_view(view, focus == FocusZone::View, cx)
                                }
                                _ => view_frame(
                                    active_element,
                                    active_entity.min_width(),
                                    &self.view_x_scroll_handle,
                                    focus == FocusZone::View,
                                    cx.listener(|shell, _: &MouseDownEvent, _window, cx| {
                                        shell.nav.set_focus(FocusZone::View);
                                        cx.notify();
                                    }),
                                    cx,
                                ),
                            }),
                    ),
            )
            .child(
                StatusLine::new(
                    self.nav.mode(),
                    self.chrome.status_message.clone(),
                    self.command_echo(),
                )
                .toast_echo(
                    self.chrome
                        .toasts
                        .echo()
                        .filter(|_| !toast_history_open)
                        .map(|toast| (toast.kind(), toast.text().to_string())),
                )
                .page(view_chrome.status)
                .mode_label(
                    active_view
                        .mode_label()
                        .filter(|_| self.nav.mode() == InputMode::Normal),
                )
                .on_hint(on_hint),
            )
            .children(
                self.chrome
                    .palette
                    .as_ref()
                    .map(|palette| palette.render(window.viewport_size().width, cx)),
            )
            .children(self.file_explorer.as_ref().map(|explorer| {
                explorer.render(
                    on_explorer_entry_click,
                    on_explorer_breadcrumb_click,
                    on_explorer_filter_toggle,
                    on_explorer_cancel,
                    on_explorer_open,
                    cx,
                )
            }))
            .children(
                self.dialog::<HelpDialog>()
                    .map(|dialog| help_view::render(on_help_close, &dialog.shortcuts, cx)),
            )
            .children(toast_history_open.then(|| {
                toast_history_view::render(self.chrome.toasts.history(), on_toast_history_close, cx)
            }))
            .children(self.render_documents_dialog(&entity, cx))
            .child(documents_view::drop_overlay::render(cx))
            .children(self.render_bills_dialog(&entity, cx))
            .children(self.render_budgets_detail(&entity, cx))
            .children(self.render_budgets_limit_dialog(&entity, cx))
            .children(self.render_budgets_fill_dialog(&entity, cx))
            .children(self.render_budgets_switcher(&entity, rail_mode, cx))
            .children(self.render_budgets_form_dialog(&entity, cx))
            .children(self.render_budgets_manage_dialog(&entity, cx))
            .children(self.dialog::<PayeesDialog>().and_then(|dialog| {
                dialog.render(
                    self.ledger.read(cx).ledger().payees(),
                    self.transactions(cx),
                    self.ledger.read(cx).ledger().accounts(),
                    payees_dialog_handlers,
                    cx,
                )
            }))
            .children(self.dialog::<DocumentTypesDialog>().and_then(|dialog| {
                dialog.render(
                    self.ledger.read(cx).ledger().document_types(),
                    documents_dialog_handlers,
                    cx,
                )
            }))
            .children(self.dialog::<InventoryDialog>().and_then(|dialog| {
                dialog.render(
                    self.ledger.read(cx).ledger().inventory(),
                    self.documents(cx),
                    inventory_dialog_handlers,
                    cx,
                )
            }))
            .children(
                self.chrome
                    .dialog
                    .as_ref()
                    .and_then(|dialog| dialog.render(&self.dialog_scope(), &dialog_actions, cx)),
            )
            .children(self.dialog::<TracingDialog>().map(|dialog| {
                dialog.render(
                    on_settings_dialog_cancel.clone(),
                    on_settings_dialog_confirm.clone(),
                    cx,
                )
            }))
            // Last: above every dialog, the palette and their scrim, never over the status line.
            .children(toast_layer)
    }
}

/// The active noun's own view interior, for a View `Shell` still draws: the placeholder Nouns. The
/// pane is focus-bordered, a property of the `View` zone itself, and does not scroll: a placeholder
/// has no content to move through.
fn render_view(view: ShellView, focused: bool, cx: &gpui::App) -> gpui::AnyElement {
    let content = match view {
        ShellView::Placeholder(noun) => placeholder::render(noun, cx),
    };

    div()
        .id("view")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
        .child(content)
        .into_any_element()
}

/// The focus-bordered frame a View that renders itself sits in (ADR-0037): the View draws its own
/// scrolling page, and the border says whether the View zone has focus, which is `Shell`'s. A
/// click anywhere in the View gives the View zone focus (`on_click_focus`), so a View never sets
/// `Shell`'s focus itself.
///
/// A View with a `min_width` lays out at least that wide, and scrolls sideways inside the frame,
/// over a horizontal scroll bar along its bottom edge, when the frame is narrower (ADR-0042).
fn view_frame(
    view: gpui::AnyElement,
    min_width: Option<gpui::Pixels>,
    x_scroll_handle: &ScrollHandle,
    focused: bool,
    on_click_focus: impl Fn(&MouseDownEvent, &mut Window, &mut gpui::App) + 'static,
    cx: &gpui::App,
) -> gpui::AnyElement {
    let frame = div()
        .id("view-frame")
        .on_mouse_down(gpui::MouseButton::Left, on_click_focus)
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        });
    let Some(min_width) = min_width else {
        return frame.child(view).into_any_element();
    };
    frame
        .relative()
        .child(
            div()
                .id("view-x-scroll")
                .debug_selector(|| "view-x-scroll".to_string())
                .size_full()
                .flex()
                .overflow_x_scroll()
                .track_scroll(x_scroll_handle)
                .child(
                    div()
                        .debug_selector(|| "view-content".to_string())
                        .flex_1()
                        .min_w(min_width)
                        .h_full()
                        .flex()
                        .child(view),
                ),
        )
        .child(
            div()
                .debug_selector(|| "view-x-scrollbar".to_string())
                // Not occluding: the bar draws only while the View overflows, and the strip must
                // not block clicks on the View's own bottom edge the rest of the time.
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .h(px(12.0))
                .child(
                    Scrollbar::horizontal(x_scroll_handle).scrollbar_show(ScrollbarShow::Always),
                ),
        )
        .into_any_element()
}
