//! The `lib-database` adapters: each implements one VIPER module's ports over its tables,
//! holding the rows in memory and saving through the shared [`Writer`](crate::writer::Writer)
//! (ADR-0046).

pub mod units;

/// What the Clients hold each module's ports by: an adapter, or the module's in-memory fakes.
pub mod store;
