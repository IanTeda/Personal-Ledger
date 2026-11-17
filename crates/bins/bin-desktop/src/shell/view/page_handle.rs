//! [`PageHandle`], the one hosting trait (ADR-0016's and ADR-0056's #1079 update): what a host
//! asks of the page it shows, without knowing which page it is. `Root` reaches the active View
//! through it from one match on the `ActiveView` (`view::active::page`), and the Settings View
//! reaches its open page through it from one match on the `SettingsPage`
//! (`settings::pages::SettingsPages::get`).
//!
//! It is implemented for every [`ModulePage<P>`] and for the Settings View, a composite host that
//! offers each key to its open page through this same trait. What only the Settings body asks of
//! its page (the heading's meta and a side pane) is [`SettingsPageHandle`], a subtrait implemented
//! only for a `ModulePage<P>` (#1129), so `Root` never sees a verb it has no use for and the
//! Settings View answers none it would only stub. Both hosts keep their typed Entities
//! for what one page hands another (a reveal, a month turning): only dispatch is erased. A
//! `[keybindings]` rebind and a Display change reach every page through it too (#1167), so a new
//! page can't be left out of either. A page's frame, `cue(PageCue::Framed(..))`, is the Window Size
//! alone: `Root` tells its active page it on every draw, the Settings View its pages as each is
//! built and as the size changes, and no host frames a page before a key (#1222).

use std::rc::Rc;

use gpui::{AnyElement, AnyView, App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_viper::display::PageDisplay;
use lib_viper::keymap::{KeyBindingConfig, KeyBound, Keymap, Palette};

use super::module_page::{ModulePage, PageCue, PageLayout, PageView};
use super::statusline::PageStatus;
use crate::settings::pages::PageRowClicked;
use crate::view::event::ViewEvent;
use crate::view::help::HelpGroup;

/// What a page made of a key offered to its module keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKey {
    /// Not the page's to take now: the key goes on to the Shell's next step.
    Declined,
    /// The page took it, and it stops there.
    Taken,
    /// The page took it and hands it on to the Shell's fallback, inside the same input
    /// ([`PageView::passes_on`]).
    PassedOn,
}

/// One key a page holds now, as [`PageHandle::keys`] reports it: its command, its live chord
/// (empty while unbound) and whether the palette lists it, which the palette's module Commands are
/// read from (#1169).
#[derive(Debug, Clone)]
pub struct PageBinding {
    pub command: &'static str,
    pub chord: String,
    pub palette: Palette,
}

impl PageBinding {
    /// Every binding of `keymap`, in binding order.
    pub fn all_of<E: Clone>(keymap: &Keymap<E>) -> Vec<Self> {
        keymap.bindings().map(Self::from).collect()
    }
}

impl From<KeyBound<'_>> for PageBinding {
    fn from(bound: KeyBound<'_>) -> Self {
        Self {
            command: bound.command,
            chord: bound.chord.to_string(),
            palette: bound.palette,
        }
    }
}

/// What a host does with one of its page's [`ViewEvent`]s.
type ViewEventSignal = dyn Fn(&ViewEvent, &mut App);

/// Where a hosted page's signals reach its host: its [`ViewEvent`]s, its changes, which repaint
/// the host, and its row clicks. Built over the host's Entity by [`Self::to`], so the trait's
/// `subscribe` names no host type.
#[derive(Clone)]
pub struct PageSignals {
    view_event: Rc<ViewEventSignal>,
    changed: Rc<dyn Fn(&mut App)>,
    row_clicked: Rc<dyn Fn(&mut App)>,
}

impl PageSignals {
    /// Signals reaching the host `cx` is the context of: each [`ViewEvent`] handed to
    /// `view_event`, each row click to `row_clicked`, and each change repainting the host. A host
    /// gone has nothing left to tell.
    pub fn to<H: 'static>(
        cx: &Context<'_, H>,
        view_event: fn(&mut H, &ViewEvent, &mut Context<'_, H>),
        row_clicked: fn(&mut H, &mut Context<'_, H>),
    ) -> Self {
        let host = cx.weak_entity();
        Self {
            view_event: Rc::new({
                let host = host.clone();
                move |event, cx| {
                    let _ = host.update(cx, |host, cx| view_event(host, event, cx));
                }
            }),
            changed: Rc::new({
                let host = host.clone();
                move |cx| {
                    let _ = host.update(cx, |_, cx| cx.notify());
                }
            }),
            row_clicked: Rc::new(move |cx| {
                let _ = host.update(cx, row_clicked);
            }),
        }
    }

    /// Hands `entity`'s [`ViewEvent`]s and changes to the host.
    pub fn follow<T: EventEmitter<ViewEvent>>(
        &self,
        entity: &Entity<T>,
        cx: &mut App,
    ) -> Vec<Subscription> {
        let (view_event, changed) = (self.view_event.clone(), self.changed.clone());
        vec![
            cx.subscribe(entity, move |_, event: &ViewEvent, cx| {
                view_event(event, cx)
            }),
            cx.observe(entity, move |_, cx| changed(cx)),
        ]
    }
}

/// A hosted page behind one `dyn` handle: what a host calls, without knowing which page it shows.
pub trait PageHandle {
    /// A key the page's module keymap takes, by `chord`. Whether it took the key, and whether the
    /// key stops there.
    fn module_key(&self, chord: &str, cx: &mut App) -> PageKey;
    /// A palette Command, by its keymap `command` name, with the `argument` typed after it (empty
    /// for none, and dropped by a binding that takes no Argument). Whether the page took it.
    fn module_command(&self, command: &str, argument: &str, cx: &mut App) -> bool;
    /// A key while the page is in a mode it entered for something of its own (its Search, a
    /// filter popover, an edit in place, Settings' page picker). In Normal mode it takes nothing.
    fn mode_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool;
    /// `Esc`: closes what the page has open. Whether it consumed the key.
    fn escape(&self, cx: &mut App) -> bool;
    /// `/`. `false` when the page has no Search, leaving `/` inert.
    fn start_search(&self, cx: &mut App) -> bool;
    /// What the host tells the page: it has become the destination, the space it is drawn in, a
    /// record handed off to show, whether it has the keyboard. The space and the focus are told
    /// outside any input, which can be every draw.
    fn cue(&self, cue: PageCue, cx: &mut App);
    /// `[keybindings]` changed: the page builds its keys afresh under `keys`.
    fn rebind(&self, keys: &KeyBindingConfig, cx: &mut App);
    /// The Display Preferences changed, or a Ledger opened.
    fn display(&self, display: PageDisplay, cx: &mut App);
    /// Every key the page holds now, each with its chord.
    fn keys(&self, cx: &App) -> Vec<PageBinding>;
    /// The status line's hints and right-hand text for the page's own state; `None` leaves the
    /// Shell's strip and the Ledger file.
    fn page_status(&self, cx: &App) -> Option<PageStatus>;
    /// The page's group of keys for the `?` Help Dialog, `None` for a page that adds none.
    fn help_group(&self, cx: &App) -> Option<HelpGroup>;
    /// How the host lays the page out.
    fn layout(&self) -> PageLayout;
    /// The page's Entity, to draw.
    fn view(&self) -> AnyView;
    /// Hands the page's events, changes and row clicks to its host through `signals`. The host
    /// keeps the subscriptions for as long as it shows the page.
    fn subscribe(&self, signals: &PageSignals, cx: &mut App) -> Vec<Subscription>;
}

impl<P: PageView> PageHandle for Entity<ModulePage<P>> {
    /// Every chord the page's keymap binds, the arrows and `g g` among them.
    fn module_key(&self, chord: &str, cx: &mut App) -> PageKey {
        self.update(cx, |page, cx| page.module_key(chord, cx))
    }

    fn module_command(&self, command: &str, argument: &str, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.module_command(command, argument, cx))
    }

    fn mode_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.mode_key(keystroke, cx))
    }

    /// Closes what the page has open; `false` leaves `Esc` to its host (Settings steps back to the
    /// index).
    fn escape(&self, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.escape(cx))
    }

    fn start_search(&self, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.start_search(cx))
    }

    /// Handed on as `PageView::cued`'s event.
    fn cue(&self, cue: PageCue, cx: &mut App) {
        self.update(cx, |page, cx| page.cued(cue, cx));
    }

    fn rebind(&self, keys: &KeyBindingConfig, cx: &mut App) {
        self.update(cx, |page, cx| {
            page.update_presenter(cx, |presenter| presenter.rebind(keys));
        });
    }

    fn display(&self, display: PageDisplay, cx: &mut App) {
        self.update(cx, |page, cx| {
            page.update_presenter(cx, |presenter| presenter.display(&display));
        });
    }

    /// Every keymap the Presenter holds, whatever state it is in (`PagePresenter::bindings`).
    fn keys(&self, cx: &App) -> Vec<PageBinding> {
        self.read(cx)
            .presenter()
            .bindings()
            .into_iter()
            .map(PageBinding::from)
            .collect()
    }

    /// An empty status (the Dashboard words no hints of its own) leaves the Shell's strip.
    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        let status = self.read(cx).page_status();
        (!status.hints.is_empty() || !status.right.is_empty()).then_some(status)
    }

    fn help_group(&self, cx: &App) -> Option<HelpGroup> {
        self.read(cx).help_group()
    }

    fn layout(&self) -> PageLayout {
        P::layout()
    }

    fn view(&self) -> AnyView {
        self.clone().into()
    }

    fn subscribe(&self, signals: &PageSignals, cx: &mut App) -> Vec<Subscription> {
        let row_clicked = signals.row_clicked.clone();
        let mut subscriptions = signals.follow(self, cx);
        subscriptions.push(cx.subscribe(self, move |_page, _: &PageRowClicked, cx| {
            row_clicked(cx);
        }));
        subscriptions
    }
}

/// A page the Settings View hosts: what its body asks of the open page beyond [`PageHandle`]. Only
/// the Settings body draws a page's heading and docks a pane beside it, so these are not `Root`'s.
pub trait SettingsPageHandle: PageHandle {
    /// The page heading's right-aligned meta; `None` draws the page's scope note.
    fn heading_meta(&self, cx: &App) -> Option<AnyElement>;
    /// A pane drawn at the View's right edge, beside the page.
    fn side_pane(&self, cx: &App) -> Option<AnyElement>;
}

impl<P: PageView> SettingsPageHandle for Entity<ModulePage<P>> {
    fn heading_meta(&self, cx: &App) -> Option<AnyElement> {
        self.read(cx).heading_meta(cx)
    }

    fn side_pane(&self, cx: &App) -> Option<AnyElement> {
        self.read(cx).side_pane(self, cx)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use lib_viper::keymap::KeyBindingConfig;

    use super::PageBinding;
    use crate::navigation::active_view::ActiveView;
    use crate::settings::SettingsPage;

    /// The module pages a Ledger open hosts, each with its page keymap's keys under `config`: what
    /// `Root` hands [`crate::navigation::command::entries`], without building the pages.
    pub(crate) fn hosted_pages(config: &KeyBindingConfig) -> Vec<(ActiveView, Vec<PageBinding>)> {
        vec![
            (
                ActiveView::Settings(SettingsPage::Accounts),
                PageBinding::all_of(&lib_accounts::keymap::accounts_page_keymap(config)),
            ),
            (
                ActiveView::Bills,
                PageBinding::all_of(&lib_bills::keymap::bills_page_keymap(config)),
            ),
            (
                ActiveView::Budgets,
                PageBinding::all_of(&lib_budgets::keymap::budgets_page_keymap(config)),
            ),
            (
                ActiveView::Settings(SettingsPage::Categories),
                PageBinding::all_of(&lib_categories::keymap::categories_page_keymap(config)),
            ),
            (
                ActiveView::Documents,
                PageBinding::all_of(&lib_documents::keymap::documents_page_keymap(config)),
            ),
            (
                ActiveView::Settings(SettingsPage::Documents),
                PageBinding::all_of(&lib_documents::keymap::document_types_page_keymap(config)),
            ),
            (
                ActiveView::Settings(SettingsPage::Institutions),
                PageBinding::all_of(&lib_institutions::keymap::institutions_page_keymap(config)),
            ),
            (
                ActiveView::Settings(SettingsPage::Inventory),
                PageBinding::all_of(&lib_inventory::keymap::inventory_page_keymap(config)),
            ),
            (
                ActiveView::Settings(SettingsPage::Payees),
                PageBinding::all_of(&lib_payees::keymap::payees_page_keymap(config)),
            ),
            (
                ActiveView::Settings(SettingsPage::Tags),
                PageBinding::all_of(&lib_tags::keymap::tags_page_keymap(config)),
            ),
            (
                ActiveView::Transactions,
                PageBinding::all_of(&lib_transactions::keymap::transactions_page_keymap(config)),
            ),
            (
                ActiveView::Settings(SettingsPage::Units),
                PageBinding::all_of(&lib_units::keymap::units_page_keymap(config)),
            ),
        ]
    }
}
