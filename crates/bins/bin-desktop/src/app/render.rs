//! `Render for Shell`, the page props and the View switch of `Shell`. An `impl Shell` block for the
//! concern, not a real module (`view/` holds the Views themselves).

use super::Shell;
use crate::categories::form::CategoriesDialog;
use crate::payees::form::PayeesDialog;
use crate::tags::form::TagsDialog;
use std::rc::Rc;

use crate::view::settings::inventory::{self as inventory_view, InventoryRow};
use crate::view::{institutions as institutions_view, units as units_view};

use gpui::{
    Context, ExternalPaths, KeyDownEvent, MouseDownEvent, ScrollHandle, SharedString, Window, div,
    prelude::*, px,
};

use crate::{
    accounts::form::AccountsDialog,
    bills::{self},
    budgets,
    categories::{self},
    chrome::rail::{self as chrome_rail, context::ContextRail, primary::PrimaryRail},
    chrome::statusline::{self, StatusLine},
    chrome::toast::history as toast_history_view,
    chrome::topbar::{self, TopBar},
    documents::{self},
    navigation::explorer::{self as explorer_view},
    navigation::nav::{FocusZone, InputMode},
    payees::{self},
    period::Period,
    settings::{SettingsDialog, SettingsFocus},
    tags::{self},
    theme::{color, type_scale},
    view::format,
    view::{
        accounts as accounts_view,
        active::{ActiveViewEntity, ShellView},
        bills as bills_view, categories as categories_view,
        dashboard::{self, Dashboard},
        documents::{self as documents_view},
        event::Reveal,
        help as help_view, payees as payees_view, placeholder, settings as settings_view,
        tags as tags_view,
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
        // Shared by the Add unit (issue #184) and Edit unit (issue #185) dialogs -- both wrap
        // the same `UnitForm`, and `Shell`'s own handlers already dispatch on whichever
        // `SettingsDialog` variant is actually open, so one set of closures serves both renders.
        let on_unit_dialog_field_click: units_view::add_dialog::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_unit_dialog_field_click(field, cx)
                });
            })
        };
        let on_unit_dialog_kind_click: units_view::add_dialog::OnKindClick = {
            let entity = entity.clone();
            Rc::new(move |kind, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_unit_dialog_kind_click(kind, cx)
                });
            })
        };
        let on_settings_dialog_cancel: units_view::add_dialog::OnCancel = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_dialog_cancel(cx));
            })
        };
        let on_settings_dialog_confirm: units_view::add_dialog::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_dialog_confirm(cx));
            })
        };
        let on_add_institution_account_type_click: institutions_view::add_dialog::OnAccountTypeClick = {
            let entity = entity.clone();
            Rc::new(move |account_type, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_add_institution_account_type_click(account_type, cx)
                });
            })
        };
        let on_add_institution_unit_click: institutions_view::add_dialog::OnUnitClick = {
            let entity = entity.clone();
            Rc::new(move |code, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_add_institution_unit_click(code, cx)
                });
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

        let on_accounts_add_click: accounts_view::OnAddClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_add_click(cx));
            })
        };
        let on_accounts_row_click: accounts_view::OnAccountClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_row_click(id, cx));
            })
        };
        let on_accounts_edit_click: accounts_view::OnAccountClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_edit_click(id, cx));
            })
        };
        let on_accounts_delete_click: accounts_view::OnAccountClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_delete_click(id, cx));
            })
        };
        let on_accounts_dialog_field_click: accounts_view::add_dialog::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_accounts_dialog_field_click(field, cx)
                });
            })
        };
        let on_accounts_dialog_option_click: accounts_view::add_dialog::OnOptionClick = {
            let entity = entity.clone();
            Rc::new(move |field, index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_accounts_dialog_option_click(field, index, cx)
                });
            })
        };
        let on_accounts_dialog_cancel: accounts_view::add_dialog::OnCancel = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_dialog_cancel(cx));
            })
        };
        let on_accounts_dialog_confirm: accounts_view::add_dialog::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_dialog_confirm(cx));
            })
        };
        let account_options = self.account_dialog_options(cx);
        let selected_account = self.accounts_view.read(cx).selected_index(cx);
        let accounts_page = accounts_view::AccountsPageProps {
            accounts: self.accounts.read(cx).accounts(cx),
            units: self.units_store.read(cx).units(cx),
            selected: selected_account,
            on_add_click: on_accounts_add_click,
            on_row_click: on_accounts_row_click,
            on_edit_click: on_accounts_edit_click,
            on_delete_click: on_accounts_delete_click,
        };
        let on_categories_add_click: categories_view::OnAddClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_categories_add_click(cx));
            })
        };
        let on_categories_add_sub_click: categories_view::OnAddSubClick = {
            let entity = entity.clone();
            Rc::new(move |parent_id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_add_sub_click(parent_id, cx)
                });
            })
        };
        let on_categories_edit_click: categories_view::OnEditClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_categories_edit_click(id, cx));
            })
        };
        let on_categories_delete_click: categories_view::OnDeleteClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_categories_delete_click(id, cx));
            })
        };
        let on_categories_disclosure_click: categories_view::OnDisclosureClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_disclosure_click(id, cx)
                });
            })
        };
        let payee_click = |handler: fn(&mut Shell, u32, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: settings_view::payees::OnPayeeClick = Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| handler(shell, id, cx));
            });
            on_click
        };
        let on_payees_add_click: crate::dialog::OnClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_payees_add_click(cx));
            })
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
            let form_edit = |change: fn(&mut payees::form::PayeeForm, usize)| {
                let entity = entity.clone();
                let on_click: payees_view::add_dialog::OnOptionClick =
                    Rc::new(move |index, _window, cx| {
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
            let on_field_click: payees_view::add_dialog::OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.edit_dialog(cx, |dialog: &mut PayeesDialog| {
                            if let Some(form) = dialog.form_mut() {
                                if field == payees::form::PayeeField::DefaultCategory {
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
                        form.focus(payees::form::PayeeField::Rule);
                        form.add_rule();
                    }
                });
            };
            payees_view::add_dialog::PayeeDialogHandlers {
                on_field_click,
                on_option_click: form_edit(payees::form::PayeeForm::choose_category),
                on_add_rule: plain(add_rule),
                on_remove_rule: form_edit(payees::form::PayeeForm::remove_rule),
                on_cancel: plain(Shell::handle_dialog_cancel),
                on_confirm: plain(Shell::handle_dialog_confirm),
            }
        };
        let settings_payees_page = settings_view::payees::PayeesPageProps {
            payees: self.payees_list(cx),
            categories: self.categories(cx),
            selected: self.settings_payees_selected_id(cx),
            on_add_click: on_payees_add_click,
            on_row_click: payee_click(Shell::handle_settings_payees_row_click),
            on_edit_click: payee_click(Shell::handle_payees_edit_click),
            on_delete_click: payee_click(Shell::handle_payees_delete_click),
        };
        let document_type_click = |handler: fn(&mut Shell, u32, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: settings_view::documents::OnTypeClick =
                Rc::new(move |id, _window, cx| {
                    entity.update(cx, |shell, cx| handler(shell, id, cx));
                });
            on_click
        };
        let settings_documents_page = settings_view::documents::DocumentsPageProps {
            types: self.document_types(cx),
            selected: self.settings_documents_selected_id(cx),
            on_add_click: {
                let entity = entity.clone();
                Rc::new(move |_window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_settings_documents_add_click(cx)
                    });
                })
            },
            on_row_click: document_type_click(Shell::handle_settings_documents_row_click),
            on_edit_click: document_type_click(Shell::handle_settings_documents_edit_click),
            on_remove_click: document_type_click(Shell::handle_settings_documents_remove_click),
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
        let tag_click = |handler: fn(&mut Shell, u32, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: tags_view::OnTagClick = Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| handler(shell, id, cx));
            });
            on_click
        };
        let tag_plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: tags_view::OnPlainClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let tags_dialog_handlers = {
            let on_field_click: tags_view::add_dialog::OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_tags_dialog_field_click(field, cx);
                    });
                })
            };
            let on_pick: tags_view::colour_field::OnPick = {
                let entity = entity.clone();
                Rc::new(move |index, _window, cx| {
                    entity.update(cx, |shell, cx| shell.handle_tags_dialog_pick(index, cx));
                })
            };
            tags_view::add_dialog::TagDialogHandlers {
                on_field_click,
                on_pick,
                on_cancel: tag_plain(Shell::handle_dialog_cancel),
                on_confirm: tag_plain(Shell::handle_dialog_confirm),
            }
        };
        let tag_groups = tags::duplicate_groups(self.tags_list(cx), self.transactions(cx));
        let settings_tags_page = settings_view::tags::TagsPageProps {
            tags: self.tags_list(cx),
            selected: self.settings_tags_selected_id(cx),
            groups: &tag_groups,
            on_add_click: tag_plain(Shell::handle_tags_add_click),
            on_row_click: tag_click(Shell::handle_settings_tags_row_click),
            on_merge_click: tag_click(Shell::handle_tags_duplicate_click),
            on_edit_click: tag_click(Shell::handle_tags_edit_click),
            on_remove_click: tag_click(Shell::handle_tags_remove_click),
        };
        let bills_base_unit = self
            .units_store
            .read(cx)
            .units(cx)
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
                            &plan.unit,
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
        let on_settings_categories_row_click: categories_view::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_settings_categories_row_click(id, cx)
                });
            })
        };
        let settings_categories_page = settings_view::categories::CategoriesPageProps {
            categories: self.categories(cx),
            expanded: self.categories_view.read(cx).expanded(),
            selected: self.categories_view.read(cx).selected_id(),
            on_add_click: on_categories_add_click.clone(),
            on_add_sub_click: on_categories_add_sub_click.clone(),
            on_edit_click: on_categories_edit_click.clone(),
            on_delete_click: on_categories_delete_click.clone(),
            on_disclosure_click: on_categories_disclosure_click.clone(),
            on_row_click: on_settings_categories_row_click,
        };

        // Categories dialog closures
        let on_categories_dialog_field_click: categories_view::add_dialog::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_field_click(field, cx);
                });
            })
        };
        let on_categories_dialog_parent_change: categories_view::add_dialog::OnParentChange = {
            let entity = entity.clone();
            Rc::new(move |parent_id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_parent_change(parent_id, cx);
                });
            })
        };
        let on_categories_dialog_type_change: categories_view::add_dialog::OnTypeChange = {
            let entity = entity.clone();
            Rc::new(move |category_type, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_type_change(category_type, cx);
                });
            })
        };
        let on_categories_dialog_cancel: categories_view::add_dialog::OnCancel = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_dialog_cancel(cx);
                });
            })
        };
        let on_categories_dialog_confirm: categories_view::add_dialog::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_dialog_confirm(cx);
                });
            })
        };

        // The Settings pages with Ledger data are still drawn here, and handed to the Settings
        // View for this frame.
        let settings_data_page = {
            let settings = self.views.settings.read(cx);
            settings_view::data_page(
                settings.selected_section(),
                focus == FocusZone::View && settings.focus() == SettingsFocus::Page,
                &settings_view::DataPageProps {
                    accounts: settings_view::accounts::AccountsPageProps {
                        accounts: accounts_page.accounts,
                        units: accounts_page.units,
                        selected: accounts_page.selected,
                        on_add_click: accounts_page.on_add_click,
                        on_row_click: accounts_page.on_row_click,
                        on_edit_click: accounts_page.on_edit_click,
                        on_delete_click: accounts_page.on_delete_click,
                    },
                    categories: settings_categories_page,
                    tags: settings_tags_page,
                    payees: settings_payees_page,
                    documents: settings_documents_page,
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
                AccountsDialog::Add(form) => accounts_view::add_dialog::render(
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
                        .accounts
                        .read(cx)
                        .accounts(cx)
                        .iter()
                        .find(|account| account.id == *id)
                    {
                        Some(account) => accounts_view::edit_dialog::render(
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
                        .accounts
                        .read(cx)
                        .accounts(cx)
                        .iter()
                        .find(|account| account.id == *id)
                    {
                        Some(account) => accounts_view::delete_dialog::render(
                            account,
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
            .children(match self.dialog::<PayeesDialog>() {
                Some(PayeesDialog::Add(form)) => Some(payees_view::add_dialog::render(
                    payees_view::add_dialog::PayeeDialogMode::Add,
                    form,
                    form.options(),
                    form.name_error(),
                    form.is_valid(),
                    payees_dialog_handlers,
                    cx,
                )),
                Some(PayeesDialog::Edit(id, form)) => {
                    payees::get(self.payees_list(cx), *id).map(|payee| {
                        payees_view::add_dialog::render(
                            payees_view::add_dialog::PayeeDialogMode::Edit {
                                name: &payee.name,
                                splits: payees::usage(
                                    self.transactions(cx),
                                    self.accounts.read(cx).accounts(cx),
                                    None,
                                    *id,
                                )
                                .splits,
                            },
                            form,
                            form.options(),
                            form.name_error(),
                            form.is_valid(),
                            payees_dialog_handlers,
                            cx,
                        )
                    })
                }
                Some(PayeesDialog::Delete(id, form)) => {
                    self.delete_payee_target(cx).map(|(payee, action)| {
                        payees_view::delete_dialog::render(
                            payees_view::delete_dialog::DeletePayeeProps {
                                payee,
                                action,
                                form,
                                splits: payees::usage(
                                    self.transactions(cx),
                                    self.accounts.read(cx).accounts(cx),
                                    None,
                                    *id,
                                )
                                .splits,
                                on_cancel: plain_payees(Shell::handle_dialog_cancel),
                                on_confirm: plain_payees(Shell::handle_dialog_confirm),
                            },
                            cx,
                        )
                    })
                }
                None => None,
            })
            .children(self.render_document_types_dialog(&entity, cx))
            .children(self.render_inventory_dialog(&entity, cx))
            .children(match self.dialog::<TagsDialog>() {
                Some(TagsDialog::Add(form)) => Some(tags_view::add_dialog::render(
                    form,
                    form.name_error(),
                    form.is_valid(),
                    tags_dialog_handlers,
                    cx,
                )),
                Some(TagsDialog::Edit(id, form)) => tags::get(self.tags_list(cx), *id).map(|tag| {
                    tags_view::edit_dialog::render(
                        tags_view::edit_dialog::EditTagProps {
                            original_name: &tag.name,
                            form,
                            name_error: form.name_error(),
                            valid: form.is_valid(),
                            transactions: tags::transaction_count(self.transactions(cx), *id),
                            on_toggle_active: tag_plain(Shell::handle_tags_dialog_toggle_active),
                        },
                        tags_dialog_handlers,
                        cx,
                    )
                }),
                Some(TagsDialog::Remove(id, form)) => {
                    tags::get(self.tags_list(cx), *id).map(|tag| {
                        tags_view::remove_dialog::render(
                            tags_view::remove_dialog::RemoveTagProps {
                                tag,
                                form,
                                transactions: tags::transaction_count(self.transactions(cx), *id),
                                on_cancel: tag_plain(Shell::handle_dialog_cancel),
                                on_confirm: tag_plain(Shell::handle_dialog_confirm),
                            },
                            cx,
                        )
                    })
                }
                Some(TagsDialog::Merge(form)) => {
                    let source = form.source_id();
                    let entity = entity.clone();
                    let on_field_click: tags_view::merge_dialog::OnFieldClick = {
                        let entity = entity.clone();
                        Rc::new(move |field, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.handle_merge_tags_field_click(field, cx);
                            });
                        })
                    };
                    let on_option_click: tags_view::merge_dialog::OnOptionClick =
                        Rc::new(move |field, index, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.handle_merge_tags_option_click(field, index, cx);
                            });
                        });
                    Some(tags_view::merge_dialog::render(
                        tags_view::merge_dialog::MergeTagsProps {
                            form,
                            source: source.and_then(|id| tags::get(self.tags_list(cx), id)),
                            target: form
                                .target_id()
                                .and_then(|id| tags::get(self.tags_list(cx), id)),
                            transactions: source
                                .map_or(0, |id| tags::transaction_count(self.transactions(cx), id)),
                            on_field_click,
                            on_option_click,
                            on_cancel: tag_plain(Shell::handle_dialog_cancel),
                            on_confirm: tag_plain(Shell::handle_dialog_confirm),
                        },
                        cx,
                    ))
                }
                None => None,
            })
            .children(
                self.dialog::<CategoriesDialog>()
                    .map(|dialog| match dialog {
                        CategoriesDialog::Add { form, .. } => {
                            let parent_options: Vec<_> = self
                                .categories(cx)
                                .iter()
                                .map(|c| {
                                    let depth = categories::depth(self.categories(cx), c.id);
                                    let is_available = depth < 2; // Can't add children to depth-2 categories
                                    categories_view::add_dialog::ParentOption {
                                        id: Some(c.id),
                                        label: categories::path(self.categories(cx), c.id)
                                            .unwrap_or_else(|| c.name.clone()),
                                        is_available,
                                    }
                                })
                                .collect();

                            categories_view::add_dialog::render(
                                form,
                                &parent_options,
                                self.categories(cx),
                                categories_view::DialogHandlers {
                                    on_field_click: on_categories_dialog_field_click,
                                    on_parent_change: on_categories_dialog_parent_change,
                                    on_type_change: on_categories_dialog_type_change,
                                    on_cancel: on_categories_dialog_cancel,
                                    on_confirm: on_categories_dialog_confirm,
                                },
                                cx,
                            )
                        }
                        CategoriesDialog::Edit(category_id, form) => {
                            let category =
                                self.categories(cx).iter().find(|c| c.id == *category_id);
                            let descendants = category
                                .map(|_| {
                                    categories::descendants_inclusive(
                                        self.categories(cx),
                                        *category_id,
                                    )
                                })
                                .unwrap_or_default();

                            let parent_options: Vec<_> = self
                                .categories(cx)
                                .iter()
                                .filter(|c| {
                                    // Exclude the category itself
                                    if c.id == *category_id {
                                        return false;
                                    }
                                    // Exclude descendants (to prevent cycles)
                                    if descendants.contains(&c.id) {
                                        return false;
                                    }
                                    // Check depth: can't be at depth 2 or deeper
                                    let depth = categories::depth(self.categories(cx), c.id);
                                    depth < 2
                                })
                                .map(|c| categories_view::edit_dialog::ParentOption {
                                    id: Some(c.id),
                                    label: categories::path(self.categories(cx), c.id)
                                        .unwrap_or_else(|| c.name.clone()),
                                    is_available: true,
                                })
                                .collect();

                            // Count splits in this category (and descendants if parent)
                            let split_count = categories::descendants_inclusive(
                                self.categories(cx),
                                *category_id,
                            )
                            .iter()
                            .flat_map(|cat_id| {
                                self.transactions(cx).iter().flat_map(move |t| {
                                    t.splits.iter().filter(move |s| s.category_id == *cat_id)
                                })
                            })
                            .count();

                            let is_parent = !categories::is_leaf(self.categories(cx), *category_id);

                            categories_view::edit_dialog::render(
                                *category_id,
                                form,
                                &parent_options,
                                self.categories(cx),
                                split_count,
                                is_parent,
                                categories_view::DialogHandlers {
                                    on_field_click: on_categories_dialog_field_click,
                                    on_parent_change: on_categories_dialog_parent_change,
                                    on_type_change: on_categories_dialog_type_change,
                                    on_cancel: on_categories_dialog_cancel,
                                    on_confirm: on_categories_dialog_confirm,
                                },
                                cx,
                            )
                        }
                        CategoriesDialog::Delete(category_id, form) => {
                            let category = self
                                .categories(cx)
                                .iter()
                                .find(|c| c.id == *category_id)
                                .cloned();

                            if let Some(category) = category {
                                // Count splits in this category (and descendants if parent)
                                let split_count = categories::descendants_inclusive(
                                    self.categories(cx),
                                    *category_id,
                                )
                                .iter()
                                .flat_map(|cat_id| {
                                    self.transactions(cx).iter().flat_map(move |t| {
                                        t.splits.iter().filter(move |s| s.category_id == *cat_id)
                                    })
                                })
                                .count();

                                // Count budgets attached to this category
                                let budget_count = self.budgets(cx).limit_count(*category_id);

                                categories_view::delete_dialog::render(
                                    &category,
                                    form,
                                    split_count,
                                    budget_count,
                                    on_categories_dialog_cancel.clone(),
                                    on_categories_dialog_confirm.clone(),
                                    cx,
                                )
                            } else {
                                div().into_any_element()
                            }
                        }
                    }),
            )
            .children(self.dialog::<SettingsDialog>().map(|dialog| match dialog {
                SettingsDialog::AddUnit(form) => units_view::add_dialog::render(
                    form,
                    on_unit_dialog_field_click.clone(),
                    on_unit_dialog_kind_click.clone(),
                    on_settings_dialog_cancel.clone(),
                    on_settings_dialog_confirm.clone(),
                    cx,
                ),
                SettingsDialog::EditUnit(_, form) => units_view::edit_dialog::render(
                    form,
                    on_unit_dialog_field_click,
                    on_unit_dialog_kind_click,
                    on_settings_dialog_cancel.clone(),
                    on_settings_dialog_confirm.clone(),
                    cx,
                ),
                SettingsDialog::DeleteUnit(index, form) => {
                    match self.units_store.read(cx).unit(cx, *index) {
                        Some(row) => units_view::delete_dialog::render(
                            row,
                            form,
                            on_settings_dialog_cancel.clone(),
                            on_settings_dialog_confirm.clone(),
                            cx,
                        ),
                        // Defensive only: `index` should always be in bounds (it's only ever
                        // set from a real row's own click handler) -- an empty overlay is a
                        // safer failure than panicking mid-render.
                        None => div().into_any_element(),
                    }
                }
                SettingsDialog::ClearLogs => settings_view::clear_logs_dialog::render(
                    on_settings_dialog_cancel.clone(),
                    on_settings_dialog_confirm.clone(),
                    cx,
                ),
                SettingsDialog::AddInstitution(form) => institutions_view::add_dialog::render(
                    form,
                    self.units_store.read(cx).units(cx),
                    on_add_institution_account_type_click,
                    on_add_institution_unit_click,
                    on_settings_dialog_cancel,
                    on_settings_dialog_confirm,
                    cx,
                ),
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
