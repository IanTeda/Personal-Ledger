//! Chrome: the Desktop's always-present visible frame -- the top bar (`topbar`), the rails
//! (`rail`), the status line (`statusline`), the command palette (`palette`), the Dialog host
//! (`dialog_host`) and the Toast layer with its session history (`toast`). Infrastructure and the
//! View interiors are not Chrome.

pub(crate) mod dialog_host;
pub(crate) mod palette;
pub(crate) mod rail;
pub(crate) mod state;
pub(crate) mod statusline;
pub(crate) mod toast;
pub(crate) mod topbar;
