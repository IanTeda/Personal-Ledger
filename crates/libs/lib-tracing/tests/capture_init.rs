//! `init` with a capture buffer, through the real global subscriber. Its own test binary, since
//! the global subscriber can only be set once per process.

use lib_tracing::{Levels, LogBuffer, init};

#[test]
fn init_feeds_the_capture_below_the_configured_level() {
    let logs = LogBuffer::new(16);
    let _guard = init(Some(&Levels::WARN), None, Some(logs.clone())).expect("first init");

    tracing::debug!(target: "bin_desktop::shell", "ours at debug");
    tracing::info!(target: "wgpu_core", "dependency info");
    log::warn!(target: "zbus::connection", "bridged warning");

    let messages: Vec<String> = logs.snapshot().iter().map(|e| e.message.clone()).collect();
    assert_eq!(messages, ["ours at debug", "bridged warning"]);
}
