//! The Documents Entity: the Document and its rules, split by what a change touches. `document` is
//! the model, `library` the Library's index and rows, `suggestion` the Inbox's Suggested Link,
//! `filing` the Inbox's actions, `import` adding and importing, and `seed` the stub Documents.

mod document;
mod filing;
mod import;
mod library;
mod seed;
mod suggestion;
#[cfg(test)]
mod tests;

pub use document::*;
pub use filing::*;
pub use import::*;
pub use library::*;
pub use seed::*;
pub use suggestion::*;
