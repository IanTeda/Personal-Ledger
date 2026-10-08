//! Navigation and the keyboard grammar: the pure state `Shell` applies (`nav`), the palette's
//! command registry (`command`), the key router (`key_router`) and the file explorer
//! (`explorer`).

pub(crate) mod command;
pub(crate) mod explorer;
#[doc(hidden)]
pub mod key_router;
pub mod nav;
