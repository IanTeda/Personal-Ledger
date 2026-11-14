//! The one Router every module page lends its Presenter (#1127): [`PageRouter<P>`] holds the
//! page's Setup and this call's gpui context, queues a Notice once for every page, and opens a
//! Dialog with the page's landing, so a module writes only its `<Domain>Router` impl on
//! `PageRouter<'_, '_, XPresenter>`. The plumbing around it (a Router struct, a `Notify` impl, a
//! Setup copied into a Routes struct) was the same in every module, so it is written here once.
//!
//! [`PageView::with_router`] can't be a default: coercing `&mut PageRouter<Self>` to the
//! Presenter's `dyn XRouter` in generic code needs the unstable `Unsize`. So it stays required,
//! and a page writes [`page_router!`] inside its `impl PageView`, where `Self` is concrete and the
//! coercion compiles. A page that reads its Presenter before `handle` (Bills' and Budgets' date
//! style, the Documents Dialogs' ports) writes its own `with_router` around
//! [`PageRouter::lending`], lending what it read as `L`.

use gpui::{App, Context};
use lib_toast::{Notice, Notify};

use super::dialog_entity::{DialogView, ModuleDialog};
use super::module_page::{ModulePage, PageView};
use crate::notices::Queue;

/// What a module's `<Domain>Setup::new` builds: the page's Presenter, which owns its keymap, and
/// the Setup its Router is built from, which [`ModulePage::new`] hosts together.
pub type Built<P> = (P, <P as PageView>::Setup);

/// A page's Router for one call: its Setup, the call's context, through which a Dialog opens in
/// the host, and what the page lent it from its Presenter (`()` for most).
pub struct PageRouter<'a, 'b, P: PageView, L = ()> {
    pub setup: &'a P::Setup,
    pub cx: &'a mut Context<'b, ModulePage<P>>,
    pub lent: L,
}

impl<'a, 'b, P: PageView> PageRouter<'a, 'b, P> {
    /// The Router over `setup` for the call `cx` belongs to, lent nothing.
    pub fn new(setup: &'a P::Setup, cx: &'a mut Context<'b, ModulePage<P>>) -> Self {
        Self::lending(setup, cx, ())
    }
}

impl<'a, 'b, P: PageView, L> PageRouter<'a, 'b, P, L> {
    /// The Router over `setup` for the call `cx` belongs to, lent `lent`, which the page read from
    /// its Presenter before the handle borrows it.
    pub fn lending(setup: &'a P::Setup, cx: &'a mut Context<'b, ModulePage<P>>, lent: L) -> Self {
        Self { setup, cx, lent }
    }

    /// Where a Dialog the page opens hands its confirmed outcome: [`ModulePage::landing`].
    pub fn landing(&self) -> impl Fn(P::Outcome, &mut App) + 'static {
        ModulePage::<P>::landing(self.cx)
    }

    /// Hosts `dialog` in the Dialog host, its confirmed outcome landing on the page.
    pub fn open<D: DialogView<Outcome = P::Outcome>>(&mut self, dialog: D) {
        let landed = self.landing();
        ModuleDialog::open(self.cx, dialog, landed);
    }
}

impl<P: PageView, L> Notify for PageRouter<'_, '_, P, L> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// A page's [`PageView::with_router`], lending it a [`PageRouter`] over its Setup. Written inside
/// the `impl PageView`, where `Self` is concrete, so the Router coerces to the Presenter's
/// `dyn XRouter`.
macro_rules! page_router {
    () => {
        fn with_router<R>(
            presenter: &mut Self,
            setup: &<Self as $crate::shell::view::module_page::PageView>::Setup,
            cx: &mut ::gpui::Context<'_, $crate::shell::view::module_page::ModulePage<Self>>,
            handle: impl FnOnce(
                &mut Self,
                &mut <Self as ::lib_viper::presenter::Presenter>::Router<'_>,
            ) -> R,
        ) -> R {
            handle(
                presenter,
                &mut $crate::shell::view::page_router::PageRouter::new(setup, cx),
            )
        }
    };
}
pub(crate) use page_router;
