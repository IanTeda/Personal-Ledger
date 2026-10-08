//! The command registry driving `crate::chrome::palette::Palette`
//! (`docs/ux/desktop/01-shell/README.md`'s "Command registry" section): "every rail
//! item, every context-rail footer affordance, and every view action registers a command with:
//! command string, description, kind ... optional binding, and a handler." Mirrors the shape of
//! `bin-tui`'s own per-domain registry (`crates/bins/bin-tui/src/popup/command/commands/`,
//! `docs/ux/mockups/navigation.md`'s "Commands" section) -- plain, hand-authored data, one flat
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
//! couldn't express "open a `Shell`-owned dialog" -- `open`/`new` had `handler: None` and
//! `Shell::run_command` matched on `command.name` string literals to special-case them, a
//! typo-prone escape hatch mirrored in the type system by nothing at all. `CommandEffect` is a
//! closed enum instead: every command's effect is real, exhaustively matched, compiler-checked
//! -- the same shape `bin-tui`'s own `Action` enum already uses (ADR-0013), just per-command
//! rather than per-keypress.

use lib_colour_theme::ColourAppearance;

use crate::{
    navigation::explorer::ExplorerMode, navigation::nav::Noun, settings::SettingsSection,
    theme::colours::ColourChange,
};

/// What running a command does -- the palette's, and `Shell::run_command`'s, one source of
/// truth. Every variant is a real effect: there's no `None`/`Option` case standing in for "not
/// wired up yet" the way the old `handler` field had, because [`Self::NotYetBuilt`] says so
/// directly instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEffect {
    /// Jump the primary rail to this noun -- `Shell::run_command` calls `NavState::set_noun`
    /// directly; there's no per-noun `fn(&mut NavState)` to indirect through any more.
    Navigate(Noun),
    /// `:settings <page>`: opens that Settings page with focus in it.
    OpenSettingsPage(SettingsSection),
    /// Open the real "1e" file explorer dialog in this mode (`:open`/`:new`, issues #165/#167).
    OpenDialog(ExplorerMode),
    /// `:close`: returns to the "1a" empty state, same as cold start.
    CloseLedger,
    /// An `accounts <verb> [<account name>]` command: `Shell::run_command` jumps to the Accounts
    /// page and opens the matching dialog, resolving the typed name (see [`split_input`]).
    Accounts(AccountsVerb),
    /// Sets the `colour_theme` or `colour_appearance` Preference, the same as picking it in
    /// Settings' Colour Theme group.
    Colour(ColourChange),
    /// `:dismiss`: removes the newest Toast.
    DismissToast,
    /// `:dismiss all`: removes every Toast, the same as the `dismiss_toasts` binding.
    DismissAllToasts,
    /// `:toasts on` / `:toasts off`: sets the Toasts Preference, the same as Settings' Toasts row.
    SetToasts(bool),
    /// `:toasts` / `:messages`: opens the session Toast history.
    OpenToastHistory,
    /// `:tags merge`: jumps to the Tags page and opens the 7e Merge dialog with neither select
    /// chosen (#354).
    MergeTags,
    /// A `budgets <verb>` command: `Shell::run_command` jumps to the Budgets page and runs the
    /// verb on the Budget on show.
    Budgets(BudgetsVerb),
    /// A `documents <verb>` command: `Shell::run_command` jumps to Documents and runs the verb.
    Documents(DocumentsVerb),
    /// `:import`: opens the stubbed 6e Import "match payees" step on the seeded statement.
    Import,
    /// No real behaviour behind this command yet (`docs/ux/mockups/README.md`'s commitment: "a
    /// command that has no real behaviour yet says so explicitly when run") --
    /// `Shell::run_command` turns this into the status-line flash.
    NotYetBuilt,
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

/// The palette's resting-state group a command sits under: one per noun, plus `Ledger` for the
/// file-level `open`/`new`/`close` trio, which has no noun of its own. The header text is a
/// Message; the variants are the stable ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    Dashboard,
    Accounts,
    Bills,
    Budgets,
    Cash,
    Categories,
    CreditCards,
    Documents,
    Inventory,
    Investments,
    Ledger,
    Loans,
    Notifications,
    Payees,
    Reports,
    Settings,
    Tags,
    Toasts,
    Transactions,
}

impl Domain {
    /// The header text in the Locale in effect, in sentence case.
    pub fn label(self) -> String {
        match self {
            Domain::Dashboard => Noun::Dashboard.label(),
            Domain::Accounts => SettingsSection::Accounts.label(),
            Domain::Bills => Noun::Bills.label(),
            Domain::Budgets => Noun::Budgets.label(),
            Domain::Cash => Noun::Cash.label(),
            Domain::Categories => SettingsSection::Categories.label(),
            Domain::CreditCards => Noun::CreditCards.label(),
            Domain::Documents => Noun::Documents.label(),
            Domain::Inventory => Noun::Inventory.label(),
            Domain::Investments => Noun::Investments.label(),
            Domain::Ledger => crate::msg::desktop_command_domain_ledger(),
            Domain::Loans => Noun::Loans.label(),
            Domain::Notifications => Noun::Notifications.label(),
            Domain::Payees => SettingsSection::Payees.label(),
            Domain::Reports => Noun::Reports.label(),
            Domain::Settings => Noun::Settings.label(),
            Domain::Tags => SettingsSection::Tags.label(),
            Domain::Toasts => crate::msg::desktop_command_domain_toasts(),
            Domain::Transactions => Noun::Transactions.label(),
        }
    }
}

/// One command: the palette's own unit of data. `binding` is a plain display string (unlike the
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
        matches!(self.effect, CommandEffect::Accounts(_))
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

/// Every command the palette can rank and run today, grouped by [`Command::domain`] --
/// `Dashboard` first, then every other domain alphabetically, mirroring the TUI's own
/// `commands::DOMAINS` order exactly (`bin-tui/src/popup/command/commands/mod.rs`: "Dashboard
/// first then alphabetical"). One [`CommandEffect::Navigate`] command per rail item (`Noun::ALL`'s
/// own order, each its own single-command domain), plus the one real footer affordance
/// (`crate::chrome::rail::context::footer`'s "+ new account · :accounts new", grouped under "Accounts"
/// alongside its own noun's command) and the file-level `open`/`new`/`close` trio under
/// `Ledger`, which has no noun of its own. The three `accounts <verb>` commands take a typed
/// account name (see [`split_input`]).
pub const COMMANDS: &[Command] = &[
    Command {
        name: "dashboard",
        domain: Domain::Dashboard,
        description: crate::msg::desktop_command_dashboard_description,
        binding: Some("g d"),
        effect: CommandEffect::Navigate(Noun::Dashboard),
    },
    Command {
        name: "accounts",
        domain: Domain::Accounts,
        description: crate::msg::desktop_command_accounts_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsSection::Accounts),
    },
    Command {
        name: "accounts new",
        domain: Domain::Accounts,
        description: accounts_new_description,
        binding: Some("n"),
        effect: CommandEffect::Accounts(AccountsVerb::New),
    },
    Command {
        name: "accounts edit",
        domain: Domain::Accounts,
        description: crate::msg::desktop_command_accounts_edit_description,
        binding: Some("e"),
        effect: CommandEffect::Accounts(AccountsVerb::Edit),
    },
    Command {
        name: "accounts delete",
        domain: Domain::Accounts,
        description: crate::msg::desktop_command_accounts_delete_description,
        binding: Some("d"),
        effect: CommandEffect::Accounts(AccountsVerb::Delete),
    },
    Command {
        name: "bills",
        domain: Domain::Bills,
        description: crate::msg::desktop_command_bills_description,
        binding: Some("g w"),
        effect: CommandEffect::Navigate(Noun::Bills),
    },
    Command {
        name: "budgets",
        domain: Domain::Budgets,
        description: crate::msg::desktop_command_budgets_description,
        binding: Some("g b"),
        effect: CommandEffect::Navigate(Noun::Budgets),
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
        name: "cash",
        domain: Domain::Cash,
        description: crate::msg::desktop_command_cash_description,
        binding: Some("g c"),
        effect: CommandEffect::Navigate(Noun::Cash),
    },
    Command {
        name: "categories",
        domain: Domain::Categories,
        description: crate::msg::desktop_command_categories_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsSection::Categories),
    },
    Command {
        name: "credit-cards",
        domain: Domain::CreditCards,
        description: crate::msg::desktop_command_credit_cards_description,
        binding: Some("g k"),
        effect: CommandEffect::Navigate(Noun::CreditCards),
    },
    Command {
        name: "documents",
        domain: Domain::Documents,
        description: crate::msg::desktop_command_documents_description,
        binding: Some("g f"),
        effect: CommandEffect::Navigate(Noun::Documents),
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
        name: "inventory",
        domain: Domain::Inventory,
        description: crate::msg::desktop_command_inventory_description,
        binding: Some("g o"),
        effect: CommandEffect::Navigate(Noun::Inventory),
    },
    Command {
        name: "investments",
        domain: Domain::Investments,
        description: crate::msg::desktop_command_investments_description,
        binding: Some("g i"),
        effect: CommandEffect::Navigate(Noun::Investments),
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
        name: "loans",
        domain: Domain::Loans,
        description: crate::msg::desktop_command_loans_description,
        binding: Some("g n"),
        effect: CommandEffect::Navigate(Noun::Loans),
    },
    Command {
        name: "notifications",
        domain: Domain::Notifications,
        description: crate::msg::desktop_command_notifications_description,
        binding: Some("g a"),
        effect: CommandEffect::Navigate(Noun::Notifications),
    },
    Command {
        name: "payees",
        domain: Domain::Payees,
        description: crate::msg::desktop_command_payees_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsSection::Payees),
    },
    Command {
        name: "reports",
        domain: Domain::Reports,
        description: crate::msg::desktop_command_reports_description,
        binding: Some("g r"),
        effect: CommandEffect::Navigate(Noun::Reports),
    },
    Command {
        name: "settings",
        domain: Domain::Settings,
        description: crate::msg::desktop_command_settings_description,
        binding: Some("g s"),
        effect: CommandEffect::Navigate(Noun::Settings),
    },
    settings_page_command("settings general", SettingsSection::General),
    settings_page_command("settings display", SettingsSection::Display),
    settings_page_command("settings units", SettingsSection::Units),
    settings_page_command("settings institutions", SettingsSection::Institutions),
    settings_page_command("settings accounts", SettingsSection::Accounts),
    settings_page_command("settings categories", SettingsSection::Categories),
    settings_page_command("settings tags", SettingsSection::Tags),
    settings_page_command("settings payees", SettingsSection::Payees),
    settings_page_command("settings documents", SettingsSection::Documents),
    settings_page_command("settings inventory", SettingsSection::Inventory),
    settings_page_command("settings sync-server", SettingsSection::SyncServer),
    settings_page_command("settings data-backup", SettingsSection::DataBackup),
    settings_page_command("settings tracing", SettingsSection::Tracing),
    settings_page_command("settings about", SettingsSection::About),
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
        effect: CommandEffect::OpenSettingsPage(SettingsSection::Tags),
    },
    Command {
        name: "tags merge",
        domain: Domain::Tags,
        description: crate::msg::desktop_command_tags_merge_description,
        binding: None,
        effect: CommandEffect::MergeTags,
    },
    Command {
        name: "dismiss",
        domain: Domain::Toasts,
        description: crate::msg::desktop_command_dismiss_description,
        binding: None,
        effect: CommandEffect::DismissToast,
    },
    Command {
        name: "dismiss all",
        domain: Domain::Toasts,
        description: crate::msg::desktop_command_dismiss_all_description,
        // The `dismiss_toasts` binding's default; a remap isn't reflected here.
        binding: Some("ctrl+l"),
        effect: CommandEffect::DismissAllToasts,
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
    Command {
        name: "toasts",
        domain: Domain::Toasts,
        description: crate::msg::desktop_command_toasts_description,
        binding: None,
        effect: CommandEffect::OpenToastHistory,
    },
    // `messages` is the alias the design names, for a user reaching for vim's `:messages`.
    Command {
        name: "messages",
        domain: Domain::Toasts,
        description: crate::msg::desktop_command_toasts_description,
        binding: None,
        effect: CommandEffect::OpenToastHistory,
    },
    Command {
        name: "import",
        domain: Domain::Transactions,
        description: crate::msg::desktop_command_import_description,
        binding: None,
        effect: CommandEffect::Import,
    },
    Command {
        name: "transactions",
        domain: Domain::Transactions,
        description: crate::msg::desktop_command_transactions_description,
        binding: Some("g l"),
        effect: CommandEffect::Navigate(Noun::Transactions),
    },
];

const fn settings_page_command(name: &'static str, section: SettingsSection) -> Command {
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

/// Every registered command, in registration order -- the palette's resting-state (empty
/// query) order, and the order ties fall back to once ranked.
pub fn all() -> impl Iterator<Item = &'static Command> {
    COMMANDS.iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_has_a_non_empty_name_and_description() {
        crate::locale::init_for_tests();
        for command in COMMANDS {
            assert!(!command.name.is_empty());
            assert!(!(command.description)().is_empty());
        }
    }

    #[test]
    fn every_settings_page_has_a_command() {
        for section in SettingsSection::ALL {
            let name = format!("settings {}", section.id());
            let effect = all().find(|c| c.name == name).map(|c| c.effect);
            assert_eq!(
                effect,
                Some(CommandEffect::OpenSettingsPage(section)),
                "{name}"
            );
        }
    }

    #[test]
    fn toasts_on_and_off_set_the_preference() {
        let effect = |name| all().find(|c| c.name == name).map(|c| c.effect);
        assert_eq!(effect("toasts on"), Some(CommandEffect::SetToasts(true)));
        assert_eq!(effect("toasts off"), Some(CommandEffect::SetToasts(false)));
    }

    #[test]
    fn the_budgets_verbs_carry_their_effects() {
        let effect = |name| all().find(|c| c.name == name).map(|c| c.effect);
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
    fn tags_merge_opens_the_merge_dialog() {
        let effect = |name| all().find(|c| c.name == name).map(|c| c.effect);
        assert_eq!(effect("tags merge"), Some(CommandEffect::MergeTags));
    }

    #[test]
    fn the_moved_noun_commands_alias_their_settings_page_with_no_binding() {
        for (name, section) in [
            ("accounts", SettingsSection::Accounts),
            ("categories", SettingsSection::Categories),
            ("payees", SettingsSection::Payees),
            ("tags", SettingsSection::Tags),
        ] {
            let command = COMMANDS.iter().find(|c| c.name == name).unwrap();
            assert_eq!(command.effect, CommandEffect::OpenSettingsPage(section));
            assert_eq!(command.binding, None, "{name} lost its g binding");
        }
    }

    #[test]
    fn toasts_and_messages_open_the_history() {
        let effect = |name| all().find(|c| c.name == name).map(|c| c.effect);
        assert_eq!(effect("toasts"), Some(CommandEffect::OpenToastHistory));
        assert_eq!(effect("messages"), Some(CommandEffect::OpenToastHistory));
    }

    #[test]
    fn every_noun_has_a_navigate_command() {
        for noun in Noun::ALL {
            assert!(
                COMMANDS
                    .iter()
                    .any(|c| c.effect == CommandEffect::Navigate(noun)),
                "no command carries CommandEffect::Navigate({noun:?})"
            );
        }
    }

    #[test]
    fn domains_are_contiguous_dashboard_first_then_alphabetical() {
        // Mirrors bin-tui's own `dashboard_is_first_and_the_rest_are_alphabetical` invariant --
        // the palette's resting-state grouping (`Palette::rows`) assumes each domain's commands
        // sit together, never split across two separate runs.
        let mut order: Vec<Domain> = Vec::new();
        for command in COMMANDS {
            if order.last() != Some(&command.domain) {
                assert!(
                    !order.contains(&command.domain),
                    "domain {:?} is split across non-adjacent commands",
                    command.domain
                );
                order.push(command.domain);
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
