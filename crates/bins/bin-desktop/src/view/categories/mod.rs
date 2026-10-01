//! The Categories dialogs and the row-click contracts for Settings' Categories page (2i, drawn
//! by `view::settings::categories`). The standalone 5a page left the primary rail for Settings
//! (#420).

pub mod add_dialog;
pub mod delete_dialog;
pub mod edit_dialog;

use std::rc::Rc;

use gpui::{App, Window};

pub type OnAddClick = Rc<dyn Fn(&mut Window, &mut App)>;
pub type OnAddSubClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnEditClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnDeleteClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnDisclosureClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnRowClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;

/// The handlers the Add and Edit dialogs both wire, grouped so each `render` stays a short list
/// of what differs between them.
pub struct DialogHandlers {
    pub on_field_click: add_dialog::OnFieldClick,
    pub on_parent_change: add_dialog::OnParentChange,
    pub on_type_change: add_dialog::OnTypeChange,
    pub on_cancel: add_dialog::OnCancel,
    pub on_confirm: add_dialog::OnConfirm,
}
