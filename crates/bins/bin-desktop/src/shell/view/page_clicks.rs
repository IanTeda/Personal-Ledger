//! A migrated page's clicks as its Presenter's events (ADR-0037's 2026-11-04 amendment), the page
//! twin of [`ModuleDialogClicks`](super::dialog_entity::ModuleDialogClicks). The page's `render` builds one
//! [`PageClicks`] at its top and hands its render helpers a borrow of it beside the view model, so
//! no page hand-writes a closure builder or a callback alias of its own.
//!
//! Every click goes through the page's own [`PresenterPage::send`], which wraps
//! `crate::input::send`, so a click stays one input with one commit. A page that has gone takes no
//! click.

use std::rc::Rc;

use gpui::{App, Context, Entity, MouseDownEvent, Pixels, Point, WeakEntity, Window};

use crate::dialog::OnClick;
use crate::form::fields::OnSegmentClick;
use crate::form::select_field::OnOptionClick;

/// A page whose clicks are events `E` of a Presenter it holds. A page with two Presenters (a list
/// and a view shown in its place) implements it once for each one's events.
pub trait PresenterPage<E>: Sized + 'static {
    /// Hands `event` to the Presenter as one input, committed once it is handled.
    fn send(&mut self, event: E, cx: &mut Context<'_, Self>);

    /// After a row's click is sent: a page that takes focus from a click on its rows says so here.
    fn row_clicked(&mut self, _cx: &mut Context<'_, Self>) {}

    /// Before a row menu's event is sent: records where the pointer opened it, in window
    /// coordinates. The position is the View's to draw the menu at; the Presenter never sees it.
    fn menu_at(&mut self, _at: Point<Pixels>, _cx: &mut Context<'_, Self>) {}
}

/// Page `P`'s clicks, each sending one of its Presenter events.
pub struct PageClicks<P> {
    page: WeakEntity<P>,
}

impl<P> Clone for PageClicks<P> {
    fn clone(&self) -> Self {
        Self {
            page: self.page.clone(),
        }
    }
}

impl<P: 'static> PageClicks<P> {
    /// The clicks of the page `render`ing in `cx`.
    pub fn new(cx: &Context<'_, P>) -> Self {
        Self {
            page: cx.weak_entity(),
        }
    }

    /// The clicks of `page`, for a part the Settings View draws from outside the page's `render`.
    pub fn of(page: &Entity<P>) -> Self {
        Self {
            page: page.downgrade(),
        }
    }

    /// A handler, for any gpui event `G`, that sends `event`.
    pub fn send<E, G>(&self, event: E) -> impl Fn(&G, &mut Window, &mut App) + 'static
    where
        P: PresenterPage<E>,
        E: Clone + 'static,
    {
        self.send_with(move |_: &G| event.clone())
    }

    /// A handler that sends the event `event` builds from the gpui event `G`.
    pub fn send_with<E, G>(
        &self,
        event: impl Fn(&G) -> E + 'static,
    ) -> impl Fn(&G, &mut Window, &mut App) + 'static
    where
        P: PresenterPage<E>,
    {
        let page = self.page.clone();
        move |gpui_event, _window, cx| {
            let event = event(gpui_event);
            // A page that has gone takes no click.
            let _ = page.update(cx, |page, cx| page.send(event, cx));
        }
    }

    /// A handler for a click on a row that sends `event`, then lets the page take focus
    /// ([`PresenterPage::row_clicked`]).
    pub fn row<E, G>(&self, event: E) -> impl Fn(&G, &mut Window, &mut App) + 'static
    where
        P: PresenterPage<E>,
        E: Clone + 'static,
    {
        let page = self.page.clone();
        move |_, _window, cx| {
            let _ = page.update(cx, |page, cx| {
                page.send(event.clone(), cx);
                page.row_clicked(cx);
            });
        }
    }

    /// A handler for the press that opens a row's menu: records the pointer
    /// ([`PresenterPage::menu_at`]), sends `event`, then lets the page take focus, as a row's click.
    pub fn menu<E>(&self, event: E) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static
    where
        P: PresenterPage<E>,
        E: Clone + 'static,
    {
        let page = self.page.clone();
        move |mouse, _window, cx| {
            let _ = page.update(cx, |page, cx| {
                page.menu_at(mouse.position, cx);
                page.send(event.clone(), cx);
                page.row_clicked(cx);
            });
        }
    }

    /// `event` as a shared widget's click.
    pub fn click<E>(&self, event: E) -> OnClick
    where
        P: PresenterPage<E>,
        E: Clone + 'static,
    {
        let send = self.send::<E, ()>(event);
        Rc::new(move |window, cx| send(&(), window, cx))
    }

    /// A shared segmented control's click, sending the event `event` builds from the value the
    /// clicked option sets.
    pub fn segment<E, T>(&self, event: impl Fn(T) -> E + 'static) -> OnSegmentClick<T>
    where
        P: PresenterPage<E>,
        T: Copy + 'static,
    {
        let send = self.send_with(move |value: &T| event(*value));
        Rc::new(move |value, window, cx| send(&value, window, cx))
    }

    /// A shared select's option click, sending the event `event` builds from the option's index.
    pub fn option<E>(&self, event: impl Fn(usize) -> E + 'static) -> OnOptionClick
    where
        P: PresenterPage<E>,
    {
        let send = self.send_with(move |index: &usize| event(*index));
        Rc::new(move |index, window, cx| send(&index, window, cx))
    }
}
