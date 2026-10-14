//! The Desktop UI tests as one test binary (ADR-0030). Each View's test file is a module here,
//! so cargo links gpui once for the whole suite instead of once per file.

mod common;

mod accounts_keyboard;
mod accounts_mouse;
mod bills_keyboard;
mod bills_mouse;
mod budgets_keyboard;
mod budgets_mouse;
mod categories_keyboard;
mod categories_mouse;
mod documents_keyboard;
mod documents_mouse;
mod harness;
mod overlays_keyboard;
mod overlays_mouse;
mod palette_keyboard;
mod palette_mouse;
mod placeholders_keyboard;
mod placeholders_mouse;
mod settings_backup_keyboard;
mod settings_backup_mouse;
mod settings_display_keyboard;
mod settings_display_mouse;
mod settings_documents_keyboard;
mod settings_documents_mouse;
mod settings_general_keyboard;
mod settings_general_mouse;
mod settings_institutions_keyboard;
mod settings_institutions_mouse;
mod settings_inventory_keyboard;
mod settings_inventory_mouse;
mod settings_keyboard;
mod settings_mouse;
mod settings_payees_keyboard;
mod settings_payees_mouse;
mod settings_sync_tracing_keyboard;
mod settings_sync_tracing_mouse;
mod settings_tags_keyboard;
mod settings_tags_mouse;
mod settings_tracing_keyboard;
mod settings_tracing_mouse;
mod shell_keyboard;
mod shell_mouse;
mod transactions_keyboard;
mod transactions_mouse;
