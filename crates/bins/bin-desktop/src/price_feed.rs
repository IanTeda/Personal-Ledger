//! Where each Open Ledger gets its price transport (ADR-0062): the composition root installs how
//! to build one, `lib-price-feed`'s over the app's Tokio runtime, and a UI test a fake. Each Ledger
//! builds its own, so a response for a closed Ledger's request never reaches the next one.
//! With none installed, nothing is fetched and `f` is offered nowhere.

use std::rc::Rc;

use gpui::{App, Global};

pub use lib_price_feed::Feed;

/// How each Open Ledger builds its [`Feed`]; `None` when it can't.
struct Feeds(Rc<dyn Fn() -> Option<Feed>>);

impl Global for Feeds {}

/// Builds every Ledger opened from now on a feed from `feeds`.
pub fn install(feeds: impl Fn() -> Option<Feed> + 'static, cx: &mut App) {
    cx.set_global(Feeds(Rc::new(feeds)));
}

/// A new feed for a Ledger being opened, if one is installed and builds.
pub(crate) fn feed(cx: &App) -> Option<Feed> {
    let feeds = cx.try_global::<Feeds>()?.0.clone();
    feeds()
}

/// The feed the app fetches through: `lib-price-feed`'s on `runtime`, `main`'s Tokio runtime.
pub(crate) fn http(runtime: tokio::runtime::Handle) -> impl Fn() -> Option<Feed> {
    move || lib_price_feed::open(runtime.clone())
}
