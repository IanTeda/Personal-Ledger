//! The Bills Dialogs' Views: the `lib-bills` Plan form's and Skip's Presenters' `DialogView`s
//! (`dialog/plan_form.rs`, `dialog/skip.rs`), which the generic `ModuleDialog` hosts in the Dialog
//! host (ADR-0056). Every Dialog command and typed key reaches the Presenter as a `lib-viper`
//! `DialogInput`, which it reads itself, and every click is a Presenter event; each View draws the
//! Presenter's view model, hints included, and nothing else.

mod plan_form;
mod skip;
