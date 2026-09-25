// -- ./generated/mod.rs --

// #![allow(unused_imports)]

// `build.rs` rewrites these files from the `.proto` sources on every build, so formatting them
// would only be undone by the next build and show up as a spurious diff.

#[rustfmt::skip]
#[path = "personal_ledger.utilities.v001.rs"]
pub mod utilities;

#[rustfmt::skip]
#[path = "personal_ledger.sync.v001.rs"]
pub mod sync;
