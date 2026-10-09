//! The Accounts dialogs and the row-click contracts for Settings' Accounts page (2o, drawn by
//! `view::settings::accounts`): the Add, Edit and Delete dialogs, and the props the page takes.
//! The standalone 3a page left the primary rail for Settings (#420).

pub mod add_dialog;
pub mod delete_dialog;
pub mod edit_dialog;
pub(crate) mod hints;
pub(crate) mod select_field;

use std::rc::Rc;

use gpui::{App, Window};

use crate::{accounts::Account, units::UnitRow};

/// Called with an account's [`Account::id`].
pub type OnAccountClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnAddClick = Rc<dyn Fn(&mut Window, &mut App)>;

pub struct AccountsPageProps<'a> {
    pub accounts: &'a [Account],
    /// Settings' live units: the base Unit for the net worth line, and which units are
    /// currencies (every other kind reads as a quantity, `1,240 u`).
    pub units: &'a [UnitRow],
    /// Index into `accounts` of the selected row.
    pub selected: Option<usize>,
    pub on_add_click: OnAddClick,
    pub on_row_click: OnAccountClick,
    pub on_edit_click: OnAccountClick,
    pub on_delete_click: OnAccountClick,
}
