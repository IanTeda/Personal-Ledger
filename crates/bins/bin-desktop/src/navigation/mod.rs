//! Navigation and the keyboard grammar: the Desktop's names for where the person is (`nav`; the
//! state machine is `lib-shell`'s, held by the Shell's Presenter), the palette's
//! command registry (`command`), the movement a key makes through the focused zone (`movement`),
//! a keystroke as a keymap names it (`chord`) and the file explorer (`explorer`). The keys
//! themselves are `lib-shell`'s global keymap, and their order is `crate::input`'s.

pub mod active_view;
pub(crate) mod chord;
pub(crate) mod command;
pub(crate) mod explorer;
pub mod movement;
pub mod nav;
