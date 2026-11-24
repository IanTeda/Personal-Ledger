//! Where each Open Ledger gets its price transport (ADR-0062): `main` installs how to build one,
//! `lib-price-feed`'s over its Tokio runtime, and a test a fake. Each Ledger builds its own, so a
//! response for a closed Ledger's request never reaches the next one. With none installed, nothing
//! is fetched and `f` is offered nowhere.

use std::rc::Rc;

pub use lib_price_feed::Feed;

/// How each Open Ledger builds its [`Feed`]; `None` when it can't.
pub type Feeds = Rc<dyn Fn() -> Option<Feed>>;

/// The feed the TUI fetches through: `lib-price-feed`'s on `runtime`, `main`'s Tokio runtime.
pub fn http(runtime: tokio::runtime::Handle) -> Feeds {
    Rc::new(move || lib_price_feed::open(runtime.clone()))
}
