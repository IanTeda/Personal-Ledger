//! The Shell's view: the frame the Desktop draws around the View on show -- the top bar
//! (`topbar`), the rails (`rail`), the status line (`statusline`), the command palette
//! (`palette`), the Dialog host and the generic Dialog Entity (`dialog_entity`), the generic page
//! Entity (`module_page`), the one hosting trait every View and Settings page is reached through
//! (`page_handle`), the page clicks (`page_clicks`) and the Toast layer with its session history
//! (`toast`).

pub(crate) mod dialog_entity;
pub(crate) mod module_page;
pub(crate) mod no_ledger;
pub(crate) mod page_clicks;
pub(crate) mod page_handle;
pub(crate) mod palette;
pub(crate) mod rail;
pub(crate) mod statusline;
pub(crate) mod text_fit;
pub(crate) mod toast;
pub(crate) mod topbar;
pub(crate) mod view_status;
pub mod window_size;
