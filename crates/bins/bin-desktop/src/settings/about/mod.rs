//! The Settings › About page (ADR-0037's amendment, one folder per page): static project info.
//! [`AboutPage`] renders itself (`render.rs`) and has no state or events.

mod render;

/// The About page.
#[derive(Debug, Default)]
pub struct AboutPage;
