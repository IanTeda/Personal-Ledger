//! `Render for Shell`, the page props and the View switch of `Shell`. An `impl Shell` block for the
//! concern, not a real module (`view/` holds the Views themselves).

use super::Shell;
use crate::form::colour_field::OnPick;
use crate::settings::categories::dialogs::{CategoriesDialog, CategoriesDialogHandlers};
use crate::settings::documents::dialogs::{
    DocumentTypesDialog, DocumentTypesDialogHandlers,
    add_edit::OnFieldClick as DocumentTypeFieldClick,
};
use crate::settings::payees::{
    dialogs::{OnFieldClick, OnOptionClick, PayeeDialogHandlers, PayeesDialog},
    form,
};
use crate::settings::tags::{
    dialogs::{
        TagsDialog, TagsDialogHandlers,
        add::OnFieldClick as TagFieldClick,
        merge::{OnFieldClick as MergeFieldClick, OnOptionClick as MergeOptionClick},
    },
    form::TagField,
};
use lib_documents::types::{RemindLead, TracksDate};
use std::rc::Rc;

use crate::settings as settings_view;
use crate::settings::institutions::dialogs::{InstitutionsDialog, InstitutionsDialogHandlers};
use crate::settings::inventory::{self as inventory_view, InventoryRow};
use crate::settings::units::dialogs::{UnitsDialog, UnitsDialogHandlers};

use gpui::{
    Context, ExternalPaths, KeyDownEvent, MouseDownEvent, ScrollHandle, SharedString, Window, div,
    prelude::*, px,
};

use crate::settings::accounts::dialogs as accounts_dialogs;
use crate::{
    bills::{self},
    budgets,
    chrome::rail::{self as chrome_rail, context::ContextRail, primary::PrimaryRail},
    chrome::statusline::{self, StatusLine},
    chrome::toast::history as toast_history_view,
    chrome::topbar::{self, TopBar},
    documents::{self},
    navigation::explorer::{self as explorer_view},
    navigation::nav::{FocusZone, InputMode},
    period::Period,
    settings::accounts::dialogs::AccountsDialog,
    settings::{SettingsDialog, SettingsFocus},
    theme::{color, type_scale},
    view::format,
    view::{
        active::{ActiveViewEntity, ShellView},
        bills as bills_view,
        dashboard::{self, Dashboard},
        documents::{self as documents_view},
        event::Reveal,
        help as help_view, placeholder,
    },
};

/// A click on the empty state's own `:open`/`:new` text (issue #167): the clicked command's
/// name (`"open"` or `"new"`), looked up in `command::COMMANDS` and run exactly as the palette's
/// own `enter` key would (see [`Shell::handle_empty_state_command_click`]).
type OnEmptyStateCommandClick = Rc<dyn Fn(&'static str, &mut Window, &mut gpui::App)>;

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let focus = self.nav.focus();
        let active_entity = self.active_view_entity(cx);
        let display = self.display(cx);
        // The "1d" spec: "The shell behind the palette drops to 30% opacity" -- the "1e" file
        // explorer reuses the same dimming pattern (Implementation note 10). Applied to the top
        // bar and content row only, not the status line -- that same spec separately describes
        // the status line's own COMMAND-mode content (the live query, "esc close command
        // window"), which stays meaningful precisely because it stays legible; only the
        // navigational chrome the palette/explorer visually floats over goes dim.
        let content_opacity = if self.chrome.palette.is_some()
            || self.file_explorer.is_some()
            || self.nav.mode() == InputMode::Help
        {
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
            Rc::new(move |_event, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_toggle_rail(cx));
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
            Rc::new(move |action, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_hint_click(action, cx));
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
            crate::chrome::toast::render(&self.chrome.toasts, on_toast_dismiss, on_toast_hover, cx)
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
        let on_help_close: help_view::OnClose = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_help_close(cx));
            })
        };
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
        let on_empty_state_command_click: OnEmptyStateCommandClick = {
            let entity = entity.clone();
            Rc::new(move |command_name, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_empty_state_command_click(command_name, cx)
                });
            })
        };

        let on_accounts_dialog_field_click: accounts_dialogs::add::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.edit_dialog(cx, |dialog: &mut AccountsDialog| dialog.click_field(field))
                });
            })
        };
        let on_accounts_dialog_option_click: accounts_dialogs::add::OnOptionClick = {
            let entity = entity.clone();
            Rc::new(move |field, index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.edit_dialog(cx, |dialog: &mut AccountsDialog| {
                        dialog.choose_option(field, index)
                    })
                });
            })
        };
        let on_accounts_dialog_cancel: accounts_dialogs::add::OnCancel = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_dialog_cancel(cx));
            })
        };
        let on_accounts_dialog_confirm: accounts_dialogs::add::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_dialog_confirm(cx));
            })
        };
        let account_options = crate::settings::accounts::dialog_options(&self.ledger, cx);
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
        let inventory_row_click =
            |handler: fn(&mut Shell, InventoryRow, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: inventory_view::OnRowClick = Rc::new(move |row, _window, cx| {
                    entity.update(cx, |shell, cx| handler(shell, row, cx));
                });
                on_click
            };
        let inventory_property_click = |handler: fn(&mut Shell, u32, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: inventory_view::OnPropertyClick = Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| handler(shell, id, cx));
            });
            on_click
        };
        let settings_inventory_page = inventory_view::InventoryPageProps {
            inventory: self.inventory(cx),
            expanded: self.views.settings.read(cx).inventory_expanded(),
            selected: self.settings_inventory_selected_row(cx),
            on_add_property_click: {
                let entity = entity.clone();
                Rc::new(move |_window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_settings_inventory_add_property_click(cx)
                    });
                })
            },
            on_add_room_click: inventory_property_click(
                Shell::handle_settings_inventory_add_room_click,
            ),
            on_toggle_click: inventory_property_click(
                Shell::handle_settings_inventory_toggle_click,
            ),
            on_row_click: inventory_row_click(Shell::handle_settings_inventory_row_click),
            on_edit_click: inventory_row_click(Shell::handle_settings_inventory_edit_click),
            on_remove_click: inventory_row_click(Shell::handle_settings_inventory_remove_click),
        };
        let plain_tags = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let tags_dialog_handlers = {
            let on_field_click: TagFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut TagsDialog| {
                            if let Some(form) = dialog.name_form_mut() {
                                form.focus(field);
                            }
                        });
                    });
                })
            };
            let on_pick: OnPick = {
                let entity = entity.clone();
                Rc::new(move |index, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut TagsDialog| {
                            if let Some(form) = dialog.name_form_mut() {
                                form.focus(TagField::Swatches);
                                form.pick(index);
                            }
                        });
                    });
                })
            };
            let on_toggle_active: crate::dialog::OnClick = {
                let entity = entity.clone();
                Rc::new(move |_window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut TagsDialog| {
                            if let Some(form) = dialog.name_form_mut() {
                                form.toggle_active();
                            }
                        });
                    });
                })
            };
            let on_merge_field_click: MergeFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut TagsDialog| {
                            if let TagsDialog::Merge(form) = dialog {
                                form.toggle(field);
                            }
                        });
                    });
                })
            };
            let on_merge_option_click: MergeOptionClick = {
                let entity = entity.clone();
                Rc::new(move |field, index, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut TagsDialog| {
                            if let TagsDialog::Merge(form) = dialog {
                                form.choose(field, index);
                            }
                        });
                    });
                })
            };
            TagsDialogHandlers {
                on_field_click,
                on_pick,
                on_toggle_active,
                on_merge_field_click,
                on_merge_option_click,
                on_cancel: plain_tags(Shell::handle_dialog_cancel),
                on_confirm: plain_tags(Shell::handle_dialog_confirm),
            }
        };
        let ledger_units = self.ledger.read(cx).ledger().units();
        let bills_base_unit = ledger_units
            .iter()
            .find(|unit| unit.is_base)
            .map(|unit| unit.code.as_str());
        let on_dashboard_bill_click: dashboard::OnBillClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.reveal(Reveal::BillEntry(id), cx);
                    cx.notify();
                });
            })
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
        let dashboard = Dashboard::new(
            bills::attention_entries(self.bill_plans(cx), self.bill_entries(cx), self.today)
                .into_iter()
                .filter_map(|id| {
                    let entry = bills::entry(self.bill_entries(cx), id)?;
                    let plan = bills::get(self.bill_plans(cx), id.plan_id)?;
                    let amount = bills::amount(entry, self.bill_plans(cx), self.transactions(cx))?;
                    Some(dashboard::AttentionBill {
                        id,
                        plan: plan.name.clone(),
                        overdue: bills::status(entry, self.today) == bills::BillStatus::Overdue,
                        due: format::date(id.due, display.date_style),
                        amount: bills_view::schedule::with_unit(
                            format::amount(&amount).1,
                            bills_view::schedule::unit_code(ledger_units, plan.unit),
                            bills_base_unit,
                        ),
                    })
                })
                .collect(),
            format::flag_glyph(display.status_glyphs),
            on_dashboard_bill_click,
        )
        .budgets(
            default_budget_figures
                .as_ref()
                .map(|figures| self.dashboard_budget_list(figures, cx))
                .unwrap_or_default(),
        );
        // The Categories Dialogs' handlers: each change to the open form runs on `Shell`, which
        // holds the open Dialog, and confirm and cancel run the Dialog host's cycle.
        let on_categories_dialog_field_click: crate::settings::categories::dialogs::add::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_field_click(field, cx);
                });
            })
        };
        let on_categories_dialog_parent_change: crate::settings::categories::dialogs::add::OnParentChange = {
            let entity = entity.clone();
            Rc::new(move |parent_id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_parent_change(parent_id, cx);
                });
            })
        };
        let on_categories_dialog_type_change: crate::settings::categories::dialogs::add::OnTypeChange = {
            let entity = entity.clone();
            Rc::new(move |category_type, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_type_change(category_type, cx);
                });
            })
        };
        let on_categories_dialog_cancel: crate::settings::categories::dialogs::add::OnCancel = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_dialog_cancel(cx);
                });
            })
        };
        let on_categories_dialog_confirm: crate::settings::categories::dialogs::add::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_dialog_confirm(cx);
                });
            })
        };
        let categories_dialog_handlers = CategoriesDialogHandlers {
            on_field_click: on_categories_dialog_field_click,
            on_parent_change: on_categories_dialog_parent_change,
            on_type_change: on_categories_dialog_type_change,
            on_cancel: on_categories_dialog_cancel,
            on_confirm: on_categories_dialog_confirm,
        };

        // The Settings pages with Ledger data are still drawn here, and handed to the Settings
        // View for this frame.
        let settings_data_page = {
            let settings = self.views.settings.read(cx);
            settings_view::data_page(
                settings.selected_section(),
                focus == FocusZone::View && settings.focus() == SettingsFocus::Page,
                &settings_view::DataPageProps {
                    inventory: settings_inventory_page,
                },
                cx,
            )
        };
        self.views
            .settings
            .update(cx, |view, _| view.set_data_page(settings_data_page));
        // The active View's own element, told whether the View zone has focus before the status
        // line reads its page status.
        let active_element = active_entity.element(focus == FocusZone::View, cx);
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
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                if this.handle_key_down(event, cx) {
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
                    .child(
                        TopBar::new(on_rail_toggle, view_chrome.title.clone())
                            .context(active_view.context()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.0))
                            .flex()
                            .child(
                                PrimaryRail::new(
                                    self.nav.primary_highlight(),
                                    focus == FocusZone::PrimaryRail,
                                    self.nav.primary_rail(),
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
                                ActiveViewEntity::Shell(view) => render_view(
                                    view,
                                    self.nav.ledger_open(),
                                    focus == FocusZone::View,
                                    &self.view_scroll_handle,
                                    on_empty_state_command_click,
                                    dashboard,
                                    cx,
                                ),
                                _ => view_frame(
                                    active_element,
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
                    .map(|palette| palette.render(cx)),
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
            .children((self.nav.mode() == InputMode::Help).then(|| {
                let mut sheet = self.settings_cheat_sheet(cx);
                sheet.extend(self.documents_cheat_sheet(cx));
                help_view::render(on_help_close, sheet, cx)
            }))
            .children(toast_history_open.then(|| {
                toast_history_view::render(self.chrome.toasts.history(), on_toast_history_close, cx)
            }))
            .children(self.render_documents_dialog(&entity, cx))
            .child(documents_view::drop_overlay::render(cx))
            .children(self.dialog::<AccountsDialog>().map(|dialog| match dialog {
                AccountsDialog::Add(form) => accounts_dialogs::add::render(
                    form,
                    &account_options,
                    on_accounts_dialog_field_click,
                    on_accounts_dialog_option_click,
                    on_accounts_dialog_cancel,
                    on_accounts_dialog_confirm,
                    cx,
                ),
                AccountsDialog::Edit(id, form) => {
                    match self
                        .ledger
                        .read(cx)
                        .ledger()
                        .accounts()
                        .iter()
                        .find(|account| account.id == *id)
                    {
                        Some(account) => accounts_dialogs::edit::render(
                            form,
                            account,
                            &account_options,
                            on_accounts_dialog_field_click,
                            on_accounts_dialog_option_click,
                            on_accounts_dialog_cancel,
                            on_accounts_dialog_confirm,
                            cx,
                        ),
                        // Defensive only: the id comes from a live row when the dialog opens.
                        None => div().into_any_element(),
                    }
                }
                AccountsDialog::Delete(id, form) => {
                    match self
                        .ledger
                        .read(cx)
                        .ledger()
                        .accounts()
                        .iter()
                        .find(|account| account.id == *id)
                    {
                        Some(account) => accounts_dialogs::delete::render(
                            account,
                            self.ledger.read(cx).ledger().unit_code(account.unit),
                            form,
                            on_accounts_dialog_cancel,
                            on_accounts_dialog_confirm,
                            cx,
                        ),
                        // Defensive only: the id comes from a live row when the dialog opens.
                        None => div().into_any_element(),
                    }
                }
            }))
            .children(self.render_bills_dialog(&entity, cx))
            .children(self.render_budgets_detail(&entity, cx))
            .children(self.render_budgets_limit_dialog(&entity, cx))
            .children(self.render_budgets_fill_dialog(&entity, cx))
            .children(self.render_budgets_switcher(&entity, cx))
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
            .children(self.render_inventory_dialog(&entity, cx))
            .children(self.dialog::<TagsDialog>().and_then(|dialog| {
                dialog.render(
                    self.ledger.read(cx).ledger().tags(),
                    self.transactions(cx),
                    tags_dialog_handlers,
                    cx,
                )
            }))
            .children(self.dialog::<CategoriesDialog>().map(|dialog| {
                dialog.render(
                    self.categories(cx),
                    self.transactions(cx),
                    self.budgets(cx),
                    categories_dialog_handlers,
                    cx,
                )
            }))
            .children(self.dialog::<SettingsDialog>().map(|dialog| match dialog {
                SettingsDialog::ClearLogs => settings_view::clear_logs_dialog::render(
                    on_settings_dialog_cancel.clone(),
                    on_settings_dialog_confirm.clone(),
                    cx,
                ),
            }))
            .children(self.dialog::<InstitutionsDialog>().map(|dialog| {
                dialog.render(
                    self.ledger.read(cx).ledger().units(),
                    InstitutionsDialogHandlers {
                        on_change: {
                            let entity = entity.clone();
                            Rc::new(move |change, _window, cx| {
                                entity.update(cx, |shell, cx| {
                                    shell.edit_dialog(cx, |dialog: &mut InstitutionsDialog| {
                                        dialog.apply_change(change)
                                    })
                                });
                            })
                        },
                        on_cancel: on_settings_dialog_cancel.clone(),
                        on_confirm: on_settings_dialog_confirm.clone(),
                    },
                    cx,
                )
            }))
            .children(self.dialog::<UnitsDialog>().map(|dialog| {
                dialog.render(
                    self.ledger.read(cx).ledger().units(),
                    UnitsDialogHandlers {
                        on_change: {
                            let entity = entity.clone();
                            Rc::new(move |change, _window, cx| {
                                entity.update(cx, |shell, cx| {
                                    shell.edit_dialog(cx, |dialog: &mut UnitsDialog| {
                                        dialog.apply_change(change)
                                    })
                                });
                            })
                        },
                        on_cancel: on_settings_dialog_cancel,
                        on_confirm: on_settings_dialog_confirm,
                    },
                    cx,
                )
            }))
            // Last: above every dialog, the palette and their scrim, never over the status line.
            .children(toast_layer)
    }
}

/// The active noun's own view interior, for a View `Shell` still draws: the Dashboard, and the
/// placeholder Nouns. `Dashboard` further branches on `ledger_open`
/// (`docs/ux/desktop-mockups/01-shell/README.md`'s "1a" empty state). The pane is scrollable
/// and focus-bordered, both properties of the `View` zone itself.
fn render_view(
    view: ShellView,
    ledger_open: bool,
    focused: bool,
    scroll_handle: &ScrollHandle,
    on_empty_state_command_click: OnEmptyStateCommandClick,
    dashboard: Dashboard,
    cx: &gpui::App,
) -> gpui::AnyElement {
    let content = match view {
        ShellView::Dashboard if ledger_open => dashboard.into_any_element(),
        ShellView::Dashboard => empty_state(on_empty_state_command_click, cx),
        ShellView::Placeholder(noun) => placeholder::render(noun, cx),
    };

    scrolling_pane(content, focused, scroll_handle, cx)
}

/// The focus-bordered frame a View that renders itself sits in (ADR-0037): the View draws its own
/// scrolling page, and the border says whether the View zone has focus, which is `Shell`'s. A
/// click anywhere in the View gives the View zone focus (`on_click_focus`), so a View never sets
/// `Shell`'s focus itself.
fn view_frame(
    view: gpui::AnyElement,
    focused: bool,
    on_click_focus: impl Fn(&MouseDownEvent, &mut Window, &mut gpui::App) + 'static,
    cx: &gpui::App,
) -> gpui::AnyElement {
    div()
        .id("view-frame")
        .on_mouse_down(gpui::MouseButton::Left, on_click_focus)
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
        .child(view)
        .into_any_element()
}

/// The scrollable, focus-bordered frame every non-Settings View sits in.
fn scrolling_pane(
    content: impl IntoElement,
    focused: bool,
    scroll_handle: &ScrollHandle,
    cx: &gpui::App,
) -> gpui::AnyElement {
    div()
        .id("view")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
        .child(content)
        .into_any_element()
}

/// The "1a" cold-start empty state: "No ledger open" centered in the main pane, `:open`/`:new`
/// named in the body copy (`docs/ux/desktop-mockups/01-shell/README.md`'s "Main pane"
/// bullet). `gpui` 0.2's `Styled` trait has no letter-spacing hook, so the title's `-.01em`
/// tracking from the spec has no equivalent here -- a real, not merely unverified, gap.
///
/// Both command names are real click targets (issue #167), not just copy: clicking one lands
/// in exactly the state running it from the palette would (this shell's own repeated invariant
/// -- rail click, `g`-jump and the palette already all call `NavState::set_noun` identically).
fn empty_state(on_command_click: OnEmptyStateCommandClick, cx: &gpui::App) -> gpui::AnyElement {
    let command = |name: &'static str, on_command_click: OnEmptyStateCommandClick, text: String| {
        div()
            .id(SharedString::from(format!("empty-state-{name}")))
            .cursor_pointer()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::foreground(cx))
            .on_click(move |_event, window, cx| on_command_click(name, window, cx))
            .child(text)
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(14.0))
        .p(px(24.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(26.0))
                .line_height(gpui::relative(1.1))
                .child(crate::msg::desktop_empty_state_title()),
        )
        .child(
            div()
                .max_w(px(380.0))
                .flex()
                .flex_wrap()
                .justify_center()
                .text_align(gpui::TextAlign::Center)
                .text_size(px(13.0))
                .text_color(color::muted(cx))
                .children(
                    crate::msg::desktop_empty_state_hint(":open", ":new")
                        .into_iter()
                        .map(|segment| match segment.tag.as_deref() {
                            Some("open") => command("open", on_command_click.clone(), segment.text)
                                .into_any_element(),
                            Some("new") => command("new", on_command_click.clone(), segment.text)
                                .into_any_element(),
                            _ => div().child(segment.text).into_any_element(),
                        }),
                ),
        )
        .into_any_element()
}
