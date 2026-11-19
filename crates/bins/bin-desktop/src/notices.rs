//! The bin's one queue of [`Notice`]s: every module Router pushes onto it through [`Queue`], and
//! each input's `send` (`crate::input`) drains it into the Shell's Presenter, which holds the Toast
//! stack and status line, after the Presenter's `handle` and the commit (ADR-0048).
//!
//! The queue is per thread rather than held by `Root`, since today's Dialog host calls a Dialog
//! with no gpui context to reach `Root` through.

use std::cell::RefCell;

use lib_toast::{Notice, Notify};

thread_local! {
    static QUEUE: RefCell<Vec<Notice>> = const { RefCell::new(Vec::new()) };
}

/// A handle on the queue, for a Router to notify through.
#[derive(Debug, Clone, Copy, Default)]
pub struct Queue;

impl Notify for Queue {
    fn notify(&mut self, notice: Notice) {
        QUEUE.with_borrow_mut(|queue| queue.push(notice));
    }
}

/// Every Notice queued since the last drain, oldest first.
pub fn drain() -> Vec<Notice> {
    QUEUE.with_borrow_mut(std::mem::take)
}

/// The labels of the `Notice::Changed`s queued, oldest first, taken off the queue so each input's
/// `send` names its History entry by them, leaving every other Notice queued to show.
pub fn take_changed() -> Vec<String> {
    QUEUE.with_borrow_mut(|queue| {
        let (changed, rest) = std::mem::take(queue)
            .into_iter()
            .partition::<Vec<_>, _>(|notice| matches!(notice, Notice::Changed(_)));
        *queue = rest;
        changed
            .into_iter()
            .filter_map(|notice| match notice {
                Notice::Changed(label) => Some(label),
                _ => None,
            })
            .collect()
    })
}

/// Every Notice queued to show, oldest first, leaving any `Notice::Changed` queued for the running
/// input's `send` to label its History entry with.
pub fn drain_shown() -> Vec<Notice> {
    QUEUE.with_borrow_mut(|queue| {
        let (changed, shown) = std::mem::take(queue)
            .into_iter()
            .partition(|notice| matches!(notice, Notice::Changed(_)));
        *queue = changed;
        shown
    })
}

#[cfg(test)]
mod tests {
    use lib_toast::ToastKind;

    use super::*;

    #[test]
    fn a_drain_hands_back_every_notice_queued_since_the_last_once() {
        drain();
        Queue.notify(Notice::Status("unbound".to_string()));
        Queue.notify(Notice::Toast(ToastKind::Error, "Save failed".to_string()));

        assert_eq!(
            drain(),
            [
                Notice::Status("unbound".to_string()),
                Notice::Toast(ToastKind::Error, "Save failed".to_string()),
            ]
        );
        assert_eq!(drain(), []);
    }

    #[test]
    fn the_changed_labels_come_off_the_queue_apart_from_the_notices_shown() {
        drain();
        Queue.notify(Notice::Status("Saved".to_string()));
        Queue.notify(Notice::Changed("Add payee Netflix".to_string()));

        assert_eq!(drain_shown(), [Notice::Status("Saved".to_string())]);
        Queue.notify(Notice::Toast(ToastKind::Error, "Save failed".to_string()));
        assert_eq!(take_changed(), ["Add payee Netflix".to_string()]);
        assert_eq!(
            drain(),
            [Notice::Toast(ToastKind::Error, "Save failed".to_string())]
        );
    }
}
