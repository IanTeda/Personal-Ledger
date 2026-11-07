//! The Institutions Dialogs' Views: each `lib-institutions` Dialog Presenter's [`DialogView`],
//! which the generic `ModuleDialog` hosts in the Dialog host (ADR-0056). Every Dialog command and
//! typed key reaches the Presenter as a `lib-viper` `DialogInput`, which it reads itself, and every
//! click is a Presenter event; each View draws the Presenter's view model, hints included, and
//! nothing else. The Usual unit list's scroll window is the form's widget state.

mod delete_institution;
mod institution_form;
