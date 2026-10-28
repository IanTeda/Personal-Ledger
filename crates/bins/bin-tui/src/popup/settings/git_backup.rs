//! Settings › Backup's Set up Git backup overlay (TUI-036, Desktop 16t), drawn like Sync Server's
//! Edit (TUI-035): the remote, branch, authentication, push schedule and commit-message text, a
//! test, and Save & push, which stays dim until a test passes.
//!
//! Git Backup pushes a text dump of the Ledger, plus its Documents through LFS, only to a private
//! repository (ADR-0033), so the test is a dry-run push that also checks the repository is
//! private and supports LFS. A wireframe: there is no Git Backup writer yet, so the test runs on a
//! stub, [`check`], and Save only flashes "not yet built". An access token is a [`SecretString`],
//! drawn masked and never logged.

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use secrecy::{ExposeSecret as _, SecretString};

use super::overlay::{Field, Key, Outcome, Overlay, Tested, Value, stepped};
use crate::colours::Colours;
use crate::msg;
use crate::units::view::dialog::edited;

/// wireframe: the SSH keys found in `~/.ssh`, until the Client looks.
const SSH_KEYS: [&str; 2] = ["~/.ssh/id_ed25519", "~/.ssh/id_rsa"];

/// What a dry-run push to a remote that answers reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reply {
    Ready,
    Public,
    NoLfs,
}

/// wireframe: the remotes the stub answers for, and what each reports; any other is unreachable.
const REMOTES: [(&str, Reply); 3] = [
    ("git@github.com:teda/ledger-backup.git", Reply::Ready),
    ("git@github.com:teda/ledger-public.git", Reply::Public),
    ("git@git.home.lan:ledger.git", Reply::NoLfs),
];

/// wireframe: the account the stub authenticates as.
const USER: &str = "teda";

/// The fields, top to bottom; the fourth is the SSH key or the access token, by authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Remote,
    Branch,
    Authentication,
    Credential,
    Schedule,
    Message,
}

impl Focus {
    const ALL: [Self; 6] = [
        Self::Remote,
        Self::Branch,
        Self::Authentication,
        Self::Credential,
        Self::Schedule,
        Self::Message,
    ];

    fn step(self, delta: isize) -> Self {
        let index = Self::ALL
            .iter()
            .position(|focus| *focus == self)
            .unwrap_or(0);
        let len = Self::ALL.len();
        Self::ALL[(index + len).wrapping_add_signed(delta) % len]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Authentication {
    SshKey,
    AccessToken,
}

/// When Git Backup pushes (ADR-0033).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Schedule {
    AfterChanges,
    Hourly,
    Daily,
    Manual,
}

impl Schedule {
    const ALL: [Self; 4] = [Self::AfterChanges, Self::Hourly, Self::Daily, Self::Manual];

    fn label(self) -> String {
        match self {
            Self::AfterChanges => msg::tui_settings_git_schedule_after_changes(),
            Self::Hourly => msg::tui_settings_git_schedule_hourly(),
            Self::Daily => msg::tui_settings_git_schedule_daily(),
            Self::Manual => msg::tui_settings_git_schedule_manual(),
        }
    }

    fn note(self) -> String {
        match self {
            Self::AfterChanges => msg::tui_settings_git_schedule_note_after_changes(),
            Self::Hourly => msg::tui_settings_git_schedule_note_hourly(),
            Self::Daily => msg::tui_settings_git_schedule_note_daily(),
            Self::Manual => msg::tui_settings_git_schedule_note_manual(),
        }
    }
}

pub struct GitBackupPopup {
    remote: String,
    branch: String,
    authentication: Authentication,
    /// The selected one of [`SSH_KEYS`].
    ssh_key: usize,
    token: SecretString,
    schedule: Schedule,
    /// The commit message after its fixed date prefix.
    message: String,
    focus: Focus,
    tested: Tested,
    /// The first push's size, for the save note.
    size: &'static str,
}

impl GitBackupPopup {
    /// The overlay on `remote` and `branch`, pushing by SSH key after changes, with `size` the
    /// first push's.
    pub fn new(remote: &str, branch: &str, size: &'static str) -> Self {
        Self {
            remote: remote.to_string(),
            branch: branch.to_string(),
            authentication: Authentication::SshKey,
            ssh_key: 0,
            token: SecretString::from(String::new()),
            schedule: Schedule::AfterChanges,
            message: msg::tui_settings_git_commit_default(),
            focus: Focus::Remote,
            tested: Tested::Untested,
            size,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        let changed = match Key::of(key) {
            Key::Cancel => return Outcome::Cancel,
            Key::Save if self.tested.passed() => return Outcome::Save,
            Key::Test => {
                self.tested = self.check();
                false
            }
            Key::Next => {
                self.focus = self.focus.step(1);
                false
            }
            Key::Prev => {
                self.focus = self.focus.step(-1);
                false
            }
            Key::Step(delta) => self.step(delta),
            Key::Edit(key) => self.edit(key),
            Key::Save => false,
        };
        if changed {
            self.tested = Tested::Untested;
        }
        Outcome::Stay
    }

    /// Steps the focused choice; whether it changed.
    fn step(&mut self, delta: isize) -> bool {
        match self.focus {
            Focus::Authentication => {
                let was = self.authentication;
                self.authentication = if delta < 0 {
                    Authentication::SshKey
                } else {
                    Authentication::AccessToken
                };
                self.authentication != was
            }
            Focus::Credential if self.authentication == Authentication::SshKey => {
                let was = self.ssh_key;
                self.ssh_key = stepped(self.ssh_key, SSH_KEYS.len(), delta);
                self.ssh_key != was
            }
            Focus::Schedule => {
                let was = self.schedule;
                let index = Schedule::ALL
                    .iter()
                    .position(|schedule| *schedule == was)
                    .unwrap_or(0);
                self.schedule = Schedule::ALL[stepped(index, Schedule::ALL.len(), delta)];
                self.schedule != was
            }
            _ => false,
        }
    }

    /// Types `key` into the focused text field; whether it changed.
    fn edit(&mut self, key: KeyEvent) -> bool {
        let text = match self.focus {
            Focus::Remote => &mut self.remote,
            Focus::Branch => &mut self.branch,
            Focus::Message => &mut self.message,
            Focus::Credential if self.authentication == Authentication::AccessToken => {
                return edited(self.token.expose_secret(), key)
                    .map(|token| self.token = token.into())
                    .is_some();
            }
            _ => return false,
        };
        edited(text, key).map(|edited| *text = edited).is_some()
    }

    /// wireframe: the stub test. It checks what the Client can check alone, a remote, a branch
    /// and a token, then answers as [`REMOTES`] say: a public repository or one without LFS is
    /// refused, and any other remote is unreachable.
    fn check(&self) -> Tested {
        let remote = self.remote.trim();
        let branch = self.branch.trim();
        let failed = |title, detail| Tested::Failed { title, detail };
        if !(remote.starts_with("git@") || remote.starts_with("https://")) {
            return failed(
                msg::tui_settings_git_failed_remote(),
                msg::tui_settings_git_failed_remote_detail(),
            );
        }
        if branch.is_empty() {
            return failed(
                msg::tui_settings_git_failed_branch(),
                msg::tui_settings_git_failed_branch_detail(),
            );
        }
        if self.authentication == Authentication::AccessToken
            && self.token.expose_secret().trim().is_empty()
        {
            return failed(
                msg::tui_settings_git_failed_token(),
                msg::tui_settings_git_failed_token_detail(),
            );
        }
        match REMOTES.iter().find(|(known, _)| *known == remote) {
            None => failed(
                msg::tui_settings_git_failed_unreachable(),
                msg::tui_settings_git_failed_unreachable_detail(remote),
            ),
            Some((_, Reply::Public)) => failed(
                msg::tui_settings_git_failed_public(),
                msg::tui_settings_git_failed_public_detail(),
            ),
            Some((_, Reply::NoLfs)) => failed(
                msg::tui_settings_git_failed_lfs(),
                msg::tui_settings_git_failed_lfs_detail(),
            ),
            Some((_, Reply::Ready)) => Tested::Passed {
                title: msg::tui_settings_git_passed(),
                detail: msg::tui_settings_git_passed_detail(USER, branch),
            },
        }
    }

    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let credential = match self.authentication {
            Authentication::SshKey => Field {
                label: msg::tui_settings_git_ssh_key(),
                value: Value::Choice {
                    options: SSH_KEYS.iter().map(ToString::to_string).collect(),
                    selected: self.ssh_key,
                },
                note: None,
            },
            Authentication::AccessToken => Field {
                label: msg::tui_settings_git_token(),
                value: Value::Secret(self.token.expose_secret().chars().count()),
                note: None,
            },
        };
        Overlay {
            title: msg::tui_settings_git_setup_title(),
            tag: None,
            fields: vec![
                Field {
                    label: msg::tui_settings_git_remote(),
                    value: Value::Text(self.remote.clone()),
                    note: Some(msg::tui_settings_git_remote_note()),
                },
                Field {
                    label: msg::tui_settings_git_branch(),
                    value: Value::Text(self.branch.clone()),
                    note: None,
                },
                Field {
                    label: msg::tui_settings_git_authentication(),
                    value: Value::Choice {
                        options: vec![
                            msg::tui_settings_git_auth_ssh(),
                            msg::tui_settings_git_auth_token(),
                        ],
                        selected: match self.authentication {
                            Authentication::SshKey => 0,
                            Authentication::AccessToken => 1,
                        },
                    },
                    note: None,
                },
                credential,
                Field {
                    label: msg::tui_settings_git_schedule(),
                    value: Value::Choice {
                        options: Schedule::ALL.iter().map(|s| s.label()).collect(),
                        selected: Schedule::ALL
                            .iter()
                            .position(|schedule| *schedule == self.schedule)
                            .unwrap_or(0),
                    },
                    note: Some(self.schedule.note()),
                },
                Field {
                    label: msg::tui_settings_git_commit_message(),
                    value: Value::Prefixed {
                        prefix: msg::tui_settings_git_commit_prefix(),
                        text: self.message.clone(),
                    },
                    note: Some(msg::tui_settings_git_commit_note()),
                },
            ],
            focus: Focus::ALL
                .iter()
                .position(|focus| *focus == self.focus)
                .unwrap_or(0),
            tested: &self.tested,
            save_note: msg::tui_settings_git_save_note(self.size),
            save_label: msg::tui_settings_git_save(),
        }
        .render(frame, area, c);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;

    const READY: &str = "git@github.com:teda/ledger-backup.git";

    fn press(popup: &mut GitBackupPopup, code: KeyCode) -> Outcome {
        popup.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn test(popup: &mut GitBackupPopup) {
        popup.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
    }

    fn failed_title(popup: &mut GitBackupPopup) -> String {
        test(popup);
        match &popup.tested {
            Tested::Failed { title, .. } => title.clone(),
            tested => panic!("the test didn't fail: {tested:?}"),
        }
    }

    #[test]
    fn the_test_refuses_a_public_repository_one_without_lfs_and_an_unknown_remote() {
        crate::locale::init_for_tests();
        let size = "2.84 mb";
        assert!(
            failed_title(&mut GitBackupPopup::new(
                "git@github.com:teda/ledger-public.git",
                "main",
                size
            ))
            .contains("public")
        );
        assert!(
            failed_title(&mut GitBackupPopup::new(
                "git@git.home.lan:ledger.git",
                "main",
                size
            ))
            .contains("LFS")
        );
        assert!(
            failed_title(&mut GitBackupPopup::new(
                "git@example.com:x.git",
                "main",
                size
            ))
            .contains("couldn't connect")
        );
        assert!(failed_title(&mut GitBackupPopup::new("ledger", "main", size)).contains("remote"));
        assert!(failed_title(&mut GitBackupPopup::new(READY, " ", size)).contains("branch"));
    }

    #[test]
    fn a_passing_test_unlocks_save_and_a_changed_choice_locks_it_again() {
        crate::locale::init_for_tests();
        let mut popup = GitBackupPopup::new(READY, "main", "2.84 mb");
        assert_eq!(press(&mut popup, KeyCode::Enter), Outcome::Stay);
        test(&mut popup);
        assert!(popup.tested.passed(), "{:?}", popup.tested);
        // Tab to the schedule and step it.
        for _ in 0..4 {
            press(&mut popup, KeyCode::Tab);
        }
        press(&mut popup, KeyCode::Right);
        assert_eq!(popup.schedule, Schedule::Hourly);
        assert_eq!(popup.tested, Tested::Untested);
        assert_eq!(press(&mut popup, KeyCode::Enter), Outcome::Stay);
        test(&mut popup);
        assert_eq!(press(&mut popup, KeyCode::Enter), Outcome::Save);
    }

    #[test]
    fn an_access_token_needs_typing_before_the_test_passes() {
        crate::locale::init_for_tests();
        let mut popup = GitBackupPopup::new(READY, "main", "2.84 mb");
        press(&mut popup, KeyCode::Tab);
        press(&mut popup, KeyCode::Tab);
        press(&mut popup, KeyCode::Right);
        assert_eq!(popup.authentication, Authentication::AccessToken);
        assert!(failed_title(&mut popup).contains("access token"));
        press(&mut popup, KeyCode::Tab);
        for ch in "ghp_secret".chars() {
            press(&mut popup, KeyCode::Char(ch));
        }
        test(&mut popup);
        assert!(popup.tested.passed(), "{:?}", popup.tested);
    }
}
