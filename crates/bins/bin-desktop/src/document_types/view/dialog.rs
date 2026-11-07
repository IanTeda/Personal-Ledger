//! The Document Types Dialogs' Views: each `lib-documents` Dialog Presenter's [`DialogView`], which
//! the generic `ModuleDialog` hosts in the Dialog host (ADR-0056). Every Dialog command and typed
//! key reaches the Presenter as a `lib-viper` `DialogInput`, which it reads itself, and every click
//! is a Presenter event; each View draws the Presenter's view model, hints included, and nothing
//! else. The Remove destination list's scroll window is that Dialog's widget state.

mod default_notice;
mod remove_type;
mod type_form;
