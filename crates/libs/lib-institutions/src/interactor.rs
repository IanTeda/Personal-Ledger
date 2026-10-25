//! One Interactor per Institutions intent, each a request value whose `run` takes the port it
//! needs. Every rule is the Entity's and is checked before anything is written, so a refusal
//! leaves the port untouched.

mod add_institution;

pub use add_institution::AddInstitution;
