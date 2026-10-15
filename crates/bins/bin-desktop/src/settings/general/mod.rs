//! The **General** section (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state", first in
//! scroll order): ledger identity fields plus a "THIS LEDGER" summary panel. Dummy values
//! throughout, matching the mockup's own -- no real `lib_database::Preferences` wiring in this
//! map (see the Desktop Settings Surface map's Destination).

mod render;

pub(crate) use render::render;
