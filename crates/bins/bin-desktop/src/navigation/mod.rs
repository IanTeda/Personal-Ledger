//! Navigation and the keyboard grammar: the pure state `Shell` applies (`nav`), the palette's
//! command registry (`command`), the movement a key makes through the focused zone (`movement`),
//! a keystroke as a keymap names it (`chord`) and the file explorer (`explorer`). The keys
//! themselves are `lib-shell`'s global keymap, and their order is `crate::input`'s.

pub mod active_view;
pub(crate) mod chord;
pub(crate) mod command;
pub(crate) mod explorer;
pub mod movement;
pub mod nav;
