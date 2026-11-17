//! The Tag Removal module's Desktop half (`docs/development/viper-architecture.md`, #1003): the
//! Remove tag and Merge tags Dialogs' Views and the entry points the Tags page's Router opens them
//! through. Every rule, every piece of text and the Presenters are `lib-tag-removal`'s; this folder
//! turns gpui keys and clicks into Presenter events and draws the view models.
//!
//! - `view/` holds each Presenter's `DialogView`, the generic `ModuleDialog` hosting each in the
//!   Dialog host (ADR-0056).
//! - `router.rs` is the module's entry point ([`router::open_remove`], [`router::open_merge`]).
//!
//! Remove and merge write through `lib-tag-removal`'s `TagRemovalPorts`, which the open Ledger hands out whole
//! (#1205): its port writes the Tags and Transactions stores together (#1082), so the
//! rewritten Splits and the Tag's delete join the one pending batch and land in the one commit of
//! the confirm's input.
//! Nothing here reads the `Announcer`: the Tags page hears of a confirm through its `landed`
//! callback and brings what else reads the two domains in line.

pub mod router;
pub(crate) mod view;
