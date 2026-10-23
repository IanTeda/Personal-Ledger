//! [`Movement`]: one movement through whichever zone has focus, the rails or a View that isn't a
//! module yet. The Shell's global keymap names the keys (`lib_shell::keymap`) and the key route in
//! `crate::input` hands the movement on once the focused View has declined the key.

use lib_list_cursor::Step;
use lib_shell::ShellCommand;

/// A single semantic movement, dispatched against whichever zone is focused -- the same keys mean
/// different things per zone, but the keys-to-intent mapping itself doesn't vary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    Next,
    Prev,
    First,
    Last,
    HalfPageDown,
    HalfPageUp,
    Enter,
}

impl Movement {
    /// The movement a global command makes, `None` for one that isn't a movement.
    pub fn of(command: ShellCommand) -> Option<Self> {
        Some(match command {
            ShellCommand::Move(Step::Next) => Self::Next,
            ShellCommand::Move(Step::Prev) => Self::Prev,
            ShellCommand::Move(Step::First) => Self::First,
            ShellCommand::Move(Step::Last) => Self::Last,
            ShellCommand::Move(Step::HalfPageDown) => Self::HalfPageDown,
            ShellCommand::Move(Step::HalfPageUp) => Self::HalfPageUp,
            ShellCommand::Select => Self::Enter,
            _ => return None,
        })
    }

    /// The List Cursor step this movement makes, or `None` for `Enter`, which each list handles
    /// itself.
    pub fn step(self) -> Option<Step> {
        Some(match self {
            Self::Next => Step::Next,
            Self::Prev => Step::Prev,
            Self::First => Step::First,
            Self::Last => Step::Last,
            Self::HalfPageDown => Step::HalfPageDown,
            Self::HalfPageUp => Step::HalfPageUp,
            Self::Enter => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_move_command_is_its_movement_and_the_step_comes_back() {
        for step in [
            Step::Next,
            Step::Prev,
            Step::First,
            Step::Last,
            Step::HalfPageDown,
            Step::HalfPageUp,
        ] {
            let movement = Movement::of(ShellCommand::Move(step));
            assert_eq!(movement.and_then(Movement::step), Some(step));
        }
        assert_eq!(Movement::of(ShellCommand::Select), Some(Movement::Enter));
        assert_eq!(Movement::of(ShellCommand::Help), None);
    }
}
