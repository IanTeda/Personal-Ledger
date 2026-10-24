//! The Desktop's Shell (ADR-0045): the always-present frame around the View on show. `router` is
//! the root Router, the [`router::Root`] the window renders, with its `impl Root` blocks per
//! concern; `view` the frame it draws (the top bar, the rails, the status line, the command
//! palette, the Dialog host and the Toast layer); `state` what of the frame is the bin's rather
//! than `lib-shell`'s Presenter's. Infrastructure and the View interiors are not the Shell.

pub mod router;
pub(crate) mod state;
pub(crate) mod view;
