//! The Add and Edit account Dialog's View: the `lib-accounts` form Presenter's `DialogView`
//! (`account_form.rs`), which the generic `ModuleDialog` hosts in the Dialog host (ADR-0056). Every
//! Dialog command and typed key reaches the Presenter as a `lib-viper` `DialogInput`, which it
//! reads itself, and every click is a Presenter event; the View draws the Presenter's view model,
//! hints included, and nothing else. Delete account is `lib-account-deletion`'s Dialog, opened
//! through the page's Router.

mod account_form;
