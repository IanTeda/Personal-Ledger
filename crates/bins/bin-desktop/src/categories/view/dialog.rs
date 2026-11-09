//! The Add and Edit category Dialog's View: `lib-categories`' form Presenter's [`DialogView`], which
//! the generic `ModuleDialog` hosts in the Dialog host (ADR-0056). Every Dialog command and typed
//! key reaches the Presenter as a `lib-viper` `DialogInput`, which it reads itself, and every click
//! is a Presenter event; the View draws the Presenter's view model, hints included, and nothing
//! else. The Parent category list's scroll window is the form's widget state. The Delete category
//! Dialog is the Category Deletion module's (`crate::category_deletion`, #1005).

mod category_form;
