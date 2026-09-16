//! Bundled Lucide icon names, from `docs/ux/desktop/Shell & Navigation/README.md`'s "Assets"
//! table -- resolved through `crate::assets::Assets`, the app's own `gpui::AssetSource`
//! (`gpui-component`'s `Icon` ships no SVGs of its own).

use gpui_component::Icon;

use crate::nav::Noun;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopIcon {
    Dashboard,
    Transactions,
    Accounts,
    Categories,
    Payees,
    Tags,
    Bills,
    Budgets,
    Reports,
    Settings,
    RailToggle,
    Palette,
    AddTransaction,
    Flagged,
    /// TopBar window controls (issue #162) -- minimize-to-taskbar.
    WindowMinimize,
    /// TopBar window controls (issue #162) -- maximize/restore toggle.
    WindowMaximize,
    /// TopBar window controls (issue #162) -- quits the app.
    WindowClose,
}

impl DesktopIcon {
    fn path(self) -> &'static str {
        match self {
            Self::Dashboard => "icons/layout-dashboard.svg",
            Self::Transactions => "icons/align-justify.svg",
            Self::Accounts => "icons/wallet.svg",
            Self::Categories => "icons/tag.svg",
            Self::Payees => "icons/circle-user.svg",
            Self::Tags => "icons/tags.svg",
            Self::Bills => "icons/receipt.svg",
            Self::Budgets => "icons/gauge.svg",
            Self::Reports => "icons/trending-up.svg",
            Self::Settings => "icons/settings.svg",
            Self::RailToggle => "icons/panel-left.svg",
            Self::Palette => "icons/search.svg",
            Self::AddTransaction => "icons/plus.svg",
            Self::Flagged => "icons/flag.svg",
            Self::WindowMinimize => "icons/minus.svg",
            Self::WindowMaximize => "icons/square.svg",
            Self::WindowClose => "icons/x.svg",
        }
    }

    /// A `gpui_component::Icon` ready to size/color and render, e.g.
    /// `icon.icon().text_color(color::INK).with_size(px(14.0))`.
    pub fn icon(self) -> Icon {
        Icon::empty().path(self.path())
    }
}

/// The primary rail's own icon for each noun. `Tags` uses the plural Lucide `tags` glyph,
/// distinct from `Categories`' singular `tag` (already bundled and reused there) -- the two
/// read as siblings without being visually identical.
impl From<Noun> for DesktopIcon {
    fn from(noun: Noun) -> Self {
        match noun {
            Noun::Dashboard => DesktopIcon::Dashboard,
            Noun::Transactions => DesktopIcon::Transactions,
            Noun::Accounts => DesktopIcon::Accounts,
            Noun::Categories => DesktopIcon::Categories,
            Noun::Payees => DesktopIcon::Payees,
            Noun::Tags => DesktopIcon::Tags,
            Noun::Bills => DesktopIcon::Bills,
            Noun::Budgets => DesktopIcon::Budgets,
            Noun::Reports => DesktopIcon::Reports,
            Noun::Settings => DesktopIcon::Settings,
        }
    }
}
