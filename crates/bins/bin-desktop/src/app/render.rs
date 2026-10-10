//! `Render for Shell`, the page props and the View switch of `Shell`. An `impl Shell` block for the
//! concern, not a real module (`view/` holds the Views themselves).

use super::Shell;
use std::rc::Rc;

use crate::view::settings::inventory::{self as inventory_view, InventoryRow};
use crate::view::{institutions as institutions_view, units as units_view};

use gpui::{
    Context, ExternalPaths, KeyDownEvent, ScrollHandle, SharedString, Window, div, prelude::*, px,
};

use lib_core::DateStyle;

use crate::{
    accounts::form::AccountsDialog,
    bills::{self},
    budgets,
    categories::{self},
    chrome::rail::{
        self as chrome_rail,
        context::ContextRail,
        primary::PrimaryRail,
        settings_index::{self, SettingsIndexRail},
    },
    chrome::statusline::{self, StatusLine},
    chrome::toast::history as toast_history_view,
    chrome::topbar::{self, TopBar},
    documents::{self},
    import::{self},
    institutions::InstitutionRow,
    navigation::active_view::ActiveView,
    navigation::explorer::{self as explorer_view},
    navigation::nav::{FocusZone, InputMode, Noun},
    payees::{self},
    period::Period,
    settings::{
        SettingsDialog, SettingsFocus, SettingsSection,
        display::{RowDensity, StatusGlyphs},
    },
    tags::{self},
    theme::colours::ColourChange,
    theme::{color, type_scale},
    transactions::{self},
    units::{PriceSourceRow, UnitRow},
    view::format,
    view::{
        accounts as accounts_view, bills as bills_view,
        budgets::{self as budgets_view},
        categories as categories_view,
        dashboard::{self, Dashboard},
        documents::{self as documents_view},
        help as help_view, import as import_view, payees as payees_view,
        settings::{self as settings_view, SettingsBodyProps},
        tags as tags_view, transactions as transactions_view,
    },
};

/// A click on the empty state's own `:open`/`:new` text (issue #167): the clicked command's
/// name (`"open"` or `"new"`), looked up in `command::COMMANDS` and run exactly as the palette's
/// own `enter` key would (see [`Shell::handle_empty_state_command_click`]).
type OnEmptyStateCommandClick = Rc<dyn Fn(&'static str, &mut Window, &mut gpui::App)>;

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        // 6e shows only on Transactions: leaving the page (a jump, a rail click) abandons it.
        if self.nav.noun() != Noun::Transactions && self.import_state(cx).is_some() {
            self.edit_import(cx, |import| *import = None);
        }
        let focus = self.nav.focus();
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
                entity.update(cx, |shell, cx| shell.handle_settings_dialog_cancel(cx));
            })
        };
        let on_settings_dialog_confirm: units_view::add_dialog::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_settings_dialog_confirm(cx));
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
        let on_settings_index_click: settings_index::OnEntryClick = {
            let entity = entity.clone();
            Rc::new(move |section, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_settings_index_click(section, cx)
                });
            })
        };
        let on_price_source_test_click: settings_view::units::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_price_source_test_click(index, cx)
                });
            })
        };
        let on_price_source_edit_click: settings_view::units::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_price_source_edit_click(index, cx)
                });
            })
        };
        let on_price_source_delete_click: settings_view::units::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_price_source_delete_click(index, cx)
                });
            })
        };
        let on_add_price_source_click: settings_view::units::OnAddClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_add_price_source_click(cx));
            })
        };
        let on_unit_edit_click: settings_view::units::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_unit_edit_click(index, cx));
            })
        };
        let on_unit_delete_click: settings_view::units::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_unit_delete_click(index, cx));
            })
        };
        let on_add_unit_click: settings_view::units::OnAddClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_add_unit_click(cx));
            })
        };
        let on_institution_edit_click: settings_view::institutions::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_institution_edit_click(index, cx)
                });
            })
        };
        let on_institution_delete_click: settings_view::institutions::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_institution_delete_click(index, cx)
                });
            })
        };
        let on_add_institution_click: settings_view::institutions::OnAddClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_add_institution_click(cx));
            })
        };
        let on_sync_now_click: settings_view::sync_server::OnSyncNowClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_sync_now_click(cx));
            })
        };
        let on_backup_now_click: settings_view::data_backup::OnBackupNowClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_backup_now_click(cx));
            })
        };
        let on_export_ledger_click: settings_view::data_backup::OnExportLedgerClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_export_ledger_click(cx));
            })
        };
        let on_tracing_level_click: settings_view::tracing::OnLevelClick = {
            let entity = entity.clone();
            Rc::new(move |level, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_tracing_level_click(level, cx));
            })
        };
        let on_clear_logs_click: settings_view::tracing::OnClearLogsClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_clear_logs_click(cx));
            })
        };
        let on_date_style_click: settings_view::display::OnDateStyleClick = {
            let entity = entity.clone();
            Rc::new(move |style, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_date_style_click(style, cx));
            })
        };
        let on_row_density_click: settings_view::display::OnRowDensityClick = {
            let entity = entity.clone();
            Rc::new(move |density, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_row_density_click(density, cx));
            })
        };
        // A click selects the card and clears any keyboard focus left on the grid.
        let on_colour_theme_click: settings_view::colour_theme::OnColourThemeClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.colour_theme_focus = None;
                    cx.notify();
                });
                ColourChange::Theme(id).apply(cx);
            })
        };
        let on_status_glyphs_click: settings_view::display::OnStatusGlyphsClick = {
            let entity = entity.clone();
            Rc::new(move |glyphs, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_status_glyphs_click(glyphs, cx));
            })
        };

        let on_toasts_click: settings_view::display::OnToastsClick = {
            let entity = entity.clone();
            Rc::new(move |on, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.set_toasts_on(on);
                    cx.notify();
                });
            })
        };

        let on_start_sidebar_minimised_click: settings_view::display::OnPlainClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_start_sidebar_minimised_click(cx)
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
                entity.update(cx, |shell, cx| shell.handle_accounts_dialog_cancel(cx));
            })
        };
        let on_accounts_dialog_confirm: accounts_view::add_dialog::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_dialog_confirm(cx));
            })
        };
        let account_options = self.account_dialog_options();
        let selected_account = self.accounts_view.read(cx).selected_index(cx);
        let accounts_page = accounts_view::AccountsPageProps {
            accounts: self.accounts.read(cx).accounts(),
            units: &self.settings_units,
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
        let bills_dialog_element = self.bills_dialog().and_then(|dialog| {
            let (editing, form) = match dialog {
                bills::BillsDialog::Add(form) => (None, form),
                bills::BillsDialog::Edit(id, form) => (
                    Some(bills::get(self.bill_plans(cx), *id)?.name.as_str()),
                    form,
                ),
                bills::BillsDialog::Pay(form) => {
                    return self.render_pay_bill_dialog(form, &entity, cx);
                }
                bills::BillsDialog::Skip(entry) => {
                    return self.render_skip_bill_dialog(*entry, &entity, cx);
                }
            };
            let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                    entity.update(cx, handler);
                });
                on_click
            };
            let on_field_click: bills_view::plan_dialog::OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_bill_plan_field_click(field, cx)
                    });
                })
            };
            let on_option_click: bills_view::plan_dialog::OnOptionClick = {
                let entity = entity.clone();
                Rc::new(move |field, index, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_bill_plan_option_click(field, index, cx);
                    });
                })
            };
            let on_amount_kind_click: bills_view::plan_dialog::OnAmountKindClick = {
                let entity = entity.clone();
                Rc::new(move |kind, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_bill_plan_amount_kind_click(kind, cx);
                    });
                })
            };
            Some(bills_view::plan_dialog::render(
                bills_view::plan_dialog::PlanDialogProps {
                    editing,
                    form,
                    options: &form.options,
                    errors: form.errors(),
                    valid: form.draft().is_some(),
                    handlers: bills_view::plan_dialog::PlanDialogHandlers {
                        on_field_click,
                        on_option_click,
                        on_amount_kind_click,
                        on_cancel: plain(Shell::handle_bills_dialog_cancel),
                        on_confirm: plain(Shell::handle_bills_dialog_confirm),
                    },
                },
                cx,
            ))
        });
        let payees_dialog_handlers = {
            let plain = plain_payees;
            let indexed = |handler: fn(&mut Shell, usize, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: payees_view::add_dialog::OnOptionClick =
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| handler(shell, index, cx));
                    });
                on_click
            };
            let on_field_click: payees_view::add_dialog::OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_payees_dialog_field_click(field, cx);
                    });
                })
            };
            payees_view::add_dialog::PayeeDialogHandlers {
                on_field_click,
                on_option_click: indexed(Shell::handle_payees_dialog_option_click),
                on_add_rule: plain(Shell::handle_payees_dialog_add_rule),
                on_remove_rule: indexed(Shell::handle_payees_dialog_remove_rule),
                on_cancel: plain(Shell::handle_payees_dialog_cancel),
                on_confirm: plain(Shell::handle_payees_dialog_confirm),
            }
        };
        let settings_payees_page = settings_view::payees::PayeesPageProps {
            payees: &self.payees,
            categories: &self.categories,
            selected: self.settings_payees_selected_id(),
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
            inventory: &self.inventory,
            expanded: &self.settings_inventory_expanded,
            selected: self.settings_inventory_selected_row(),
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
                on_cancel: tag_plain(Shell::handle_tags_dialog_cancel),
                on_confirm: tag_plain(Shell::handle_tags_dialog_confirm),
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
        let bills_unfiltered = self.bills_unfiltered_rows(cx);
        let bills_rows = self
            .bills_state(cx)
            .filters
            .apply(&bills_unfiltered, self.bill_plans(cx));
        let bills_summary = bills::period_summary(
            &bills_rows,
            self.bill_plans(cx),
            self.bill_entries(cx),
            self.transactions(cx),
        );
        let bills_planner_plans = bills::planner_order(self.bill_plans(cx));
        let bills_base_unit = self
            .settings_units
            .iter()
            .find(|unit| unit.is_base)
            .map(|unit| unit.code.as_str());
        let bills_plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: bills_view::OnPlainClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let bills_indexed = |handler: fn(&mut Shell, usize, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: bills_view::OnRowClick = Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| handler(shell, index, cx));
            });
            on_click
        };
        let on_dashboard_bill_click: dashboard::OnBillClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.open_bill_entry(id, cx);
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
                        due: format::date(id.due, self.settings_date_style),
                        amount: bills_view::schedule::with_unit(
                            format::amount(&amount).1,
                            &plan.unit,
                            bills_base_unit,
                        ),
                    })
                })
                .collect(),
            format::flag_glyph(self.settings_status_glyphs),
            on_dashboard_bill_click,
        )
        .budgets(
            default_budget_figures
                .as_ref()
                .map(|figures| self.dashboard_budget_list(figures))
                .unwrap_or_default(),
        );
        let bills_page = bills_view::BillsPageProps {
            tab: self.bills_state(cx).tab,
            period: self.bills_state(cx).period,
            all: self.bills_state(cx).all,
            schedule: bills_view::schedule::ScheduleProps {
                rows: &bills_rows,
                total: bills_unfiltered.len(),
                all: self.bills_state(cx).all,
                filters: bills_view::filters::FilterProps {
                    filters: &self.bills_state(cx).filters,
                    selects: bills_view::filters::FilterField::ORDER
                        .into_iter()
                        .map(|field| {
                            let focused = self
                                .bills_state(cx)
                                .filter_focus
                                .as_ref()
                                .filter(|(focused, _)| *focused == field);
                            bills_view::filters::FilterSelect {
                                field,
                                options: self.bills_filter_options(field, cx).0,
                                state: focused.map_or_else(
                                    || self.bills_filter_select_state(field, cx),
                                    |(_, state)| state.clone(),
                                ),
                                focused: focused.is_some(),
                            }
                        })
                        .collect(),
                    stats: self
                        .bills_state(cx)
                        .filters
                        .plan_id
                        .and_then(|id| bills::get(self.bill_plans(cx), id))
                        .map(|plan| {
                            let stats = bills::history::plan_stats(
                                plan,
                                self.bill_plans(cx),
                                self.bill_entries(cx),
                                self.transactions(cx),
                                self.today,
                            );
                            (plan, stats)
                        }),
                    base_unit: bills_base_unit,
                    on_chip_click: bills_indexed(Shell::handle_bills_filter_chip_click),
                    on_field_click: {
                        let entity = entity.clone();
                        Rc::new(move |field, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.handle_bills_filter_field_click(field, cx);
                            });
                        })
                    },
                    on_option_click: {
                        let entity = entity.clone();
                        Rc::new(move |field, index, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.handle_bills_filter_option_click(field, index, cx);
                            });
                        })
                    },
                },
                summary: &bills_summary,
                plans: self.bill_plans(cx),
                entries: self.bill_entries(cx),
                accounts: self.accounts.read(cx).accounts(),
                transactions: self.transactions(cx),
                base_unit: bills_base_unit,
                glyphs: self.settings_status_glyphs,
                selected: (!bills_rows.is_empty())
                    .then(|| self.bills_state(cx).selected.min(bills_rows.len() - 1)),
                on_row_click: bills_indexed(Shell::handle_bills_row_click),
                on_pay_click: bills_indexed(Shell::handle_bills_pay_click),
                on_skip_click: bills_indexed(Shell::handle_bills_skip_click),
                on_view_transaction_click: bills_indexed(
                    Shell::handle_bills_view_transaction_click,
                ),
            },
            planner: bills_view::planner::PlannerProps {
                plans: &bills_planner_plans,
                inactive: bills::inactive_count(self.bill_plans(cx)),
                categories: &self.categories,
                accounts: self.accounts.read(cx).accounts(),
                base_unit: bills_base_unit,
                selected: (!bills_planner_plans.is_empty()).then(|| {
                    self.bills_state(cx)
                        .selected
                        .min(bills_planner_plans.len() - 1)
                }),
                on_row_click: bills_indexed(Shell::handle_bills_row_click),
                on_edit_click: bills_indexed(Shell::handle_bills_edit_plan_click),
            },
            on_add_click: bills_plain(Shell::handle_bills_add_click),
            on_tab_click: {
                let entity = entity.clone();
                Rc::new(move |tab, _window, cx| {
                    entity.update(cx, |shell, cx| shell.handle_bills_tab_click(tab, cx));
                })
            },
            on_period_prev: bills_plain(Shell::handle_bills_period_prev),
            on_period_next: bills_plain(Shell::handle_bills_period_next),
            on_all_click: bills_plain(Shell::handle_bills_all_click),
        };
        let import_categories = self.import_category_options();
        let import_page = self.import_state(cx).map(|state| {
            let entity_for = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: import_view::OnClick = Rc::new(move |_window, cx| {
                    entity.update(cx, handler);
                });
                on_click
            };
            let indexed = |handler: fn(&mut Shell, usize, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: import_view::OnRowClick = Rc::new(move |index, _window, cx| {
                    entity.update(cx, |shell, cx| handler(shell, index, cx));
                });
                on_click
            };
            let on_select_click: import_view::OnSelectClick = {
                let entity = entity.clone();
                Rc::new(move |index, select, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_import_select_click(index, select, cx);
                    });
                })
            };
            import_view::ImportPageProps {
                state,
                payees: &self.payees,
                categories: &import_categories,
                on_row_click: indexed(Shell::handle_import_row_click),
                on_select_click,
                on_option_click: indexed(Shell::handle_import_option_click),
                on_remember_click: entity_for(Shell::handle_import_remember_click),
                on_back_click: entity_for(Shell::handle_import_back_click),
                on_continue_click: entity_for(Shell::handle_import_continue_click),
            }
        });
        let budgets_figures = (self.nav.noun() == Noun::Budgets)
            .then(|| self.budgets_figures(cx))
            .flatten();
        let budgets_plan = (self.nav.noun() == Noun::Budgets
            && self.budgets_state(cx).tab == budgets::BudgetsTab::Plan)
            .then(|| self.budgets_plan_data(cx))
            .flatten();
        let budgets_history = (self.nav.noun() == Noun::Budgets
            && self.budgets_state(cx).tab == budgets::BudgetsTab::History)
            .then(|| self.budgets_history_data(cx))
            .flatten();
        let budgets_page = budgets_figures.as_ref().map(|(budget, figures)| {
            let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: budgets_view::OnPlainClick = Rc::new(move |_window, cx| {
                    entity.update(cx, handler);
                });
                on_click
            };
            budgets_view::BudgetsPageProps {
                name: &budget.name,
                method: budget.method,
                tab: self.budgets_state(cx).tab,
                period: self.budgets_state(cx).period,
                figures,
                plan: budgets_plan
                    .as_ref()
                    .map(|plan| budgets_view::plan::PlanProps {
                        plan,
                        current: Period::of(self.today),
                        cursor: self.budgets_plan_cursor_in(plan, cx),
                        edit: self.budgets_state(cx).plan_edit.as_ref(),
                        on_cell_click: {
                            let entity = entity.clone();
                            Rc::new(move |row, column, _window, cx| {
                                entity.update(cx, |shell, cx| {
                                    shell.handle_budgets_plan_cell_click(row, column, cx);
                                });
                            })
                        },
                    }),
                history: budgets_history.as_ref().map(|history| {
                    budgets_view::history::HistoryProps {
                        history,
                        categories: &self.categories,
                        cursor: self.budgets_history_cursor_in(history, cx),
                        on_cell_click: {
                            let entity = entity.clone();
                            Rc::new(move |row, column, _window, cx| {
                                entity.update(cx, |shell, cx| {
                                    shell.handle_budgets_history_cell_click(row, column, cx);
                                });
                            })
                        },
                    }
                }),
                on_export_click: plain(Shell::handle_budgets_export_click),
                on_range_prev: plain(Shell::handle_budgets_range_prev),
                on_range_next: plain(Shell::handle_budgets_range_next),
                categories: &self.categories,
                selected: (!figures.rows.is_empty())
                    .then(|| self.budgets_state(cx).selected.min(figures.rows.len() - 1)),
                on_tab_click: {
                    let entity = entity.clone();
                    Rc::new(move |tab, _window, cx| {
                        entity.update(cx, |shell, cx| shell.handle_budgets_tab_click(tab, cx));
                    })
                },
                on_title_click: plain(Shell::handle_budgets_title_click),
                on_period_prev: plain(Shell::handle_budgets_period_prev),
                on_period_next: plain(Shell::handle_budgets_period_next),
                on_edit_plan_click: plain(Shell::handle_budgets_edit_plan_click),
                on_add_click: plain(Shell::handle_budgets_add_click),
                fill_label: crate::msg::desktop_budgets_fill_button(
                    &lib_locale::format::format_month(self.budgets_fill_target(cx).month),
                ),
                on_fill_click: plain(Shell::handle_budgets_fill_click),
                on_action_click: {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.handle_budgets_action_click(index, cx);
                        });
                    })
                },
                on_row_click: {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| shell.handle_budgets_row_click(index, cx));
                    })
                },
                on_known_click: plain(Shell::handle_budgets_known_click),
            }
        });
        let on_settings_categories_row_click: categories_view::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_settings_categories_row_click(id, cx)
                });
            })
        };
        let settings_categories_page = settings_view::categories::CategoriesPageProps {
            categories: &self.categories,
            expanded: &self.categories_expanded,
            selected: self.categories_selected_id,
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
                    shell.handle_categories_dialog_cancel(cx);
                });
            })
        };
        let on_categories_dialog_confirm: categories_view::add_dialog::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_confirm(cx);
                });
            })
        };

        let on_transactions_row_click: transactions_view::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_transactions_row_click(index, cx)
                });
            })
        };
        // Built only while the page is showing: formatting every visible row is a pass over the
        // whole filtered set, which no other page needs.
        let on_transactions_add_click: transactions_view::OnPlainClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_transactions_add_click(cx));
            })
        };
        let on_transactions_chip_click: transactions_view::OnChipClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_transactions_chip_click(field, cx)
                });
            })
        };
        let on_transactions_chip_clear: transactions_view::OnChipClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_transactions_chip_clear(field, cx)
                });
            })
        };
        let on_transactions_clear_all: transactions_view::OnPlainClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_transactions_clear_all(cx));
            })
        };
        let on_transactions_search_click: transactions_view::OnPlainClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_transactions_search_click(cx));
            })
        };
        let filter_popover = self
            .transactions_state(cx)
            .filter_form
            .as_ref()
            .map(|form| {
                let entity_for = |shell_call: fn(&mut Shell, &mut Context<'_, Shell>)| {
                    let entity = entity.clone();
                    Rc::new(move |_window: &mut Window, cx: &mut gpui::App| {
                        entity.update(cx, shell_call);
                    }) as Rc<dyn Fn(&mut Window, &mut gpui::App)>
                };
                let on_field_click: transactions_view::OnFieldClick = {
                    let entity = entity.clone();
                    Rc::new(move |field, _window, cx| {
                        entity.update(cx, |shell, cx| shell.handle_filter_field_click(field, cx));
                    })
                };
                let on_option_click: transactions_view::OnOptionClick = {
                    let entity = entity.clone();
                    Rc::new(move |field, index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.handle_filter_option_click(field, index, cx)
                        });
                    })
                };
                let on_status_click: transactions_view::OnStatusClick = {
                    let entity = entity.clone();
                    Rc::new(move |status, _window, cx| {
                        entity.update(cx, |shell, cx| shell.handle_filter_status_click(status, cx));
                    })
                };
                // Anchor just below the chip that opened it, left-aligned to it and kept inside the
                // window; before any chip has been painted, a fixed spot.
                let viewport = window.viewport_size();
                let (left, top) = match self
                    .transactions_state(cx)
                    .chip_bounds
                    .borrow()
                    .get(&self.transactions_state(cx).filter_anchor)
                    .copied()
                {
                    Some(bounds) => {
                        let mut left = bounds.origin.x;
                        let max_left = viewport.width - px(416.0);
                        if left > max_left {
                            left = max_left;
                        }
                        if left < px(8.0) {
                            left = px(8.0);
                        }
                        (left, bounds.origin.y + bounds.size.height + px(6.0))
                    }
                    None => (px(300.0), px(200.0)),
                };
                let options = self.filter_form_options(cx);
                transactions_view::render_popover(
                    transactions_view::PopoverProps {
                        form,
                        options: &options,
                        start_hint: form.start_hint(self.today, self.settings_date_style),
                        end_hint: form.end_hint(self.today, self.settings_date_style),
                        can_apply: form.is_valid(self.today, self.settings_date_style),
                        left,
                        top,
                        on_field_click,
                        on_option_click,
                        on_status_click,
                        on_reset: entity_for(Shell::handle_filter_reset),
                        on_apply: entity_for(Shell::handle_filter_apply),
                        on_cancel: entity_for(Shell::handle_filter_cancel),
                    },
                    cx,
                )
            });
        let transactions_page = (self.nav.noun() == Noun::Transactions).then(|| {
            let ledger = self.transactions_ledger(cx);
            let visible = transactions::query::query(
                &ledger,
                self.transactions(cx),
                &self.transactions_state(cx).filters,
                &self.transactions_state(cx).search,
            );
            let rows =
                transactions::rows::build_rows(&visible, &ledger, &self.transactions_prefs());
            let footer = transactions::chips::footer(
                &visible,
                &self.transactions_state(cx).filters,
                &ledger,
            );
            let chips = transactions::chips::chips(
                &self.transactions_state(cx).filters,
                &ledger,
                self.today,
                self.settings_date_style,
            );
            transactions_view::TransactionsPageProps {
                dimmed: self.transactions_state(cx).filter_form.is_some(),
                header: transactions_view::HeaderProps {
                    count_line: transactions::chips::count_line(self.transactions(cx)),
                    chips,
                    show_clear: !self.transactions_state(cx).filters.is_default(self.today),
                    search: self.transactions_state(cx).search.clone(),
                    searching: self.nav.mode() == InputMode::Search,
                    on_add_click: on_transactions_add_click,
                    on_chip_click: on_transactions_chip_click,
                    on_chip_clear: on_transactions_chip_clear,
                    on_clear_all: on_transactions_clear_all,
                    on_search_click: on_transactions_search_click,
                    chip_bounds: self.transactions_state(cx).chip_bounds.clone(),
                },
                selected: transactions::rows::clamp_selection(
                    self.transactions_state(cx).selected,
                    rows.len(),
                ),
                rows: Rc::new(rows),
                row_height: px(format::row_height_px(self.settings_row_density)),
                scroll: self.transactions_state(cx).scroll.clone(),
                on_row_click: on_transactions_row_click,
                footer,
            }
        });
        let documents_page =
            (self.nav.noun() == Noun::Documents).then(|| self.documents_page_props(&entity, cx));
        let view_chrome = self.view_chrome(
            budgets_figures.as_ref(),
            budgets_plan.as_ref(),
            budgets_history.as_ref(),
            cx,
        );

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
                let chosen = settings_view::colour_theme::chosen_index(cx);
                if this.handle_settings_form_key(&event.keystroke, chosen)
                    || this.handle_settings_tracing_key(&event.keystroke)
                    || this.handle_settings_focus_key(&event.keystroke)
                    || this.handle_colour_theme_grid_key(&event.keystroke)
                    || this.handle_bills_tab_key(&event.keystroke, cx)
                    || this.handle_budgets_tab_key(&event.keystroke, cx)
                    || this.handle_key_down(event, cx)
                {
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
                        TopBar::new(on_rail_toggle, view_chrome.title.clone()).context(
                            if self.nav.noun() == Noun::Settings {
                                Some(self.settings_selected_section.label())
                            } else {
                                self.import_state(cx).map(|_| {
                                    crate::msg::desktop_import_context(import::STATEMENT_FILE)
                                })
                            },
                        ),
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
                            .child(render_view(
                                self.active_view(cx),
                                self.nav.ledger_open(),
                                focus == FocusZone::View,
                                &self.view_scroll_handle,
                                on_empty_state_command_click,
                                PageProps {
                                    dashboard,
                                    accounts: accounts_page,
                                    settings_categories: settings_categories_page,
                                    settings_tags: settings_tags_page,
                                    settings_payees: settings_payees_page,
                                    settings_documents: settings_documents_page,
                                    settings_inventory: settings_inventory_page,
                                    bills: bills_page,
                                    budgets: budgets_page,
                                    import: import_page,
                                    transactions: transactions_page,
                                    documents: documents_page,
                                },
                                SettingsPanelProps {
                                    selected: self.settings_selected_section,
                                    focus: self.settings_focus,
                                    on_index_click: on_settings_index_click,
                                    date_style: self.settings_date_style,
                                    row_density: self.settings_row_density,
                                    status_glyphs: self.settings_status_glyphs,
                                    start_sidebar_minimised: self.settings_start_sidebar_minimised,
                                    on_start_sidebar_minimised_click,
                                    on_date_style_click,
                                    on_row_density_click,
                                    on_status_glyphs_click,
                                    toasts_on: self.chrome.toasts.display().toasts_on,
                                    on_toasts_click,
                                    colour_theme_focus: self.colour_theme_focus,
                                    display_field: self.settings_display_field,
                                    on_colour_theme_click,
                                    units: &self.settings_units,
                                    on_unit_edit_click,
                                    on_unit_delete_click,
                                    on_add_unit_click,
                                    price_sources: &self.settings_price_sources,
                                    on_price_source_test_click,
                                    on_price_source_edit_click,
                                    on_price_source_delete_click,
                                    on_add_price_source_click,
                                    institutions: &self.settings_institutions,
                                    on_institution_edit_click,
                                    on_institution_delete_click,
                                    on_add_institution_click,
                                    on_sync_now_click,
                                    on_backup_now_click,
                                    on_export_ledger_click,
                                    log: settings_view::tracing::LogBoxProps {
                                        level: self.settings_log.level(),
                                        entries: self.settings_log.visible(),
                                        hidden: self.settings_log.hidden_count(),
                                        list: self.settings_log_list.clone(),
                                    },
                                    on_tracing_level_click,
                                    on_clear_logs_click,
                                },
                                cx,
                            )),
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
                    (self.import_state(cx).is_some() && self.nav.mode() == InputMode::Normal)
                        .then(crate::msg::desktop_mode_import),
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
            .children(filter_popover)
            .children((self.nav.mode() == InputMode::Help).then(|| {
                let mut sheet = self.settings_cheat_sheet();
                sheet.extend(self.documents_cheat_sheet(cx));
                help_view::render(on_help_close, sheet, cx)
            }))
            .children(toast_history_open.then(|| {
                toast_history_view::render(self.chrome.toasts.history(), on_toast_history_close, cx)
            }))
            .children(self.render_documents_dialog(&entity, cx))
            .child(documents_view::drop_overlay::render(cx))
            .children(self.accounts_dialog().map(|dialog| match dialog {
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
                        .accounts()
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
                        .accounts()
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
            .children(bills_dialog_element)
            .children(self.render_budgets_detail(&entity, cx))
            .children(self.render_budgets_limit_dialog(&entity, cx))
            .children(self.render_budgets_fill_dialog(&entity, cx))
            .children(self.render_budgets_switcher(&entity, cx))
            .children(self.render_budgets_form_dialog(&entity, cx))
            .children(self.render_budgets_manage_dialog(&entity, cx))
            .children(match self.payees_dialog() {
                Some(payees::form::PayeesDialog::Add(form)) => {
                    Some(payees_view::add_dialog::render(
                        payees_view::add_dialog::PayeeDialogMode::Add,
                        form,
                        form.options(),
                        form.name_error(),
                        form.is_valid(),
                        payees_dialog_handlers,
                        cx,
                    ))
                }
                Some(payees::form::PayeesDialog::Edit(id, form)) => payees::get(&self.payees, *id)
                    .map(|payee| {
                        payees_view::add_dialog::render(
                            payees_view::add_dialog::PayeeDialogMode::Edit {
                                name: &payee.name,
                                splits: payees::usage(
                                    self.transactions(cx),
                                    self.accounts.read(cx).accounts(),
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
                    }),
                Some(payees::form::PayeesDialog::Delete(id, form)) => {
                    self.delete_payee_target().map(|(payee, action)| {
                        payees_view::delete_dialog::render(
                            payees_view::delete_dialog::DeletePayeeProps {
                                payee,
                                action,
                                form,
                                splits: payees::usage(
                                    self.transactions(cx),
                                    self.accounts.read(cx).accounts(),
                                    None,
                                    *id,
                                )
                                .splits,
                                on_cancel: plain_payees(Shell::handle_payees_dialog_cancel),
                                on_confirm: plain_payees(Shell::handle_payees_dialog_confirm),
                            },
                            cx,
                        )
                    })
                }
                None => None,
            })
            .children(self.render_document_types_dialog(&entity, cx))
            .children(self.render_inventory_dialog(&entity, cx))
            .children(match self.tags_dialog() {
                Some(tags::form::TagsDialog::Add(form)) => Some(tags_view::add_dialog::render(
                    form,
                    form.name_error(),
                    form.is_valid(),
                    tags_dialog_handlers,
                    cx,
                )),
                Some(tags::form::TagsDialog::Edit(id, form)) => tags::get(self.tags_list(cx), *id)
                    .map(|tag| {
                        tags_view::edit_dialog::render(
                            tags_view::edit_dialog::EditTagProps {
                                original_name: &tag.name,
                                form,
                                name_error: form.name_error(),
                                valid: form.is_valid(),
                                transactions: tags::transaction_count(self.transactions(cx), *id),
                                on_toggle_active: tag_plain(
                                    Shell::handle_tags_dialog_toggle_active,
                                ),
                            },
                            tags_dialog_handlers,
                            cx,
                        )
                    }),
                Some(tags::form::TagsDialog::Remove(id, form)) => {
                    tags::get(self.tags_list(cx), *id).map(|tag| {
                        tags_view::remove_dialog::render(
                            tags_view::remove_dialog::RemoveTagProps {
                                tag,
                                form,
                                transactions: tags::transaction_count(self.transactions(cx), *id),
                                on_cancel: tag_plain(Shell::handle_tags_dialog_cancel),
                                on_confirm: tag_plain(Shell::handle_tags_dialog_confirm),
                            },
                            cx,
                        )
                    })
                }
                Some(tags::form::TagsDialog::Merge(form)) => {
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
                            on_cancel: tag_plain(Shell::handle_tags_dialog_cancel),
                            on_confirm: tag_plain(Shell::handle_tags_dialog_confirm),
                        },
                        cx,
                    ))
                }
                None => None,
            })
            .children(self.categories_dialog().map(|dialog| match dialog {
                categories::form::CategoriesDialog::Add { form, .. } => {
                    let parent_options: Vec<_> = self
                        .categories
                        .iter()
                        .map(|c| {
                            let depth = categories::depth(&self.categories, c.id);
                            let is_available = depth < 2; // Can't add children to depth-2 categories
                            categories_view::add_dialog::ParentOption {
                                id: Some(c.id),
                                label: categories::path(&self.categories, c.id)
                                    .unwrap_or_else(|| c.name.clone()),
                                is_available,
                            }
                        })
                        .collect();

                    categories_view::add_dialog::render(
                        form,
                        &parent_options,
                        &self.categories,
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
                categories::form::CategoriesDialog::Edit(category_id, form) => {
                    let category = self.categories.iter().find(|c| c.id == *category_id);
                    let descendants = category
                        .map(|_| categories::descendants_inclusive(&self.categories, *category_id))
                        .unwrap_or_default();

                    let parent_options: Vec<_> = self
                        .categories
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
                            let depth = categories::depth(&self.categories, c.id);
                            depth < 2
                        })
                        .map(|c| categories_view::edit_dialog::ParentOption {
                            id: Some(c.id),
                            label: categories::path(&self.categories, c.id)
                                .unwrap_or_else(|| c.name.clone()),
                            is_available: true,
                        })
                        .collect();

                    // Count splits in this category (and descendants if parent)
                    let split_count =
                        categories::descendants_inclusive(&self.categories, *category_id)
                            .iter()
                            .flat_map(|cat_id| {
                                self.transactions(cx).iter().flat_map(move |t| {
                                    t.splits.iter().filter(move |s| s.category_id == *cat_id)
                                })
                            })
                            .count();

                    let is_parent = !categories::is_leaf(&self.categories, *category_id);

                    categories_view::edit_dialog::render(
                        *category_id,
                        form,
                        &parent_options,
                        &self.categories,
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
                categories::form::CategoriesDialog::Delete(category_id, form) => {
                    let category = self
                        .categories
                        .iter()
                        .find(|c| c.id == *category_id)
                        .cloned();

                    if let Some(category) = category {
                        // Count splits in this category (and descendants if parent)
                        let split_count =
                            categories::descendants_inclusive(&self.categories, *category_id)
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
            }))
            .children(self.settings_dialog().map(|dialog| match dialog {
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
                    match self.settings_units.get(*index) {
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
                    &self.settings_units,
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

/// The per-page props `render_view` needs for the pages that own their whole pane: Accounts, and
/// Transactions (built only while it is the active page, hence the `Option`).
struct PageProps<'a> {
    dashboard: Dashboard,
    accounts: accounts_view::AccountsPageProps<'a>,
    /// Settings' own Categories page (2i), mounted only while Settings shows it.
    settings_categories: settings_view::categories::CategoriesPageProps<'a>,
    /// Settings' own Tags page (2j), mounted only while Settings shows it.
    settings_tags: settings_view::tags::TagsPageProps<'a>,
    /// Settings' own Payees page, mounted only while Settings shows it.
    settings_payees: settings_view::payees::PayeesPageProps<'a>,
    /// Settings' own Documents page, mounted only while Settings shows it.
    settings_documents: settings_view::documents::DocumentsPageProps<'a>,
    /// Settings' own Inventory page, mounted only while Settings shows it.
    settings_inventory: inventory_view::InventoryPageProps<'a>,
    bills: bills_view::BillsPageProps<'a>,
    /// `None` when the Budget shown has gone.
    budgets: Option<budgets_view::BudgetsPageProps<'a>>,
    /// `Some` while 6e shows in place of the Transactions page.
    import: Option<import_view::ImportPageProps<'a>>,
    transactions: Option<transactions_view::TransactionsPageProps>,
    /// `Some` while Documents is the noun on show.
    documents: Option<documents_view::DocumentsPageProps>,
}

/// Bundles `render_view`'s Settings-only parameters (keeps the function under Clippy's
/// `too_many_arguments` threshold) -- ignored entirely for every noun besides `Settings`. Covers
/// both the index rail's own state and the body's per-section interactive state
/// (`view::settings::SettingsBodyProps`); `render_view` splits it back apart when it builds
/// each half's own component.
struct SettingsPanelProps<'a> {
    selected: SettingsSection,
    focus: SettingsFocus,
    on_index_click: settings_index::OnEntryClick,
    date_style: Option<DateStyle>,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
    start_sidebar_minimised: bool,
    on_start_sidebar_minimised_click: settings_view::display::OnPlainClick,
    on_date_style_click: settings_view::display::OnDateStyleClick,
    on_row_density_click: settings_view::display::OnRowDensityClick,
    on_status_glyphs_click: settings_view::display::OnStatusGlyphsClick,
    toasts_on: bool,
    on_toasts_click: settings_view::display::OnToastsClick,
    colour_theme_focus: Option<usize>,
    display_field: Option<usize>,
    on_colour_theme_click: settings_view::colour_theme::OnColourThemeClick,
    units: &'a [UnitRow],
    on_unit_edit_click: settings_view::units::OnRowIndexClick,
    on_unit_delete_click: settings_view::units::OnRowIndexClick,
    on_add_unit_click: settings_view::units::OnAddClick,
    price_sources: &'a [PriceSourceRow],
    on_price_source_test_click: settings_view::units::OnRowIndexClick,
    on_price_source_edit_click: settings_view::units::OnRowIndexClick,
    on_price_source_delete_click: settings_view::units::OnRowIndexClick,
    on_add_price_source_click: settings_view::units::OnAddClick,
    institutions: &'a [InstitutionRow],
    on_institution_edit_click: settings_view::institutions::OnRowIndexClick,
    on_institution_delete_click: settings_view::institutions::OnRowIndexClick,
    on_add_institution_click: settings_view::institutions::OnAddClick,
    on_sync_now_click: settings_view::sync_server::OnSyncNowClick,
    on_backup_now_click: settings_view::data_backup::OnBackupNowClick,
    on_export_ledger_click: settings_view::data_backup::OnExportLedgerClick,
    log: settings_view::tracing::LogBoxProps,
    on_tracing_level_click: settings_view::tracing::OnLevelClick,
    on_clear_logs_click: settings_view::tracing::OnClearLogsClick,
}

/// The active noun's own view interior. Only `Dashboard` and `Settings` are real; every other
/// noun is a placeholder until its own view lands (issue #153). `Dashboard` itself further
/// branches on `ledger_open` (`docs/ux/desktop-mockups/01-shell/README.md`'s "1a" empty
/// state) -- implementation note 2's "only the main pane branches on `ledgerOpen`" scopes that
/// to the one real view; the still-placeholder nouns say "not yet built" either way.
///
/// `Settings` (issue #173) is handled separately, before the generic match below: it renders
/// its own two-column [index rail][scrollable body] layout filling the whole slot, rather than
/// the single scrollable `#view` div every other noun gets -- the settings body owns
/// `scroll_handle` directly (see `view::settings::render`), so wrapping the whole thing in a
/// second scrollable container here would fight it for the same scroll state. Every other noun
/// is scrollable and focus-bordered regardless of which is active, since both are properties of
/// the `View` zone itself, not of any one noun's content.
#[expect(
    clippy::too_many_arguments,
    reason = "the view props plus the App the Colour Theme is read from; the remaining sweeps may fold them into one struct"
)]
fn render_view(
    view: ActiveView,
    ledger_open: bool,
    focused: bool,
    scroll_handle: &ScrollHandle,
    on_empty_state_command_click: OnEmptyStateCommandClick,
    pages: PageProps<'_>,
    settings: SettingsPanelProps<'_>,
    cx: &gpui::App,
) -> gpui::AnyElement {
    // Each View's props are `Some` exactly while it is active; the placeholder arms keep a missing
    // one from panicking.
    match view {
        ActiveView::Documents => {
            return match pages.documents {
                Some(documents) => documents_view::render(focused, documents, cx),
                None => scrolling_pane(
                    placeholder_view(Noun::Documents, cx),
                    focused,
                    scroll_handle,
                    cx,
                ),
            };
        }
        ActiveView::Bills => return bills_view::render(focused, scroll_handle, pages.bills, cx),
        ActiveView::Budgets => {
            return match pages.budgets {
                Some(budgets) => budgets_view::render(focused, scroll_handle, budgets, cx),
                None => scrolling_pane(
                    placeholder_view(Noun::Budgets, cx),
                    focused,
                    scroll_handle,
                    cx,
                ),
            };
        }
        ActiveView::Import => {
            if let Some(import) = pages.import {
                return import_view::render(focused, scroll_handle, import, cx);
            }
        }
        ActiveView::Transactions => {
            if let Some(transactions) = pages.transactions {
                return transactions_view::render(focused, transactions, cx);
            }
        }
        ActiveView::Settings(_) | ActiveView::Dashboard | ActiveView::Placeholder(_) => {}
    }

    if let ActiveView::Settings(_) = view {
        return div()
            .id("settings")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .child(SettingsIndexRail::new(
                settings.selected,
                focused && settings.focus == SettingsFocus::Index,
                settings.on_index_click,
            ))
            .child(settings_view::render(
                focused && settings.focus == SettingsFocus::Page,
                scroll_handle,
                SettingsBodyProps {
                    selected: settings.selected,
                    page_focused: focused && settings.focus == SettingsFocus::Page,
                    accounts: settings_view::accounts::AccountsPageProps {
                        accounts: pages.accounts.accounts,
                        units: pages.accounts.units,
                        selected: pages.accounts.selected,
                        on_add_click: pages.accounts.on_add_click,
                        on_row_click: pages.accounts.on_row_click,
                        on_edit_click: pages.accounts.on_edit_click,
                        on_delete_click: pages.accounts.on_delete_click,
                    },
                    categories: pages.settings_categories,
                    tags: pages.settings_tags,
                    payees: pages.settings_payees,
                    documents: pages.settings_documents,
                    inventory: pages.settings_inventory,
                    date_style: settings.date_style,
                    row_density: settings.row_density,
                    status_glyphs: settings.status_glyphs,
                    start_sidebar_minimised: settings.start_sidebar_minimised,
                    on_start_sidebar_minimised_click: settings.on_start_sidebar_minimised_click,
                    on_date_style_click: settings.on_date_style_click,
                    on_row_density_click: settings.on_row_density_click,
                    on_status_glyphs_click: settings.on_status_glyphs_click,
                    toasts_on: settings.toasts_on,
                    on_toasts_click: settings.on_toasts_click,
                    colour_theme_focus: settings.colour_theme_focus,
                    display_field: settings.display_field,
                    on_colour_theme_click: settings.on_colour_theme_click,
                    units: settings.units,
                    on_unit_edit_click: settings.on_unit_edit_click,
                    on_unit_delete_click: settings.on_unit_delete_click,
                    on_add_unit_click: settings.on_add_unit_click,
                    price_sources: settings.price_sources,
                    on_price_source_test_click: settings.on_price_source_test_click,
                    on_price_source_edit_click: settings.on_price_source_edit_click,
                    on_price_source_delete_click: settings.on_price_source_delete_click,
                    on_add_price_source_click: settings.on_add_price_source_click,
                    institutions: settings.institutions,
                    on_institution_edit_click: settings.on_institution_edit_click,
                    on_institution_delete_click: settings.on_institution_delete_click,
                    on_add_institution_click: settings.on_add_institution_click,
                    on_sync_now_click: settings.on_sync_now_click,
                    on_backup_now_click: settings.on_backup_now_click,
                    on_export_ledger_click: settings.on_export_ledger_click,
                    log: settings.log,
                    on_tracing_level_click: settings.on_tracing_level_click,
                    on_clear_logs_click: settings.on_clear_logs_click,
                },
                cx,
            ))
            .into_any_element();
    }

    let content = match view {
        ActiveView::Dashboard if ledger_open => pages.dashboard.into_any_element(),
        ActiveView::Dashboard => empty_state(on_empty_state_command_click, cx),
        ActiveView::Placeholder(noun) => placeholder_view(noun, cx),
        // Import and Transactions land here only without their props, which `Shell` never sends.
        ActiveView::Transactions | ActiveView::Import => placeholder_view(Noun::Transactions, cx),
        ActiveView::Documents
        | ActiveView::Bills
        | ActiveView::Budgets
        | ActiveView::Settings(_) => {
            unreachable!("handled above")
        }
    };

    scrolling_pane(content, focused, scroll_handle, cx)
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

/// The "not yet built" view of a noun whose surface is a later map's work (#420): its name and
/// one line saying so. The rail row and `g` binding are live; the surface is not.
fn placeholder_view(noun: Noun, cx: &gpui::App) -> gpui::AnyElement {
    div()
        .debug_selector(|| "placeholder-view".to_string())
        .p(px(24.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(18.0))
                .child(crate::msg::desktop_placeholder_title(&noun.label())),
        )
        .child(
            div()
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_placeholder_body()),
        )
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
