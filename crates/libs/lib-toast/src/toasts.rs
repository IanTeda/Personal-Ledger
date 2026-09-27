use std::time::Duration;

use crate::{History, ToastKind};

/// Toasts drawn at once; past this the oldest timed Toast is evicted (#306).
pub const MAX_VISIBLE: usize = 3;

/// What the bin can show, passed in so the model never reads the Preference or the view size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
    /// The Client-scoped Toasts Preference (ADR-0027). Off never silences an Error.
    pub toasts_on: bool,
    /// `false` when the bin cannot draw a Toast at all (a TUI view under 40×8), so the
    /// status-line echo carries every Kind, Errors included.
    pub can_draw: bool,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            toasts_on: true,
            can_draw: true,
        }
    }
}

/// One live Toast, on the stack or in the status-line echo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    // Raise order, bumped on a merge, so "newest" is a plain comparison across stack and echo.
    seq: u64,
    history_id: u64,
    kind: ToastKind,
    text: String,
    count: u32,
    remaining: Option<Duration>,
}

impl Toast {
    pub fn kind(&self) -> ToastKind {
        self.kind
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The `×N` badge; 1 means no badge.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Time left before it expires, or `None` for a sticky Error.
    pub fn remaining(&self) -> Option<Duration> {
        self.remaining
    }

    fn is_timed(&self) -> bool {
        self.remaining.is_some()
    }

    /// Counts one more raise and restarts the timer, as a merged duplicate does.
    fn merge(&mut self, seq: u64) {
        self.seq = seq;
        self.count = self.count.saturating_add(1);
        self.remaining = self.kind.lifetime();
    }

    /// Runs the timer down; `true` once it has expired.
    fn tick(&mut self, elapsed: Duration) -> bool {
        match &mut self.remaining {
            Some(left) => {
                *left = left.saturating_sub(elapsed);
                left.is_zero()
            }
            None => false,
        }
    }
}

/// The Toasts for one Client window: the stack, the status-line echo and the session history.
/// `S` is the bin's wall-clock stamp for the history's "time last raised".
#[derive(Debug, Clone)]
pub struct Toasts<S> {
    // Oldest first. Only the last `MAX_VISIBLE` are drawn; anything before them is a sticky
    // Error held back because every visible Toast was sticky, counted by `more_count`.
    stack: Vec<Toast>,
    // The non-Error Toast carried by the status line while Toasts are off.
    off_echo: Option<Toast>,
    history: History<S>,
    display: Display,
    paused: bool,
    next_seq: u64,
}

impl<S> Default for Toasts<S> {
    fn default() -> Self {
        Self::new(Display::default())
    }
}

impl<S> Toasts<S> {
    pub fn new(display: Display) -> Self {
        Self {
            stack: Vec::new(),
            off_echo: None,
            history: History::default(),
            display,
            paused: false,
            next_seq: 0,
        }
    }

    /// Raises a Toast whose Message the bin has already resolved to text. A Toast with the same
    /// Kind and text as a live one merges into it instead, and every raise is recorded in the
    /// history whatever the Preference.
    pub fn raise(&mut self, kind: ToastKind, text: impl Into<String>, raised_at: S) {
        let text = text.into();
        let seq = self.take_seq();
        let to_stack = kind == ToastKind::Error || self.display.toasts_on;

        let existing = if to_stack {
            self.stack
                .iter()
                .position(|t| t.kind == kind && t.text == text)
                .map(|i| self.stack.remove(i))
        } else {
            self.off_echo.take_if(|t| t.kind == kind && t.text == text)
        };

        let toast = match existing {
            Some(mut toast) => {
                toast.merge(seq);
                // A sticky Error can outlive 100 newer raises, so its entry may be gone.
                if let Err(stamp) = self.history.bump(toast.history_id, toast.count, raised_at) {
                    toast.history_id =
                        self.history
                            .record(kind, toast.text.clone(), toast.count, stamp);
                }
                toast
            }
            None => {
                let history_id = self.history.record(kind, text.clone(), 1, raised_at);
                Toast {
                    seq,
                    history_id,
                    kind,
                    text,
                    count: 1,
                    remaining: kind.lifetime(),
                }
            }
        };

        if to_stack {
            self.push(toast);
        } else {
            self.off_echo = Some(toast);
        }
    }

    /// The only clock: runs every live Toast's timer down by `elapsed` and drops the expired
    /// ones. Does nothing while paused.
    pub fn advance(&mut self, elapsed: Duration) {
        if self.paused {
            return;
        }
        self.stack.retain_mut(|t| !t.tick(elapsed));
        if self.off_echo.as_mut().is_some_and(|t| t.tick(elapsed)) {
            self.off_echo = None;
        }
    }

    /// Stops the timers, for hover (Desktop) and while a modal surface is open.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// `:dismiss`: removes the newest live Toast, stacked or echoed. `false` if there was none.
    pub fn dismiss_newest(&mut self) -> bool {
        let newest_stacked = self.stack.last().map(|t| t.seq);
        let echoed = self.off_echo.as_ref().map(|t| t.seq);
        match (newest_stacked, echoed) {
            (Some(s), Some(e)) if e > s => self.off_echo = None,
            (Some(_), _) => {
                self.stack.pop();
            }
            (None, Some(_)) => self.off_echo = None,
            (None, None) => return false,
        }
        true
    }

    /// The Desktop ✕: removes the Toast at `index` in [`Self::visible`]. `false` if out of range.
    pub fn dismiss_visible(&mut self, index: usize) -> bool {
        let start = self.stack.len().saturating_sub(MAX_VISIBLE);
        if !self.display.can_draw || index >= self.stack.len() - start {
            return false;
        }
        self.stack.remove(start + index);
        true
    }

    /// `:dismiss all` and `Ctrl+L`: removes every live Toast, held-back Errors included.
    pub fn dismiss_all(&mut self) {
        self.stack.clear();
        self.off_echo = None;
    }

    /// Applies a change to the Preference or the view size. Turning Toasts off takes the showing
    /// Info, Success and Warning Toasts off the stack and moves the newest to the echo for the
    /// rest of its lifetime; turning them back on returns the echoed Toast to the stack.
    pub fn set_display(&mut self, display: Display) {
        let was_on = self.display.toasts_on;
        self.display = display;
        if was_on && !display.toasts_on {
            let (errors, others): (Vec<_>, Vec<_>) = std::mem::take(&mut self.stack)
                .into_iter()
                .partition(|t| t.kind == ToastKind::Error);
            self.stack = errors;
            self.off_echo = others.into_iter().max_by_key(|t| t.seq);
        } else if !was_on
            && display.toasts_on
            && let Some(toast) = self.off_echo.take()
        {
            self.push(toast);
        }
    }

    pub fn display(&self) -> Display {
        self.display
    }

    /// The Toasts to draw, oldest first (the bin stacks the last nearest the status line).
    /// Empty when the bin cannot draw.
    pub fn visible(&self) -> &[Toast] {
        if !self.display.can_draw {
            return &[];
        }
        let start = self.stack.len().saturating_sub(MAX_VISIBLE);
        &self.stack[start..]
    }

    /// The `+N more` count above the stack: held-back sticky Errors.
    pub fn more_count(&self) -> usize {
        if !self.display.can_draw {
            return 0;
        }
        self.stack.len().saturating_sub(MAX_VISIBLE)
    }

    /// What the status-line echo shows, if anything: the Toast carried while Toasts are off,
    /// or, when the bin cannot draw, the newest live Toast of any Kind.
    pub fn echo(&self) -> Option<&Toast> {
        let off = self.off_echo.as_ref();
        if self.display.can_draw {
            return off;
        }
        [self.stack.last(), off]
            .into_iter()
            .flatten()
            .max_by_key(|t| t.seq)
    }

    pub fn history(&self) -> &History<S> {
        &self.history
    }

    fn take_seq(&mut self) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }

    /// Pushes onto the stack, then evicts: the oldest timed visible Toast goes early; if every
    /// visible Toast is sticky the oldest simply falls out of view and waits, since an Error is
    /// never silently lost.
    fn push(&mut self, toast: Toast) {
        let visible_before = self.stack.len().saturating_sub(MAX_VISIBLE);
        let was_full = self.stack.len() - visible_before >= MAX_VISIBLE;
        self.stack.push(toast);
        if was_full
            && let Some(i) =
                (visible_before..self.stack.len() - 1).find(|&i| self.stack[i].is_timed())
        {
            self.stack.remove(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ToastKind::{Error, Info, Success, Warning};

    const TICK: Duration = Duration::from_millis(250);

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    fn texts(toasts: &Toasts<u32>) -> Vec<&str> {
        toasts.visible().iter().map(Toast::text).collect()
    }

    fn off() -> Display {
        Display {
            toasts_on: false,
            can_draw: true,
        }
    }

    fn cannot_draw() -> Display {
        Display {
            toasts_on: true,
            can_draw: false,
        }
    }

    #[test]
    fn info_and_success_expire_after_four_seconds() {
        for kind in [Info, Success] {
            let mut toasts = Toasts::default();
            toasts.raise(kind, "a", 0);
            toasts.advance(secs(4) - TICK);
            assert_eq!(texts(&toasts), ["a"]);
            toasts.advance(TICK);
            assert!(toasts.visible().is_empty());
        }
    }

    #[test]
    fn warning_expires_after_eight_seconds() {
        let mut toasts = Toasts::default();
        toasts.raise(Warning, "a", 0);
        toasts.advance(secs(8) - TICK);
        assert_eq!(texts(&toasts), ["a"]);
        toasts.advance(TICK);
        assert!(toasts.visible().is_empty());
    }

    #[test]
    fn error_is_sticky() {
        let mut toasts = Toasts::default();
        toasts.raise(Error, "a", 0);
        toasts.advance(secs(3600));
        assert_eq!(texts(&toasts), ["a"]);
        assert_eq!(toasts.visible()[0].remaining(), None);
    }

    #[test]
    fn paused_toasts_do_not_run_down_and_resume_where_they_left_off() {
        let mut toasts = Toasts::default();
        toasts.raise(Info, "a", 0);
        toasts.advance(secs(3));
        toasts.pause();
        assert!(toasts.is_paused());
        toasts.advance(secs(60));
        assert_eq!(texts(&toasts), ["a"]);
        toasts.resume();
        toasts.advance(secs(1));
        assert!(toasts.visible().is_empty());
    }

    #[test]
    fn a_duplicate_merges_counts_and_restarts_the_timer() {
        let mut toasts = Toasts::default();
        toasts.raise(Info, "a", 0);
        toasts.advance(secs(3));
        toasts.raise(Info, "a", 1);
        assert_eq!(toasts.visible().len(), 1);
        assert_eq!(toasts.visible()[0].count(), 2);
        toasts.advance(secs(3));
        assert_eq!(texts(&toasts), ["a"]);
        toasts.advance(secs(1));
        assert!(toasts.visible().is_empty());
    }

    #[test]
    fn a_merged_duplicate_becomes_the_newest() {
        let mut toasts = Toasts::default();
        toasts.raise(Info, "a", 0);
        toasts.raise(Info, "b", 1);
        toasts.raise(Info, "a", 2);
        assert_eq!(texts(&toasts), ["b", "a"]);
    }

    #[test]
    fn same_text_of_a_different_kind_does_not_merge() {
        let mut toasts = Toasts::default();
        toasts.raise(Info, "a", 0);
        toasts.raise(Warning, "a", 1);
        assert_eq!(toasts.visible().len(), 2);
    }

    #[test]
    fn at_most_three_show_and_the_oldest_timed_is_evicted() {
        let mut toasts = Toasts::default();
        toasts.raise(Error, "e", 0);
        toasts.raise(Info, "a", 1);
        toasts.raise(Info, "b", 2);
        toasts.raise(Info, "c", 3);
        assert_eq!(texts(&toasts), ["e", "b", "c"]);
        assert_eq!(toasts.more_count(), 0);
    }

    #[test]
    fn a_sticky_error_is_held_back_only_when_every_visible_toast_is_sticky() {
        let mut toasts = Toasts::default();
        for (i, text) in ["e1", "e2", "e3"].into_iter().enumerate() {
            toasts.raise(Error, text, i as u32);
        }
        toasts.raise(Info, "a", 3);
        assert_eq!(texts(&toasts), ["e2", "e3", "a"]);
        assert_eq!(toasts.more_count(), 1);

        // The held-back Error comes back into view once a slot frees.
        toasts.advance(secs(4));
        assert_eq!(texts(&toasts), ["e1", "e2", "e3"]);
        assert_eq!(toasts.more_count(), 0);
    }

    #[test]
    fn dismiss_newest_then_all() {
        let mut toasts = Toasts::default();
        toasts.raise(Error, "e", 0);
        toasts.raise(Info, "a", 1);
        assert!(toasts.dismiss_newest());
        assert_eq!(texts(&toasts), ["e"]);
        toasts.raise(Info, "b", 2);
        toasts.dismiss_all();
        assert!(toasts.visible().is_empty());
        assert!(!toasts.dismiss_newest());
    }

    #[test]
    fn dismiss_visible_removes_that_toast_and_brings_a_held_back_error_into_view() {
        let mut toasts = Toasts::default();
        for i in 0..4 {
            toasts.raise(Error, format!("e{i}"), i);
        }
        assert_eq!(texts(&toasts), ["e1", "e2", "e3"]);
        assert!(toasts.dismiss_visible(1));
        assert_eq!(texts(&toasts), ["e0", "e1", "e3"]);
        assert_eq!(toasts.more_count(), 0);
        assert!(!toasts.dismiss_visible(3));
    }

    #[test]
    fn dismiss_all_includes_held_back_errors() {
        let mut toasts = Toasts::default();
        for i in 0..5 {
            toasts.raise(Error, format!("e{i}"), i);
        }
        assert_eq!(toasts.more_count(), 2);
        toasts.dismiss_all();
        assert_eq!(toasts.more_count(), 0);
        assert!(toasts.visible().is_empty());
    }

    #[test]
    fn with_toasts_off_non_errors_go_to_the_echo_for_their_lifetime() {
        let mut toasts = Toasts::new(off());
        toasts.raise(Success, "a", 0);
        assert!(toasts.visible().is_empty());
        assert_eq!(toasts.echo().map(Toast::text), Some("a"));
        toasts.advance(secs(4));
        assert!(toasts.echo().is_none());
    }

    #[test]
    fn with_toasts_off_the_newest_non_error_replaces_the_echo() {
        let mut toasts = Toasts::new(off());
        toasts.raise(Info, "a", 0);
        toasts.raise(Warning, "b", 1);
        assert_eq!(toasts.echo().map(Toast::text), Some("b"));
    }

    #[test]
    fn with_toasts_off_an_echoed_duplicate_merges() {
        let mut toasts = Toasts::new(off());
        toasts.raise(Info, "a", 0);
        toasts.raise(Info, "a", 1);
        assert_eq!(toasts.echo().map(Toast::count), Some(2));
    }

    #[test]
    fn errors_always_toast() {
        let mut toasts = Toasts::new(off());
        toasts.raise(Error, "e", 0);
        assert_eq!(texts(&toasts), ["e"]);
        assert!(toasts.echo().is_none());
    }

    #[test]
    fn with_toasts_on_nothing_echoes() {
        let mut toasts = Toasts::default();
        toasts.raise(Info, "a", 0);
        assert!(toasts.echo().is_none());
    }

    #[test]
    fn turning_off_clears_non_errors_and_moves_the_newest_to_the_echo() {
        let mut toasts = Toasts::default();
        toasts.raise(Info, "a", 0);
        toasts.raise(Error, "e", 1);
        toasts.raise(Warning, "w", 2);
        toasts.advance(secs(5));
        toasts.set_display(off());
        assert_eq!(texts(&toasts), ["e"]);
        let echo = toasts.echo().map(|t| (t.text(), t.remaining()));
        assert_eq!(echo, Some(("w", Some(secs(3)))));
    }

    #[test]
    fn turning_back_on_returns_the_echo_to_the_stack() {
        let mut toasts = Toasts::new(off());
        toasts.raise(Info, "a", 0);
        toasts.set_display(Display::default());
        assert!(toasts.echo().is_none());
        assert_eq!(texts(&toasts), ["a"]);
    }

    #[test]
    fn when_the_bin_cannot_draw_the_echo_carries_the_newest_of_any_kind() {
        let mut toasts = Toasts::new(cannot_draw());
        toasts.raise(Info, "a", 0);
        toasts.raise(Error, "e", 1);
        assert!(toasts.visible().is_empty());
        assert_eq!(toasts.more_count(), 0);
        assert_eq!(toasts.echo().map(Toast::text), Some("e"));

        // The Error echo stays until dismissed.
        toasts.advance(secs(3600));
        assert_eq!(toasts.echo().map(Toast::text), Some("e"));
        toasts.dismiss_newest();
        assert!(toasts.echo().is_none());
    }

    #[test]
    fn when_the_bin_cannot_draw_with_toasts_off_the_newest_still_echoes() {
        let mut toasts = Toasts::new(Display {
            toasts_on: false,
            can_draw: false,
        });
        toasts.raise(Error, "e", 0);
        toasts.raise(Info, "a", 1);
        assert_eq!(toasts.echo().map(Toast::text), Some("a"));
        toasts.advance(secs(4));
        assert_eq!(toasts.echo().map(Toast::text), Some("e"));
    }

    #[test]
    fn regaining_room_to_draw_shows_the_stack_again() {
        let mut toasts = Toasts::new(cannot_draw());
        toasts.raise(Error, "e", 0);
        toasts.set_display(Display::default());
        assert_eq!(texts(&toasts), ["e"]);
        assert!(toasts.echo().is_none());
    }

    #[test]
    fn every_toast_enters_the_history_whatever_the_preference() {
        let mut toasts = Toasts::new(off());
        toasts.raise(Info, "a", 10);
        toasts.raise(Error, "e", 20);
        let history: Vec<_> = toasts
            .history()
            .iter()
            .map(|e| (e.kind(), e.text(), *e.last_raised()))
            .collect();
        assert_eq!(history, [(Error, "e", 20), (Info, "a", 10)]);
    }

    #[test]
    fn a_merged_duplicate_is_one_history_entry_with_time_last_raised() {
        let mut toasts = Toasts::default();
        toasts.raise(Info, "a", 10);
        toasts.raise(Info, "b", 20);
        toasts.raise(Info, "a", 30);
        let history: Vec<_> = toasts
            .history()
            .iter()
            .map(|e| (e.text(), e.count(), *e.last_raised()))
            .collect();
        assert_eq!(history, [("a", 2, 30), ("b", 1, 20)]);
    }

    #[test]
    fn an_expired_toast_raised_again_is_a_new_history_entry() {
        let mut toasts = Toasts::default();
        toasts.raise(Info, "a", 10);
        toasts.advance(secs(4));
        toasts.raise(Info, "a", 20);
        assert_eq!(toasts.history().len(), 2);
    }

    #[test]
    fn history_keeps_the_newest_hundred() {
        let mut toasts = Toasts::default();
        for i in 0..150 {
            toasts.raise(Info, format!("t{i}"), i);
            toasts.advance(secs(4));
        }
        let history = toasts.history();
        assert_eq!(history.len(), crate::HISTORY_LIMIT);
        assert_eq!(history.iter().next().map(|e| e.text()), Some("t149"));
        assert_eq!(history.iter().last().map(|e| e.text()), Some("t50"));
    }

    #[test]
    fn a_sticky_error_merging_after_its_entry_dropped_records_afresh() {
        let mut toasts = Toasts::default();
        toasts.raise(Error, "e", 0);
        for i in 1..=100 {
            toasts.raise(Info, format!("t{i}"), i);
            toasts.advance(secs(4));
        }
        assert!(toasts.history().iter().all(|e| e.text() != "e"));
        toasts.raise(Error, "e", 200);
        let newest = toasts
            .history()
            .iter()
            .next()
            .map(|e| (e.text(), e.count()));
        assert_eq!(newest, Some(("e", 2)));
    }
}
