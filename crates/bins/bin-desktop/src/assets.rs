//! Bundled asset source for the desktop shell. `gpui-component`'s own `Icon` element ships
//! no SVG files by default (its README: "you can add any icons you need to your project"),
//! so this registers the app's bundled Lucide icons
//! (`crates/bins/bin-desktop/assets/icons/`, see that directory's own `LICENSE`) as `gpui`'s
//! `AssetSource` -- resolving the icon sourcing question `docs/ux/desktop/README.md` left
//! open.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

/// Embeds one bundled icon at `path` (relative to `assets/icons/`) under the asset path
/// `crate::icon::DesktopIcon::path` hands to `gpui_component::Icon::path`.
macro_rules! icon {
    ($name:literal) => {
        (
            concat!("icons/", $name, ".svg"),
            include_bytes!(concat!("../assets/icons/", $name, ".svg")) as &[u8],
        )
    };
}

/// Every bundled icon. A plain match table rather than `rust-embed`: 14 small SVGs don't need
/// a build-time directory scan.
const ICONS: &[(&str, &[u8])] = &[
    icon!("layout-dashboard"),
    icon!("align-justify"),
    icon!("wallet"),
    icon!("gauge"),
    icon!("trending-up"),
    icon!("tag"),
    icon!("tags"),
    icon!("receipt"),
    icon!("circle-user"),
    icon!("settings"),
    icon!("panel-left"),
    icon!("search"),
    icon!("plus"),
    icon!("flag"),
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| SharedString::from(*name))
            .collect())
    }
}
