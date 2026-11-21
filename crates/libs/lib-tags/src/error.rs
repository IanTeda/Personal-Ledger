//! The Tags Refusals (`GLOSSARY.md`): one variant per rule a Tag write enforces, named for what the
//! rule forbids. The `#[error]` text is for logs; the person sees the Presenter's `msg::` Message.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("a tag needs a name")]
    NameRequired,
    /// The name normalises to nothing: it has no letter or digit.
    #[error("a tag name needs a letter or a digit")]
    NoLetterOrDigit,
    /// Another Tag already has this name once normalised; carries that Tag's name.
    #[error("another tag has this name")]
    DuplicateName(String),
    #[error("tag not found")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, Error>;
