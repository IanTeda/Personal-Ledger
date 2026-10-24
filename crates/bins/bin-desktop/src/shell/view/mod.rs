//! The Shell's view: the frame the Desktop draws around the View on show -- the top bar
//! (`topbar`), the rails (`rail`), the status line (`statusline`), the command palette
//! (`palette`), the Dialog host (`dialog_host`) and the Toast layer with its session history
//! (`toast`).

pub(crate) mod dialog_apply;
pub(crate) mod dialog_entity;
pub(crate) mod dialog_host;
pub(crate) mod palette;
pub(crate) mod rail;
pub(crate) mod statusline;
pub(crate) mod text_fit;
pub(crate) mod toast;
pub(crate) mod topbar;
pub(crate) mod view_status;
pub mod window_size;
