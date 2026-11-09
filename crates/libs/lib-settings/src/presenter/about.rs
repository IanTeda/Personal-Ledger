//! Settings › About's Presenter: the version line and the "Built with" note. The Client hands it
//! the version, since only the bin's `env!` knows it, and draws the project facts shared with the
//! `?` help overlay itself.

use std::convert::Infallible;

use lib_toast::Notify;
use lib_viper::keymap::{KeyBindingConfig, Keymap};
use lib_viper::presenter::{PagePresenter, Presenter};

use crate::msg;

/// What the Client hands Settings › About as it builds it.
pub struct AboutSetup {
    /// The Client's version, `env!("CARGO_PKG_VERSION")`.
    pub version: String,
}

/// What Settings › About takes: nothing, since it has no key and no button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AboutEvent {}

/// Settings › About as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutViewModel {
    pub version: String,
    /// The release date, the mockup's own since no build stamps one.
    pub released: String,
    pub built_with: String,
}

/// Settings › About's Presenter. It takes no events, so its keymap binds nothing.
#[derive(Debug)]
pub struct AboutPresenter {
    version: String,
    keymap: Keymap<AboutEvent>,
}

impl AboutPresenter {
    pub fn new(setup: AboutSetup) -> Self {
        Self {
            version: setup.version,
            keymap: Keymap::new(&[], &KeyBindingConfig::default()),
        }
    }
}

impl Presenter for AboutPresenter {
    type Event = AboutEvent;
    type ViewModel = AboutViewModel;
    type Router<'r> = dyn Notify + 'r;

    fn handle(&mut self, event: AboutEvent, _router: &mut Self::Router<'_>) {
        match event {}
    }

    fn view_model(&self) -> AboutViewModel {
        AboutViewModel {
            version: msg::settings_about_version(&self.version),
            released: "(08 September 2026)".to_string(),
            built_with: msg::settings_about_built_with(),
        }
    }
}

impl PagePresenter for AboutPresenter {
    /// About opens no Dialog.
    type Outcome = Infallible;

    /// About opens no Dialog, so there is nothing to tell it.
    fn dialog_closed(_outcome: Option<Infallible>) -> Option<AboutEvent> {
        None
    }

    fn keymap(&self) -> &Keymap<AboutEvent> {
        &self.keymap
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

    #[test]
    fn about_binds_no_key() {
        let about = AboutPresenter::new(AboutSetup {
            version: "1.2.3".to_string(),
        });

        assert_eq!(about.keymap().event("enter"), None);
        assert_eq!(about.keymap().listed().count(), 0);
    }
}
