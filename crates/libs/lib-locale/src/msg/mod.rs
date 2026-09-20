//! Typed accessors for the shared Messages, generated at build time from `i18n/<locale>/*.ftl`.
//!
//! Each Message and attribute is a free function (`nav_accounts()`, `accounts_delete_title(count)`)
//! taking `i64` for a plural count and `&str` for anything else. Ids are kebab-case
//! `<area>-<thing>-<role>`; the accessor is the same name in snake_case, and an attribute joins
//! with an underscore (`accounts-delete.title` gives `accounts_delete_title`).
//!
//! [`LAYER`] holds the embedded Catalogue files, which the loader composes with any bin layer.

#![allow(clippy::all, missing_docs)]

include!(concat!(env!("OUT_DIR"), "/msg.rs"));
