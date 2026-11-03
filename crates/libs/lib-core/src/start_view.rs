//! # Start view
//!
//! The View a Client lands on as it starts (ADR-0053): Configuration both Clients read from the one
//! `start_view` key, and the one resolver that turns `last` into the View remembered at the last
//! quit. Which nouns a Client has, and which of them need a Ledger, is the Client's own.

/// The Start view setting, spelled in the file and by `:set` as [`StartView::key`] gives.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum StartView {
    /// The Dashboard, the app's home View.
    #[default]
    Dashboard,
    /// The View showing when the Client last quit.
    Last,
    /// Settings, on the This device page the Client remembers.
    Settings,
    /// A noun, kept as the file spells it in lower case, so one only the other Client has, or a
    /// newer release's, survives a run of this one. A Client that doesn't know it lands on the
    /// Dashboard.
    Noun(String),
}

impl StartView {
    /// The spelling for the Dashboard.
    pub const DASHBOARD: &str = "dashboard";
    /// The spelling for the View remembered at the last quit.
    pub const LAST: &str = "last";
    /// The spelling for Settings.
    pub const SETTINGS: &str = "settings";

    /// The value the file and `:set start-view` spell it with.
    pub fn key(&self) -> &str {
        match self {
            Self::Dashboard => Self::DASHBOARD,
            Self::Last => Self::LAST,
            Self::Settings => Self::SETTINGS,
            Self::Noun(noun) => noun,
        }
    }

    /// The Start view `key` spells, ignoring case and surrounding space. Any word that isn't
    /// `dashboard`, `last` or `settings` is a noun; nothing at all is the Dashboard.
    pub fn from_key(key: &str) -> Self {
        let key = key.trim().to_lowercase();
        match key.as_str() {
            "" | Self::DASHBOARD => Self::Dashboard,
            Self::LAST => Self::Last,
            Self::SETTINGS => Self::Settings,
            _ => Self::Noun(key),
        }
    }

    /// Where the Client lands: `last` resolved to `remembered`, the View remembered at the last
    /// quit as [`Self::key`] spells it, or the Dashboard when none was. Never [`Self::Last`].
    pub fn resolve(&self, remembered: Option<&str>) -> Self {
        match self {
            Self::Last => match remembered.map(Self::from_key) {
                Some(Self::Last) | None => Self::Dashboard,
                Some(view) => view,
            },
            view => view.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dashboard_is_the_default() {
        assert_eq!(StartView::default(), StartView::Dashboard);
        assert_eq!(StartView::from_key("  "), StartView::Dashboard);
    }

    #[test]
    fn every_value_round_trips_through_its_key_ignoring_case() {
        for view in [
            StartView::Dashboard,
            StartView::Last,
            StartView::Settings,
            StartView::Noun("units".into()),
        ] {
            assert_eq!(StartView::from_key(view.key()), view);
            assert_eq!(StartView::from_key(&view.key().to_uppercase()), view);
        }
        assert_eq!(
            StartView::from_key(" Bills "),
            StartView::Noun("bills".into())
        );
    }

    #[test]
    fn last_resolves_to_the_remembered_view_or_the_dashboard() {
        assert_eq!(
            StartView::Last.resolve(Some("units")),
            StartView::Noun("units".into())
        );
        assert_eq!(
            StartView::Last.resolve(Some("settings")),
            StartView::Settings
        );
        assert_eq!(StartView::Last.resolve(None), StartView::Dashboard);
        assert_eq!(StartView::Last.resolve(Some("last")), StartView::Dashboard);
    }

    #[test]
    fn any_other_value_ignores_the_remembered_view() {
        assert_eq!(
            StartView::Settings.resolve(Some("units")),
            StartView::Settings
        );
        assert_eq!(
            StartView::Noun("tags".into()).resolve(Some("units")),
            StartView::Noun("tags".into())
        );
    }
}
