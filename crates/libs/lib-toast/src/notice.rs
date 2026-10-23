use crate::ToastKind;

/// What a module asks the Shell to tell the person: a Toast, or a message on the status line.
/// The text is already a resolved Message. A status-line message is only ever set: the Shell
/// clears it on the next keypress, so no module sends a clear (ADR-0048).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    Toast(ToastKind, String),
    Status(String),
}

/// Where a module's Presenter sends its [`Notice`]s: every module Router declares it as a
/// supertrait, so a new module never re-declares Toast or status methods (ADR-0048).
pub trait Notify {
    fn notify(&mut self, notice: Notice);
}

/// A queue of Notices, for a Router that records them and a Client that drains them later.
impl Notify for Vec<Notice> {
    fn notify(&mut self, notice: Notice) {
        self.push(notice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vec_queues_notices_in_the_order_sent() {
        let mut queue = Vec::new();

        queue.notify(Notice::Status("unbound".to_string()));
        queue.notify(Notice::Toast(ToastKind::Success, "Saved".to_string()));

        assert_eq!(
            queue,
            [
                Notice::Status("unbound".to_string()),
                Notice::Toast(ToastKind::Success, "Saved".to_string()),
            ]
        );
    }
}
