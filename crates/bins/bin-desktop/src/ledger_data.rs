//! The event the Units, Institutions and Inventory stores emit after a write lands.

/// Emitted by the Units, Institutions and Inventory stores after a write lands, so `Shell` can refresh the
/// Views that read the rows (ADR-0032's View events).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerDataEvent {
    Changed,
}
