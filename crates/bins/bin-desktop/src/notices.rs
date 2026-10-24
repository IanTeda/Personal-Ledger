//! The bin's one queue of [`Notice`]s: every module Router pushes onto it through [`Queue`], and
//! each input's `send` (`crate::input`) drains it into the Shell's Presenter, which holds the Toast
//! stack and status line, after the Presenter's `handle` and the commit (ADR-0048).
//!
//! The queue is per thread rather than held by `Shell`, since today's Dialog host calls a Dialog
//! with no gpui context to reach `Shell` through.

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
}
