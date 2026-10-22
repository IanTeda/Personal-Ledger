//! The Bills module's Views: the page and its tabs, its Dialogs, and today's Pay Dialog, which the
//! Shell still draws.

pub mod dialog;
mod filters;
pub mod page;
pub(crate) mod pay_dialog;
mod planner;
mod schedule;
pub mod snapshot;

use std::rc::Rc;

use gpui::{App, Window};

pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;
/// Called with a row's position in the tab's rows as its view model lists them.
pub type OnRowClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
