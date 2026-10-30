//! The command registry driving `crate::shell::view::palette::Palette`
//! (`docs/ux/desktop-mockups/01-shell/README.md`'s "Command registry" section): "every rail
//! item, every context-rail footer affordance, and every view action registers a command with:
//! command string, description, kind ... optional binding, and a handler." Mirrors the shape of
//! `bin-tui`'s own per-domain registry (`crates/bins/bin-tui/src/popup/command/commands/`,
//! `docs/ux/tui-mockups/navigation.md`'s "Commands" section) -- plain, hand-authored data, one flat
//! array rather than domain modules (the desktop registry is small enough not to need
//! splitting the way the TUI's much larger one does), but each `Command` now carries a
//! `domain` field so the palette's own resting-state list can group by it the same way.
//!
//! **This reverses the "1d" spec's original "Results are ranked across kinds, not grouped"**
//! (recorded when the palette was first built, issue #151) -- the user's own later call, for
//! consistency with the TUI's own domain-headed resting-state list
//! (`bin-tui/src/popup/command/mod.rs`'s `Row::Header`), take precedence over that earlier
//! spec text.
//!
//! `Command::effect` (issue #144's own architecture review, "deepen the command's interface")
//! replaces an earlier `handler: Option<fn(&mut NavState)>` plus a `CommandKind` taxonomy field
//! that was almost entirely decorative (nothing rendered it, one test read it). That shape
//! couldn't express "open a `Root`-owned dialog" -- `open`/`new` had `handler: None` and
//! `Root::run_command` matched on `command.name` string literals to special-case them, a
//! typo-prone escape hatch mirrored in the type system by nothing at all. `CommandEffect` is a
//! closed enum instead: every command's effect is real, exhaustively matched, compiler-checked
//! -- the same shape `bin-tui`'s own `Action` enum already uses (ADR-0013), just per-command
//! rather than per-keypress.
//!
//! **The keymap is the table** (#846, ADR-0037's 2026-10-30 amendment): the global keymap and each
//! module's keymap mark every binding `Listed` or `Hidden` (`lib_viper::keymap::Palette`), and
//! [`entries`] lists each `Listed` one with its live key, one registration line per keymap. Running
//! one goes back through its keymap by name ([`CommandEffect::Shell`], [`CommandEffect::Module`]),
//! so the palette and the key can't drift. [`COMMANDS`] keeps only the Commands no keymap has.

use crate::navigation::nav::NounLabel;
use lib_colour_theme::ColourAppearance;
use lib_config::KeyBindingConfig;
use lib_shell::ShellCommand;
use lib_viper::keymap::Keymap;

use crate::{
    navigation::explorer::ExplorerMode, navigation::nav::Noun, settings::SettingsPage,
    theme::colours::ColourChange,
};

/// What running a command does -- the palette's, and `Root::run_command`'s, one source of
/// truth. Every variant is a real effect: there's no `None`/`Option` case standing in for "not
/// wired up yet" the way the old `handler` field had, because [`Self::NotYetBuilt`] says so
/// directly instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEffect {
    /// `:settings <page>`: opens that Settings page with focus in it.
    OpenSettingsPage(SettingsPage),
    /// Open the real "1e" file explorer dialog in this mode (`:open`/`:new`, issues #165/#167).
    OpenDialog(ExplorerMode),
    /// `:close`: returns to the "1a" empty state, same as cold start.
    CloseLedger,
    /// An `accounts <verb> [<account name>]` command: `Root::run_command` jumps to the Accounts
    /// page and opens the matching dialog, resolving the typed name (see [`split_input`]).
    Accounts(AccountsVerb),
    /// Sets the `colour_theme` or `colour_appearance` Preference, the same as picking it in
    /// Settings' Colour Theme group.
    Colour(ColourChange),
    /// `:dismiss`: removes the newest Toast.
    DismissToast,
    /// `:toasts on` / `:toasts off`: sets the Toasts Preference, the same as Settings' Toasts row.
    SetToasts(bool),
    /// A `budgets <verb>` command: `Root::run_command` jumps to the Budgets page and runs the
    /// verb on the Budget on show.
    Budgets(BudgetsVerb),
    /// A `documents <verb>` command: `Root::run_command` jumps to Documents and runs the verb.
    Documents(DocumentsVerb),
    /// `:import`: opens the stubbed 6e Import "match payees" step on the seeded statement.
    Import,
    /// A Command the Shell's global keymap lists: runs as its key does.
    Shell(ShellCommand),
    /// A Command a module's keymap lists: opens `page`, whose module Router sends the event its
    /// keymap binds `command` to, as the key would, on whatever row the page last had selected.
    Module {
        page: ModulePage,
        command: &'static str,
    },
    /// No real behaviour behind this command yet (`docs/ux/tui-mockups/01-shell/README.md`'s commitment: "a
    /// command that has no real behaviour yet says so explicitly when run") --
    /// `Root::run_command` turns this into the status-line flash.
    NotYetBuilt,
}

impl CommandEffect {
    /// Whether text typed after the command name is an argument (`accounts delete Home Loan`)
    /// rather than more of the query. The one predicate [`split_input`] and the palette's Tab
    /// completion both read; a keymap event can't carry an argument, so only the Accounts verbs do.
    pub fn takes_argument(&self) -> bool {
        matches!(self, Self::Accounts(_))
    }
}

/// The page a module Command runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModulePage {
    /// A Settings page that is a module's View.
    Settings(SettingsPage),
    Bills,
    Transactions,
}

/// The Accounts verbs the palette understands -- the same three the page's `n`/`e`/`d` keys and
/// buttons reach, through the same dialog-opening handlers. The TUI's `account off`/`on`/`check`
/// have no desktop counterpart: the desktop design has no active flag and no balance checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountsVerb {
    /// `accounts new [<account name>]`: the Add dialog, with Name pre-filled when given.
    New,
    /// `accounts edit [<account name>]`: the Edit dialog for the named (or selected) account.
    Edit,
    /// `accounts delete [<account name>]`: the Delete dialog for the named (or selected) account.
    Delete,
}

/// The Documents verbs the palette understands: the same actions the page's keys reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentsVerb {
    /// `documents inbox`: the Inbox.
    Inbox,
    /// `documents library`: the Library, on its last scope.
    Library,
    /// `documents accept-all`: the count-first confirm for every strong Inbox match.
    AcceptAll,
    /// `documents add`: the `+ Add` dialog.
    Add,
    /// `documents import`: the `Import…` dialog.
    Import,
}

/// The Budgets verbs the palette understands (#400): the same actions the page's keys and the
/// Switcher reach, through the same handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetsVerb {
    /// `budgets switch`: the Switcher popover (11b).
    Switch,
    /// `budgets new`: the New budget dialog (11c).
    New,
    /// `budgets edit`: 11c's edit mode on the Budget on show.
    Edit,
    /// `budgets manage`: Manage budgets (11f).
    Manage,
    /// `budgets duplicate`: copies the Budget on show and opens the copy in edit mode.
    Duplicate,
    /// `budgets set-default`: makes the Budget on show the default.
    SetDefault,
    /// `budgets archive`: archives the Budget on show.
    Archive,
    /// `budgets restore`: restores the Budget on show.
    Restore,
}

/// The palette's resting-state group a command sits under: one per noun or module, plus `Ledger`
/// for the file-level `open`/`new`/`close` trio and `Navigation` for the Shell's own keys, which
/// have no noun. The header text is a Message; the variants are the stable ids, declared in the
/// palette's order (`Dashboard` first, then alphabetical), which `Ord` follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Domain {
    Dashboard,
    Accounts,
    Bills,
    Budgets,
    Cash,
    Categories,
    CreditCards,
    Documents,
    Institutions,
    Inventory,
    Investments,
    Ledger,
    Loans,
    Navigation,
    Notifications,
    Payees,
    Reports,
    Settings,
    Tags,
    Toasts,
    Transactions,
    Units,
}

impl Domain {
    /// The header text in the Locale in effect, in sentence case.
    pub fn label(self) -> String {
        match self {
            Domain::Dashboard => Noun::Dashboard.label(),
            Domain::Accounts => SettingsPage::Accounts.label(),
            Domain::Bills => Noun::Bills.label(),
            Domain::Budgets => Noun::Budgets.label(),
            Domain::Cash => Noun::Cash.label(),
            Domain::Categories => SettingsPage::Categories.label(),
            Domain::CreditCards => Noun::CreditCards.label(),
            Domain::Documents => Noun::Documents.label(),
            Domain::Institutions => SettingsPage::Institutions.label(),
            Domain::Inventory => Noun::Inventory.label(),
            Domain::Investments => Noun::Investments.label(),
            Domain::Ledger => crate::msg::desktop_command_domain_ledger(),
            Domain::Loans => Noun::Loans.label(),
            Domain::Navigation => crate::msg::desktop_command_domain_navigation(),
            Domain::Notifications => Noun::Notifications.label(),
            Domain::Payees => SettingsPage::Payees.label(),
            Domain::Reports => Noun::Reports.label(),
            Domain::Settings => Noun::Settings.label(),
            Domain::Tags => SettingsPage::Tags.label(),
            Domain::Toasts => crate::msg::desktop_command_domain_toasts(),
            Domain::Transactions => Noun::Transactions.label(),
            Domain::Units => SettingsPage::Units.label(),
        }
    }
}

/// One [`COMMANDS`] row. `binding` is a plain display string (unlike the
/// TUI's `crossterm`-typed `Chord`, since `gpui`'s key model has no equivalent to render) --
/// `None` renders as an em dash in the palette, matching the TUI's `Chord::NONE`.
pub struct Command {
    /// The command as typed: a stable English id, never translated.
    pub name: &'static str,
    /// The palette's resting-state group (mirroring `bin-tui`'s own per-domain grouping).
    pub domain: Domain,
    /// The description Message, resolved in the Locale in effect when called.
    pub description: fn() -> String,
    pub binding: Option<&'static str>,
    pub effect: CommandEffect,
}

impl Command {
    /// Whether text typed after the command name is an argument (`accounts delete Home Loan`)
    /// rather than more of the query -- see [`split_input`].
    pub fn takes_argument(&self) -> bool {
        self.effect.takes_argument()
    }
}

/// Splits palette input into the command query and its argument text: when the input starts with
/// the full name of a command that takes an argument (case-insensitively, followed by a space or
/// the end of the input), that name is the query and the rest, trimmed, is the argument; any
/// other input is all query. The longest such name wins. Free text after the name may contain
/// spaces (`accounts new Rainy Day`).
pub fn split_input(input: &str) -> (&str, &str) {
    let best = COMMANDS
        .iter()
        .filter(|command| command.takes_argument())
        .filter(|command| {
            input
                .get(..command.name.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(command.name))
                && input[command.name.len()..]
                    .chars()
                    .next()
                    .is_none_or(|next| next == ' ')
        })
        .max_by_key(|command| command.name.len());
    match best {
        Some(command) => (
            &input[..command.name.len()],
            input[command.name.len()..].trim(),
        ),
        None => (input, ""),
    }
}

/// The Commands no keymap has, which the palette lists beside the keymaps' ([`entries`]): the
/// noun aliases with no `g` jump (`accounts`, `categories`, `payees`, `tags`), the three `accounts
/// <verb>` commands, which take a typed account name a keymap event can't carry (see
/// [`split_input`]), the Budgets and Documents verbs until their modules have keymaps, the
/// file-level `open`/`new`/`close` trio, each `settings <page>`, the Colour Themes, and the Toast
/// commands `dismiss`, `toasts on|off` and `messages` (an alias of the `toasts` binding). A
/// Command a keymap lists has no row here (`no_commands_row_is_a_command_a_keymap_lists`).
pub const COMMANDS: &[Command] = &[
    Command {
        name: "accounts",
        domain: Domain::Accounts,
        description: crate::msg::desktop_command_accounts_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsPage::Accounts),
    },
    Command {
        name: "accounts new",
        domain: Domain::Accounts,
        description: accounts_new_description,
        binding: None,
        effect: CommandEffect::Accounts(AccountsVerb::New),
    },
    Command {
        name: "accounts edit",
        domain: Domain::Accounts,
        description: crate::msg::desktop_command_accounts_edit_description,
        binding: None,
        effect: CommandEffect::Accounts(AccountsVerb::Edit),
    },
    Command {
        name: "accounts delete",
        domain: Domain::Accounts,
        description: crate::msg::desktop_command_accounts_delete_description,
        binding: None,
        effect: CommandEffect::Accounts(AccountsVerb::Delete),
    },
    Command {
        name: "budgets switch",
        domain: Domain::Budgets,
        description: crate::msg::desktop_command_budgets_switch_description,
        binding: Some("B"),
        effect: CommandEffect::Budgets(BudgetsVerb::Switch),
    },
    Command {
        name: "budgets new",
        domain: Domain::Budgets,
        description: crate::msg::desktop_command_budgets_new_description,
        binding: Some("n"),
        effect: CommandEffect::Budgets(BudgetsVerb::New),
    },
    Command {
        name: "budgets edit",
        domain: Domain::Budgets,
        description: crate::msg::desktop_command_budgets_edit_description,
        binding: None,
        effect: CommandEffect::Budgets(BudgetsVerb::Edit),
    },
    Command {
        name: "budgets manage",
        domain: Domain::Budgets,
        description: crate::msg::desktop_command_budgets_manage_description,
        binding: None,
        effect: CommandEffect::Budgets(BudgetsVerb::Manage),
    },
    Command {
        name: "budgets duplicate",
        domain: Domain::Budgets,
        description: crate::msg::desktop_command_budgets_duplicate_description,
        binding: None,
        effect: CommandEffect::Budgets(BudgetsVerb::Duplicate),
    },
    Command {
        name: "budgets set-default",
        domain: Domain::Budgets,
        description: crate::msg::desktop_command_budgets_set_default_description,
        binding: None,
        effect: CommandEffect::Budgets(BudgetsVerb::SetDefault),
    },
    Command {
        name: "budgets archive",
        domain: Domain::Budgets,
        description: crate::msg::desktop_command_budgets_archive_description,
        binding: None,
        effect: CommandEffect::Budgets(BudgetsVerb::Archive),
    },
    Command {
        name: "budgets restore",
        domain: Domain::Budgets,
        description: crate::msg::desktop_command_budgets_restore_description,
        binding: None,
        effect: CommandEffect::Budgets(BudgetsVerb::Restore),
    },
    Command {
        name: "categories",
        domain: Domain::Categories,
        description: crate::msg::desktop_command_categories_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsPage::Categories),
    },
    Command {
        name: "documents inbox",
        domain: Domain::Documents,
        description: crate::msg::desktop_command_documents_inbox_description,
        binding: Some("i"),
        effect: CommandEffect::Documents(DocumentsVerb::Inbox),
    },
    Command {
        name: "documents library",
        domain: Domain::Documents,
        description: crate::msg::desktop_command_documents_library_description,
        binding: Some("i"),
        effect: CommandEffect::Documents(DocumentsVerb::Library),
    },
    Command {
        name: "documents accept-all",
        domain: Domain::Documents,
        description: crate::msg::desktop_command_documents_accept_all_description,
        binding: Some("Y"),
        effect: CommandEffect::Documents(DocumentsVerb::AcceptAll),
    },
    Command {
        name: "documents add",
        domain: Domain::Documents,
        description: crate::msg::desktop_command_documents_add_description,
        binding: Some("a"),
        effect: CommandEffect::Documents(DocumentsVerb::Add),
    },
    Command {
        name: "documents import",
        domain: Domain::Documents,
        description: crate::msg::desktop_command_documents_import_description,
        binding: Some("I"),
        effect: CommandEffect::Documents(DocumentsVerb::Import),
    },
    Command {
        name: "open",
        domain: Domain::Ledger,
        description: crate::msg::desktop_command_open_description,
        binding: None,
        effect: CommandEffect::OpenDialog(ExplorerMode::Open),
    },
    Command {
        name: "new",
        domain: Domain::Ledger,
        description: crate::msg::desktop_command_new_description,
        binding: None,
        effect: CommandEffect::OpenDialog(ExplorerMode::New),
    },
    Command {
        name: "close",
        domain: Domain::Ledger,
        description: crate::msg::desktop_command_close_description,
        binding: None,
        effect: CommandEffect::CloseLedger,
    },
    Command {
        name: "payees",
        domain: Domain::Payees,
        description: crate::msg::desktop_command_payees_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsPage::Payees),
    },
    settings_page_command("settings general", SettingsPage::General),
    settings_page_command("settings display", SettingsPage::Display),
    settings_page_command("settings units", SettingsPage::Units),
    settings_page_command("settings institutions", SettingsPage::Institutions),
    settings_page_command("settings accounts", SettingsPage::Accounts),
    settings_page_command("settings categories", SettingsPage::Categories),
    settings_page_command("settings tags", SettingsPage::Tags),
    settings_page_command("settings payees", SettingsPage::Payees),
    settings_page_command("settings documents", SettingsPage::Documents),
    settings_page_command("settings inventory", SettingsPage::Inventory),
    settings_page_command("settings sync-server", SettingsPage::SyncServer),
    settings_page_command("settings data-backup", SettingsPage::DataBackup),
    settings_page_command("settings tracing", SettingsPage::Tracing),
    settings_page_command("settings about", SettingsPage::About),
    colour_theme_command("theme modernist", "modernist", || {
        colour_theme_description(lib_locale::msg::colour_theme_modernist())
    }),
    colour_theme_command("theme high-contrast", "high_contrast", || {
        colour_theme_description(lib_locale::msg::colour_theme_high_contrast())
    }),
    colour_theme_command("theme catppuccin", "catppuccin", || {
        colour_theme_description(lib_locale::msg::colour_theme_catppuccin())
    }),
    colour_theme_command("theme gruvbox", "gruvbox", || {
        colour_theme_description(lib_locale::msg::colour_theme_gruvbox())
    }),
    colour_theme_command("theme nord", "nord", || {
        colour_theme_description(lib_locale::msg::colour_theme_nord())
    }),
    colour_appearance_command("appearance light", ColourAppearance::Light, || {
        colour_appearance_description(lib_locale::msg::colour_appearance_light())
    }),
    colour_appearance_command("appearance dark", ColourAppearance::Dark, || {
        colour_appearance_description(lib_locale::msg::colour_appearance_dark())
    }),
    colour_appearance_command("appearance system", ColourAppearance::System, || {
        colour_appearance_description(lib_locale::msg::colour_appearance_system())
    }),
    Command {
        name: "tags",
        domain: Domain::Tags,
        description: crate::msg::desktop_command_tags_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsPage::Tags),
    },
    Command {
        name: "dismiss",
        domain: Domain::Toasts,
        description: crate::msg::desktop_command_dismiss_description,
        binding: None,
        effect: CommandEffect::DismissToast,
    },
    Command {
        name: "toasts on",
        domain: Domain::Toasts,
        description: crate::msg::desktop_command_toasts_on_description,
        binding: None,
        effect: CommandEffect::SetToasts(true),
    },
    Command {
        name: "toasts off",
        domain: Domain::Toasts,
        description: crate::msg::desktop_command_toasts_off_description,
        binding: None,
        effect: CommandEffect::SetToasts(false),
    },
    // `messages` is the alias the design names, for a user reaching for vim's `:messages`.
    Command {
        name: "messages",
        domain: Domain::Toasts,
        description: lib_locale::msg::command_toast_history,
        binding: None,
        effect: CommandEffect::Shell(ShellCommand::ToastHistory),
    },
    Command {
        name: "import",
        domain: Domain::Transactions,
        description: crate::msg::desktop_command_import_description,
        binding: None,
        effect: CommandEffect::Import,
    },
];

const fn settings_page_command(name: &'static str, section: SettingsPage) -> Command {
    Command {
        name,
        domain: Domain::Settings,
        // A fn pointer cannot capture `section`, so the description is the same sentence for
        // every page, naming it by the command itself.
        description: settings_page_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(section),
    }
}

fn settings_page_description() -> String {
    crate::msg::desktop_command_settings_page_description()
}

const fn colour_theme_command(
    name: &'static str,
    id: &'static str,
    description: fn() -> String,
) -> Command {
    Command {
        name,
        domain: Domain::Settings,
        description,
        binding: None,
        effect: CommandEffect::Colour(ColourChange::Theme(id)),
    }
}

const fn colour_appearance_command(
    name: &'static str,
    appearance: ColourAppearance,
    description: fn() -> String,
) -> Command {
    Command {
        name,
        domain: Domain::Settings,
        description,
        binding: None,
        effect: CommandEffect::Colour(ColourChange::Appearance(appearance)),
    }
}

fn colour_theme_description(theme: String) -> String {
    crate::msg::desktop_command_colour_theme_description(&theme)
}

fn colour_appearance_description(appearance: String) -> String {
    crate::msg::desktop_command_colour_appearance_description(&appearance)
}

fn accounts_new_description() -> String {
    crate::msg::desktop_command_accounts_new_description("accounts new")
}

/// One palette row: a [`COMMANDS`] row or a Command a keymap lists, with the key it is bound to
/// under `[keybindings]`.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The command as typed: a stable English id, never translated.
    pub name: &'static str,
    pub domain: Domain,
    /// The description Message, resolved in the Locale in effect when called.
    pub description: fn() -> String,
    /// The key, `None` while unbound (drawn as an em dash).
    pub key: Option<String>,
    pub effect: CommandEffect,
}

impl Entry {
    /// Whether text typed after the command name is an argument. Only a [`COMMANDS`] row takes
    /// one: a keymap event can't carry it.
    pub fn takes_argument(&self) -> bool {
        self.effect.takes_argument()
    }

    /// `command` as a row, its key read live where a keymap binds the same event: the Accounts
    /// verbs share the Accounts page's `new`, `edit` and `delete`, and `messages` the global
    /// keymap's `toast_history`.
    fn from_command(command: &Command, config: &KeyBindingConfig) -> Self {
        let live = match command.effect {
            CommandEffect::Accounts(verb) => {
                let name = match verb {
                    AccountsVerb::New => lib_viper::keymap::NEW,
                    AccountsVerb::Edit => lib_viper::keymap::EDIT,
                    AccountsVerb::Delete => lib_viper::keymap::DELETE,
                };
                lib_accounts::keymap::accounts_page_keymap(config)
                    .chord(name)
                    .and_then(key)
            }
            CommandEffect::Shell(ShellCommand::ToastHistory) => lib_shell::global_keymap(config)
                .chord(lib_shell::keymap::TOAST_HISTORY)
                .and_then(key),
            _ => command.binding.map(str::to_string),
        };
        Self {
            name: command.name,
            domain: command.domain,
            description: command.description,
            key: live,
            effect: command.effect,
        }
    }
}

/// Every Command the palette lists under `config`: the global keymap's, [`COMMANDS`], then each
/// module keymap's, grouped by [`Domain`] in its order and otherwise in that order -- the
/// palette's resting-state (empty query) order, and the order ties fall back to once ranked.
pub fn entries(config: &KeyBindingConfig) -> Vec<Entry> {
    let mut entries = global_entries(config);
    entries.extend(
        COMMANDS
            .iter()
            .map(|command| Entry::from_command(command, config)),
    );
    entries.extend(module_entries(config));
    entries.sort_by_key(|entry| entry.domain);
    entries
}

/// The Commands the Shell's global keymap lists, each under its noun's group, the Toasts', or
/// Navigation's.
fn global_entries(config: &KeyBindingConfig) -> Vec<Entry> {
    let keymap = lib_shell::global_keymap(config);
    keymap
        .listed()
        .filter_map(|listed| {
            let command = keymap.event_for(listed.command)?;
            Some(Entry {
                name: listed.name,
                domain: global_domain(command),
                description: listed.description,
                key: key(listed.chord),
                effect: CommandEffect::Shell(command),
            })
        })
        .collect()
}

fn global_domain(command: ShellCommand) -> Domain {
    match command {
        ShellCommand::Jump(noun) => match noun {
            Noun::Dashboard => Domain::Dashboard,
            Noun::Transactions => Domain::Transactions,
            Noun::Documents => Domain::Documents,
            Noun::Notifications => Domain::Notifications,
            Noun::Cash => Domain::Cash,
            Noun::Inventory => Domain::Inventory,
            Noun::Loans => Domain::Loans,
            Noun::CreditCards => Domain::CreditCards,
            Noun::Investments => Domain::Investments,
            Noun::Bills => Domain::Bills,
            Noun::Budgets => Domain::Budgets,
            Noun::Reports => Domain::Reports,
            Noun::Settings => Domain::Settings,
        },
        ShellCommand::DismissToasts | ShellCommand::ToastHistory => Domain::Toasts,
        _ => Domain::Navigation,
    }
}

/// One line per module keymap: the keymap, the page its Commands run on, and their group. The
/// Unit prices keymap has none here: every Command it binds is hidden.
fn module_entries(config: &KeyBindingConfig) -> Vec<Entry> {
    use crate::settings::SettingsPage as Page;
    let mut entries = Vec::new();
    let settings = |page| ModulePage::Settings(page);
    listed(
        &mut entries,
        &lib_accounts::keymap::accounts_page_keymap(config),
        settings(Page::Accounts),
        Domain::Accounts,
    );
    listed(
        &mut entries,
        &lib_bills::keymap::bills_page_keymap(config),
        ModulePage::Bills,
        Domain::Bills,
    );
    listed(
        &mut entries,
        &lib_categories::keymap::categories_page_keymap(config),
        settings(Page::Categories),
        Domain::Categories,
    );
    listed(
        &mut entries,
        &lib_institutions::keymap::institutions_page_keymap(config),
        settings(Page::Institutions),
        Domain::Institutions,
    );
    listed(
        &mut entries,
        &lib_inventory::keymap::inventory_page_keymap(config),
        settings(Page::Inventory),
        Domain::Inventory,
    );
    listed(
        &mut entries,
        &lib_payees::keymap::payees_page_keymap(config),
        settings(Page::Payees),
        Domain::Payees,
    );
    listed(
        &mut entries,
        &lib_tags::keymap::tags_page_keymap(config),
        settings(Page::Tags),
        Domain::Tags,
    );
    listed(
        &mut entries,
        &lib_transactions::keymap::transactions_page_keymap(config),
        ModulePage::Transactions,
        Domain::Transactions,
    );
    listed(
        &mut entries,
        &lib_units::keymap::units_page_keymap(config),
        settings(Page::Units),
        Domain::Units,
    );
    entries
}

fn listed<E: Clone>(
    entries: &mut Vec<Entry>,
    keymap: &Keymap<E>,
    page: ModulePage,
    domain: Domain,
) {
    entries.extend(keymap.listed().map(|listed| Entry {
        name: listed.name,
        domain,
        description: listed.description,
        key: key(listed.chord),
        effect: CommandEffect::Module {
            page,
            command: listed.command,
        },
    }));
}

/// A live chord as the palette shows it: an empty one is unbound.
fn key(chord: &str) -> Option<String> {
    (!chord.is_empty()).then(|| chord.to_string())
}

/// The [`COMMANDS`] row named `name` under `config`, as the palette would run it: the empty
/// state's `open` and `new` links, which no keymap binds.
pub fn find(name: &str, config: &KeyBindingConfig) -> Option<Entry> {
    COMMANDS
        .iter()
        .find(|command| command.name == name)
        .map(|command| Entry::from_command(command, config))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Vec<Entry> {
        entries(&KeyBindingConfig::default())
    }

    fn effect(name: &str) -> Option<CommandEffect> {
        defaults()
            .into_iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.effect)
    }

    #[test]
    fn every_entry_has_a_non_empty_name_and_description_and_a_name_of_its_own() {
        crate::locale::init_for_tests();
        let entries = defaults();
        for entry in &entries {
            assert!(!entry.name.is_empty());
            assert!(!(entry.description)().is_empty(), "{}", entry.name);
            assert_eq!(
                entries
                    .iter()
                    .filter(|other| other.name == entry.name)
                    .count(),
                1,
                "{} is listed twice",
                entry.name
            );
        }
    }

    #[test]
    fn no_commands_row_is_a_command_a_keymap_lists() {
        let config = KeyBindingConfig::default();
        let listed: Vec<_> = global_entries(&config)
            .into_iter()
            .chain(module_entries(&config))
            .collect();
        for command in COMMANDS {
            assert!(
                listed.iter().all(|entry| entry.name != command.name),
                "{} is in COMMANDS and a keymap",
                command.name
            );
        }
    }

    #[test]
    fn a_listed_command_shows_its_live_key_or_none_while_unbound() {
        let key = |config: &KeyBindingConfig, name: &str| {
            entries(config)
                .into_iter()
                .find(|entry| entry.name == name)
                .and_then(|entry| entry.key)
        };
        let defaults = KeyBindingConfig::default();
        assert_eq!(key(&defaults, "dashboard").as_deref(), Some("g d"));
        assert_eq!(key(&defaults, "units set-base").as_deref(), Some("B"));
        assert_eq!(key(&defaults, "toasts"), None, "toast_history is unbound");

        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert("jump_dashboard".to_string(), "g h".to_string());
        assert_eq!(key(&config, "dashboard").as_deref(), Some("g h"));

        let mut config = KeyBindingConfig::default();
        config.bindings.insert("new".to_string(), "a".to_string());
        config
            .bindings
            .insert("toast_history".to_string(), "ctrl+t".to_string());
        assert_eq!(key(&config, "accounts new").as_deref(), Some("a"));
        assert_eq!(key(&config, "messages").as_deref(), Some("ctrl+t"));
        assert_eq!(key(&config, "toasts").as_deref(), Some("ctrl+t"));
    }

    #[test]
    fn a_module_command_runs_on_its_page_and_a_global_one_as_its_key() {
        assert_eq!(
            effect("units set-base"),
            Some(CommandEffect::Module {
                page: ModulePage::Settings(SettingsPage::Units),
                command: lib_units::keymap::SET_BASE,
            })
        );
        assert_eq!(
            effect("bills pay"),
            Some(CommandEffect::Module {
                page: ModulePage::Bills,
                command: lib_bills::keymap::PAY,
            })
        );
        assert_eq!(
            effect("tags merge"),
            Some(CommandEffect::Module {
                page: ModulePage::Settings(SettingsPage::Tags),
                command: lib_tags::keymap::MERGE,
            })
        );
        assert_eq!(
            effect("help"),
            Some(CommandEffect::Shell(ShellCommand::Help))
        );
        assert_eq!(
            effect("dismiss all"),
            Some(CommandEffect::Shell(ShellCommand::DismissToasts))
        );
    }

    #[test]
    fn movement_select_and_the_accounts_keys_are_not_listed() {
        use lib_viper::keymap::{ARROW_DOWN, HOME, MOVE_DOWN, MOVE_FIRST, PAGE_UP, SELECT};
        let hidden = [
            MOVE_DOWN,
            MOVE_FIRST,
            ARROW_DOWN,
            HOME,
            PAGE_UP,
            SELECT,
            lib_units::keymap::OPEN,
            lib_categories::keymap::EXPAND,
            lib_categories::keymap::COLLAPSE,
            lib_inventory::keymap::EXPAND,
            lib_inventory::keymap::COLLAPSE,
        ];
        for entry in module_entries(&KeyBindingConfig::default()) {
            let CommandEffect::Module { page, command } = entry.effect else {
                panic!("{} is a module's but runs no module Command", entry.name);
            };
            assert_ne!(
                page,
                ModulePage::Settings(SettingsPage::Accounts),
                "Accounts' verbs take a name, so only COMMANDS lists them"
            );
            assert!(!hidden.contains(&command), "{command}");
        }
    }

    #[test]
    fn every_settings_page_has_a_command() {
        for section in SettingsPage::ALL {
            let name = format!("settings {}", section.id());
            assert_eq!(
                effect(&name),
                Some(CommandEffect::OpenSettingsPage(section)),
                "{name}"
            );
        }
    }

    #[test]
    fn toasts_on_and_off_set_the_preference() {
        assert_eq!(effect("toasts on"), Some(CommandEffect::SetToasts(true)));
        assert_eq!(effect("toasts off"), Some(CommandEffect::SetToasts(false)));
    }

    #[test]
    fn the_budgets_verbs_carry_their_effects() {
        for (name, verb) in [
            ("budgets switch", BudgetsVerb::Switch),
            ("budgets new", BudgetsVerb::New),
            ("budgets edit", BudgetsVerb::Edit),
            ("budgets manage", BudgetsVerb::Manage),
            ("budgets duplicate", BudgetsVerb::Duplicate),
            ("budgets set-default", BudgetsVerb::SetDefault),
            ("budgets archive", BudgetsVerb::Archive),
            ("budgets restore", BudgetsVerb::Restore),
        ] {
            assert_eq!(effect(name), Some(CommandEffect::Budgets(verb)), "{name}");
        }
    }

    #[test]
    fn the_moved_noun_commands_alias_their_settings_page_with_no_binding() {
        for (name, section) in [
            ("accounts", SettingsPage::Accounts),
            ("categories", SettingsPage::Categories),
            ("payees", SettingsPage::Payees),
            ("tags", SettingsPage::Tags),
        ] {
            let command = COMMANDS.iter().find(|c| c.name == name).unwrap();
            assert_eq!(command.effect, CommandEffect::OpenSettingsPage(section));
            assert_eq!(command.binding, None, "{name} lost its g binding");
        }
    }

    #[test]
    fn toasts_and_messages_open_the_history() {
        let history = Some(CommandEffect::Shell(ShellCommand::ToastHistory));
        assert_eq!(effect("toasts"), history);
        assert_eq!(effect("messages"), history);
    }

    #[test]
    fn every_noun_has_a_command_that_shows_it() {
        for noun in Noun::ALL {
            assert!(
                defaults().iter().any(|entry| matches!(
                    entry.effect,
                    CommandEffect::Shell(ShellCommand::Jump(jump)) if jump == noun
                )),
                "no command shows {noun:?}"
            );
        }
    }

    #[test]
    fn domains_are_contiguous_dashboard_first_then_alphabetical() {
        // Mirrors bin-tui's own `dashboard_is_first_and_the_rest_are_alphabetical` invariant --
        // the palette's resting-state grouping (`Palette::rows`) assumes each domain's commands
        // sit together, never split across two separate runs.
        let mut order: Vec<Domain> = Vec::new();
        for entry in defaults() {
            if order.last() != Some(&entry.domain) {
                assert!(
                    !order.contains(&entry.domain),
                    "domain {:?} is split across non-adjacent commands",
                    entry.domain
                );
                order.push(entry.domain);
            }
        }
        assert_eq!(order[0], Domain::Dashboard);
        // The domains' stable ids (their variant names) sort alphabetically, whatever the Locale
        // calls them.
        let rest = &order[1..];
        let mut sorted_rest = rest.to_vec();
        sorted_rest.sort_unstable_by_key(|domain| format!("{domain:?}"));
        assert_eq!(rest, sorted_rest.as_slice());
    }

    #[test]
    fn the_accounts_verbs_carry_their_effects_and_take_an_argument() {
        for (name, verb) in [
            ("accounts new", AccountsVerb::New),
            ("accounts edit", AccountsVerb::Edit),
            ("accounts delete", AccountsVerb::Delete),
        ] {
            let command = COMMANDS.iter().find(|c| c.name == name).unwrap();
            assert_eq!(command.effect, CommandEffect::Accounts(verb));
            assert!(command.takes_argument());
        }
        let accounts = COMMANDS.iter().find(|c| c.name == "accounts").unwrap();
        assert!(!accounts.takes_argument(), "the bare noun jump takes none");
    }

    #[test]
    fn split_input_separates_the_argument_after_a_full_command_name() {
        assert_eq!(
            split_input("accounts new Rainy Day"),
            ("accounts new", "Rainy Day")
        );
        assert_eq!(
            split_input("accounts delete   Home Loan  "),
            ("accounts delete", "Home Loan")
        );
        assert_eq!(split_input("accounts edit"), ("accounts edit", ""));
        assert_eq!(split_input("accounts edit "), ("accounts edit", ""));
    }

    #[test]
    fn split_input_matches_the_command_name_case_insensitively_keeping_the_arguments_case() {
        assert_eq!(
            split_input("Accounts New ANZ Offset"),
            ("Accounts New", "ANZ Offset")
        );
    }

    #[test]
    fn split_input_leaves_everything_else_as_the_query() {
        assert_eq!(split_input(""), ("", ""));
        assert_eq!(split_input("accounts"), ("accounts", ""));
        assert_eq!(split_input("accounts del"), ("accounts del", ""));
        assert_eq!(split_input("accounts deleted"), ("accounts deleted", ""));
        assert_eq!(split_input("open something"), ("open something", ""));
    }

    #[test]
    fn open_and_new_carry_the_matching_explorer_mode() {
        let open = COMMANDS.iter().find(|c| c.name == "open").unwrap();
        assert_eq!(open.effect, CommandEffect::OpenDialog(ExplorerMode::Open));

        let new = COMMANDS.iter().find(|c| c.name == "new").unwrap();
        assert_eq!(new.effect, CommandEffect::OpenDialog(ExplorerMode::New));
    }

    #[test]
    fn every_built_in_colour_theme_and_appearance_has_a_command() {
        for theme in lib_colour_theme::ColourTheme::built_in() {
            assert!(
                COMMANDS
                    .iter()
                    .any(|c| c.effect == CommandEffect::Colour(ColourChange::Theme(theme.id))),
                "no command sets Colour Theme {}",
                theme.id
            );
        }
        for appearance in [
            ColourAppearance::Light,
            ColourAppearance::Dark,
            ColourAppearance::System,
        ] {
            assert!(
                COMMANDS.iter().any(
                    |c| c.effect == CommandEffect::Colour(ColourChange::Appearance(appearance))
                ),
                "no command sets {appearance:?}"
            );
        }
    }

    #[test]
    fn close_carries_the_close_ledger_effect() {
        let close = COMMANDS.iter().find(|c| c.name == "close").unwrap();
        assert_eq!(close.effect, CommandEffect::CloseLedger);
    }
}
