//! Settings › About's Presenter: the version line and the "Built with" note. The Client hands it
//! the version, since only the bin's `env!` knows it, and draws the project facts shared with the
//! `?` help overlay itself.

use crate::msg;

/// What the Client hands Settings › About as it builds it.
pub struct AboutSetup {
    /// The Client's version, `env!("CARGO_PKG_VERSION")`.
    pub version: String,
}

/// Settings › About as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutViewModel {
    pub version: String,
    /// The release date, the mockup's own since no build stamps one.
    pub released: String,
    pub built_with: String,
}

/// Settings › About's Presenter. It takes no events.
#[derive(Debug)]
pub struct AboutPresenter {
    version: String,
}

impl AboutPresenter {
    pub fn new(setup: AboutSetup) -> Self {
        Self {
            version: setup.version,
        }
    }

    pub fn view_model(&self) -> AboutViewModel {
        AboutViewModel {
            version: msg::settings_about_version(&self.version),
            released: "(08 September 2026)".to_string(),
            built_with: msg::settings_about_built_with(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_line_names_the_version_the_client_gave() {
        let model = AboutPresenter::new(AboutSetup {
            version: "1.2.3".to_string(),
        })
        .view_model();

        assert_eq!(model.version, "Personal Ledger v1.2.3");
        assert_eq!(model.built_with, "Built with Rust + GPUI + SQLite");
    }
}
