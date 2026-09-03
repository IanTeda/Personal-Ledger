//! Library-side re-export of `bin-sync-server`'s internals, so integration tests
//! (`tests/*.rs`) can reach modules like [`auth`] that a pure binary crate would
//! otherwise keep private to `main.rs`. `main.rs` stays the actual entry point.

pub mod auth;
