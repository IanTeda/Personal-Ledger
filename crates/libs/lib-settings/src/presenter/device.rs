//! What the Settings pages share beside the device settings form (`form.rs`): the stock note
//! under a field whose key a source above the file sets.

use crate::msg;

/// The note under a control whose key `source`, a source above the file, sets; `None` for none.
pub(super) fn shadowed_note(source: Option<&str>) -> Option<String> {
    source.map(msg::settings_device_shadowed_note)
}
