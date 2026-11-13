//! The command registry driving `lib-shell`'s palette Presenter (`lib_shell::palette`)
//! (`docs/ux/desktop-mockups/01-shell/README.md`'s "Command registry" section): "every rail
//! item, every context-rail footer affordance, and every view action registers a command with:
//! command string, description, kind ... optional binding, and a handler." Mirrors the shape of
//! `bin-tui`'s own per-domain registry (`crates/bins/bin-tui/src/popup/command/commands/`,
//! `docs/ux/tui-mockups/navigation.md`'s "Commands" section) -- plain, hand-authored data, one flat
//! array rather than domain modules (the desktop registry is small enough not to need
//! splitting the way the TUI's much larger one does), but each `Command` now carries a
//! `group` field so the palette's own resting-state list can group by it the same way.
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
//! [`entries`] lists each `Listed` one with its live key. A module's come from the pages `Root`
//! hosts, each page's keys as it holds them now (`PageHandle::keys`, #1169), so a new page is listed
//! without a line here, and only while it is hosted: a This Ledger page's Commands with a Ledger
//! open. Running one goes back through its keymap by name ([`CommandEffect::Shell`],
//! [`CommandEffect::Module`]), so the palette and the key can't drift. [`COMMANDS`] keeps only the
//! Commands no keymap has.

use crate::navigation::nav::NounLabel;
use lib_colour_theme::ColourAppearance;
use lib_config::KeyBindingConfig;
use lib_settings::ColourChoice;
use lib_shell::{Destination, PaletteEffect, PaletteEntry, ShellCommand};
use lib_viper::keymap::Palette;

use crate::navigation::active_view::ActiveView;
use crate::shell::view::page_handle::PageBinding;
use crate::{navigation::nav::Noun, settings::SettingsPage};
use lib_shell::ExplorerMode;

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
    /// page and opens the matching dialog, resolving the typed name the palette split off.
    Accounts(AccountsVerb),
    /// Sets the `colour_theme` or `colour_appearance` Preference, the same as picking it in
    /// Settings' Colour Theme group.
    Colour(ColourChoice),
    /// `:dismiss`: removes the newest Toast.
    DismissToast,
    /// `:toasts on` / `:toasts off`: sets the Toasts Preference, the same as Settings' Toasts row.
    SetToasts(bool),
    /// `:toasts test`: raises the Toasts test, the same as Settings › General's **Test**.
    TestToasts,
    /// `:import`: opens the stubbed 6e Import "match payees" step on the seeded statement.
    Import,
    /// A Command the Shell's global keymap lists: runs as its key does.
    Shell(ShellCommand),
    /// A Command a module's keymap lists: opens `page`, whose module Router sends the event its
    /// keymap binds `command` to, as the key would, on whatever row the page last had selected.
    Module {
        page: ActiveView,
        command: &'static str,
    },
    /// No real behaviour behind this command yet (`docs/ux/tui-mockups/01-shell/README.md`'s commitment: "a
    /// command that has no real behaviour yet says so explicitly when run") --
    /// `Root::run_command` turns this into the status-line flash.
    NotYetBuilt,
}

impl CommandEffect {
    /// Whether the effect needs a Ledger open, so it is refused with none (#977): one that opens a
    /// This Ledger page, a noun that needs one or a module's page but a This device Settings page,
    /// and `:close`, `:import` and the commands not yet built. `:open`, `:new`, Help, the This
    /// device Settings pages and the Shell's and Preferences' own commands run with no Ledger open.
    pub fn needs_ledger(&self) -> bool {
        match self {
            Self::OpenSettingsPage(section)
            | Self::Module {
                page: ActiveView::Settings(section),
                ..
            } => section.needs_ledger(),
            Self::Shell(ShellCommand::Jump(noun)) => crate::navigation::nav::needs_ledger(*noun),
            Self::CloseLedger
            | Self::Accounts(_)
            | Self::Import
            | Self::Module { .. }
            | Self::NotYetBuilt => true,
            Self::OpenDialog(_)
            | Self::Colour(_)
            | Self::DismissToast
            | Self::SetToasts(_)
            | Self::TestToasts
            | Self::Shell(_) => false,
        }
    }

    /// The effect as the Shell's Presenter runs it: the Shell's own (a global key's Command, the
    /// file explorer, `:dismiss`, `:close`) it runs itself, and the rest it hands back to the root
    /// Router once their destination shows, as their key would.
    pub fn palette_effect(self) -> PaletteEffect<SettingsPage, Self> {
        let destination = match self {
            Self::Shell(command) => return PaletteEffect::Shell(command),
            Self::OpenDialog(mode) => return PaletteEffect::OpenExplorer(mode),
            Self::DismissToast => return PaletteEffect::DismissNewestToast,
            Self::CloseLedger => return PaletteEffect::CloseLedger,
            Self::OpenSettingsPage(section) => Some(Destination::Settings(section)),
            Self::Accounts(_) => Some(Destination::Settings(SettingsPage::Accounts)),
            Self::Import => Some(Destination::Noun(Noun::Transactions)),
            Self::Module { page, .. } => Some(page.destination()),
            Self::Colour(_) | Self::SetToasts(_) | Self::TestToasts | Self::NotYetBuilt => None,
        };
        PaletteEffect::Client {
            destination,
            needs_ledger: self.needs_ledger(),
            effect: self,
        }
    }

    /// Whether text typed after the command name is an argument (`accounts delete Home Loan`)
    /// rather than more of the query, which the palette's Presenter reads from each entry. A
    /// keymap event can't carry an argument, so only the Accounts verbs do.
    pub fn takes_argument(&self) -> bool {
        matches!(self, Self::Accounts(_))
    }
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

/// The palette's resting-state group a command sits under: one per noun or module, plus `Ledger`
/// for the file-level `open`/`new`/`close` trio and `Navigation` for the Shell's own keys, which
/// have no noun. The header text is a Message; the variants are the stable ids, declared in the
/// palette's order (`Dashboard` first, then alphabetical), which `Ord` follows.
///
/// It groups the palette's rows and is not a store, so it stays apart from `lib_database::Domain`,
/// which names what a write touched: half its groups (the placeholder nouns, `Ledger`,
/// `Navigation`, `Toasts`, `Settings`) have no store, and a store's domain gains no palette group.
/// A page's group is [`group`]'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandGroup {
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

impl CommandGroup {
    /// The header text in the Locale in effect, in sentence case.
    pub fn label(self) -> String {
        match self {
            CommandGroup::Dashboard => Noun::Dashboard.label(),
            CommandGroup::Accounts => SettingsPage::Accounts.label(),
            CommandGroup::Bills => Noun::Bills.label(),
            CommandGroup::Budgets => Noun::Budgets.label(),
            CommandGroup::Cash => Noun::Cash.label(),
            CommandGroup::Categories => SettingsPage::Categories.label(),
            CommandGroup::CreditCards => Noun::CreditCards.label(),
            CommandGroup::Documents => Noun::Documents.label(),
            CommandGroup::Institutions => SettingsPage::Institutions.label(),
            CommandGroup::Inventory => Noun::Inventory.label(),
            CommandGroup::Investments => Noun::Investments.label(),
            CommandGroup::Ledger => crate::msg::desktop_command_domain_ledger(),
            CommandGroup::Loans => Noun::Loans.label(),
            CommandGroup::Navigation => crate::msg::desktop_command_domain_navigation(),
            CommandGroup::Notifications => Noun::Notifications.label(),
            CommandGroup::Payees => SettingsPage::Payees.label(),
            CommandGroup::Reports => Noun::Reports.label(),
            CommandGroup::Settings => Noun::Settings.label(),
            CommandGroup::Tags => SettingsPage::Tags.label(),
            CommandGroup::Toasts => crate::msg::desktop_command_domain_toasts(),
            CommandGroup::Transactions => Noun::Transactions.label(),
            CommandGroup::Units => SettingsPage::Units.label(),
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
    pub group: CommandGroup,
    /// The description Message, resolved in the Locale in effect when called.
    pub description: fn() -> String,
    pub binding: Option<&'static str>,
    pub effect: CommandEffect,
}

impl Command {
    /// Whether text typed after the command name is an argument (`accounts delete Home Loan`)
    /// rather than more of the query.
    pub fn takes_argument(&self) -> bool {
        self.effect.takes_argument()
    }
}

/// The Commands no keymap has, which the palette lists beside the keymaps' ([`entries`]): the
/// noun aliases with no `g` jump (`accounts`, `categories`, `payees`, `tags`), the three `accounts
/// <verb>` commands, which take a typed account name a keymap event can't carry, the file-level `open`/`new`/`close` trio, each `settings <page>`, the Colour Themes, and the Toast
/// commands `dismiss`, `toasts on|off|test` and `messages` (an alias of the `toasts` binding). A
/// Command a keymap lists has no row here (`no_commands_row_is_a_command_a_keymap_lists`).
pub const COMMANDS: &[Command] = &[
    Command {
        name: "accounts",
        group: CommandGroup::Accounts,
        description: crate::msg::desktop_command_accounts_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsPage::Accounts),
    },
    Command {
        name: "accounts new",
        group: CommandGroup::Accounts,
        description: accounts_new_description,
        binding: None,
        effect: CommandEffect::Accounts(AccountsVerb::New),
    },
    Command {
        name: "accounts edit",
        group: CommandGroup::Accounts,
        description: crate::msg::desktop_command_accounts_edit_description,
        binding: None,
        effect: CommandEffect::Accounts(AccountsVerb::Edit),
    },
    Command {
        name: "accounts delete",
        group: CommandGroup::Accounts,
        description: crate::msg::desktop_command_accounts_delete_description,
        binding: None,
        effect: CommandEffect::Accounts(AccountsVerb::Delete),
    },
    Command {
        name: "categories",
        group: CommandGroup::Categories,
        description: crate::msg::desktop_command_categories_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsPage::Categories),
    },
    Command {
        name: "open",
        group: CommandGroup::Ledger,
        description: crate::msg::desktop_command_open_description,
        binding: None,
        effect: CommandEffect::OpenDialog(ExplorerMode::Open),
    },
    Command {
        name: "new",
        group: CommandGroup::Ledger,
        description: crate::msg::desktop_command_new_description,
        binding: None,
        effect: CommandEffect::OpenDialog(ExplorerMode::New),
    },
    Command {
        name: "close",
        group: CommandGroup::Ledger,
        description: crate::msg::desktop_command_close_description,
        binding: None,
        effect: CommandEffect::CloseLedger,
    },
    Command {
        name: "payees",
        group: CommandGroup::Payees,
        description: crate::msg::desktop_command_payees_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsPage::Payees),
    },
    settings_page_command("settings general", SettingsPage::General),
    settings_page_command("settings ledger", SettingsPage::Ledger),
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
        name: "units",
        group: CommandGroup::Units,
        description: crate::msg::desktop_command_units_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsPage::Units),
    },
    Command {
        name: "institutions",
        group: CommandGroup::Institutions,
        description: crate::msg::desktop_command_institutions_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsPage::Institutions),
    },
    Command {
        name: "tags",
        group: CommandGroup::Tags,
        description: crate::msg::desktop_command_tags_description,
        binding: None,
        effect: CommandEffect::OpenSettingsPage(SettingsPage::Tags),
    },
    Command {
        name: "dismiss",
        group: CommandGroup::Toasts,
        description: crate::msg::desktop_command_dismiss_description,
        binding: None,
        effect: CommandEffect::DismissToast,
    },
    Command {
        name: "toasts on",
        group: CommandGroup::Toasts,
        description: crate::msg::desktop_command_toasts_on_description,
        binding: None,
        effect: CommandEffect::SetToasts(true),
    },
    Command {
        name: "toasts off",
        group: CommandGroup::Toasts,
        description: crate::msg::desktop_command_toasts_off_description,
        binding: None,
        effect: CommandEffect::SetToasts(false),
    },
    Command {
        name: "toasts test",
        group: CommandGroup::Toasts,
        description: crate::msg::desktop_command_toasts_test_description,
        binding: None,
        effect: CommandEffect::TestToasts,
    },
    // `messages` is the alias the design names, for a user reaching for vim's `:messages`.
    Command {
        name: "messages",
        group: CommandGroup::Toasts,
        description: lib_locale::msg::command_toast_history,
        binding: None,
        effect: CommandEffect::Shell(ShellCommand::ToastHistory),
    },
    Command {
        name: "import",
        group: CommandGroup::Transactions,
        description: crate::msg::desktop_command_import_description,
        binding: None,
        effect: CommandEffect::Import,
    },
];

const fn settings_page_command(name: &'static str, section: SettingsPage) -> Command {
    Command {
        name,
        group: CommandGroup::Settings,
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
        group: CommandGroup::Settings,
        description,
        binding: None,
        effect: CommandEffect::Colour(ColourChoice::Theme(id)),
    }
}

const fn colour_appearance_command(
    name: &'static str,
    appearance: ColourAppearance,
    description: fn() -> String,
) -> Command {
    Command {
        name,
        group: CommandGroup::Settings,
        description,
        binding: None,
        effect: CommandEffect::Colour(ColourChoice::Appearance(appearance)),
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
    pub group: CommandGroup,
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
            group: command.group,
            description: command.description,
            key: live,
            effect: command.effect,
        }
    }
}

/// Every Command the palette lists under `config`: the global keymap's, [`COMMANDS`], then those
/// each hosted page in `pages` lists, by its keys as it holds them now, grouped by
/// [`CommandGroup`] in its order and otherwise in that order -- the palette's resting-state (empty
/// query) order, and the order ties fall back to once ranked.
pub fn entries(
    config: &KeyBindingConfig,
    pages: impl IntoIterator<Item = (ActiveView, Vec<PageBinding>)>,
) -> Vec<Entry> {
    let mut entries = global_entries(config);
    entries.extend(
        COMMANDS
            .iter()
            .map(|command| Entry::from_command(command, config)),
    );
    entries.extend(module_entries(pages));
    entries.sort_by_key(|entry| entry.group);
    entries
}

/// [`entries`] as the palette's Presenter lists them, each under its group's header.
pub fn palette_entries(
    config: &KeyBindingConfig,
    pages: impl IntoIterator<Item = (ActiveView, Vec<PageBinding>)>,
) -> Vec<PaletteEntry<PaletteEffect<SettingsPage, CommandEffect>>> {
    entries(config, pages)
        .into_iter()
        .map(|entry| PaletteEntry {
            name: entry.name,
            description: entry.description,
            domain: entry.group.label(),
            takes_argument: entry.takes_argument(),
            key: entry.key,
            effect: entry.effect.palette_effect(),
        })
        .collect()
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
                group: global_group(command),
                description: listed.description,
                key: key(listed.chord),
                effect: CommandEffect::Shell(command),
            })
        })
        .collect()
}

fn global_group(command: ShellCommand) -> CommandGroup {
    match command {
        ShellCommand::Jump(noun) => noun_group(noun),
        ShellCommand::DismissToasts | ShellCommand::ToastHistory => CommandGroup::Toasts,
        // Beside `:open`, `:new` and `:close`, the other file-level Commands.
        ShellCommand::ChangeDataFolder => CommandGroup::Ledger,
        _ => CommandGroup::Navigation,
    }
}

fn noun_group(noun: Noun) -> CommandGroup {
    match noun {
        Noun::Dashboard => CommandGroup::Dashboard,
        Noun::Transactions => CommandGroup::Transactions,
        Noun::Documents => CommandGroup::Documents,
        Noun::Notifications => CommandGroup::Notifications,
        Noun::Cash => CommandGroup::Cash,
        Noun::Inventory => CommandGroup::Inventory,
        Noun::Loans => CommandGroup::Loans,
        Noun::CreditCards => CommandGroup::CreditCards,
        Noun::Investments => CommandGroup::Investments,
        Noun::Bills => CommandGroup::Bills,
        Noun::Budgets => CommandGroup::Budgets,
        Noun::Reports => CommandGroup::Reports,
        Noun::Settings => CommandGroup::Settings,
    }
}

/// The group a page's Commands sit under: its module's, so a Settings page that is a module's View
/// sits with that module, and the This device and info pages under Settings.
pub fn group(view: ActiveView) -> CommandGroup {
    match view {
        ActiveView::Dashboard => CommandGroup::Dashboard,
        ActiveView::Transactions | ActiveView::Import => CommandGroup::Transactions,
        ActiveView::Documents | ActiveView::Settings(SettingsPage::Documents) => {
            CommandGroup::Documents
        }
        ActiveView::Bills => CommandGroup::Bills,
        ActiveView::Budgets => CommandGroup::Budgets,
        ActiveView::Settings(SettingsPage::Units) => CommandGroup::Units,
        ActiveView::Settings(SettingsPage::Institutions) => CommandGroup::Institutions,
        ActiveView::Settings(SettingsPage::Accounts) => CommandGroup::Accounts,
        ActiveView::Settings(SettingsPage::Categories) => CommandGroup::Categories,
        ActiveView::Settings(SettingsPage::Tags) => CommandGroup::Tags,
        ActiveView::Settings(SettingsPage::Payees) => CommandGroup::Payees,
        ActiveView::Settings(SettingsPage::Inventory) => CommandGroup::Inventory,
        ActiveView::Settings(
            SettingsPage::General
            | SettingsPage::Ledger
            | SettingsPage::Display
            | SettingsPage::SyncServer
            | SettingsPage::DataBackup
            | SettingsPage::Tracing
            | SettingsPage::About,
        ) => CommandGroup::Settings,
        ActiveView::Placeholder(noun) => noun_group(noun),
    }
}

/// The Commands each hosted page lists, by its keys as it holds them now (a rebind's chord
/// included), each running on its page and grouped by [`group`]. A Command a page holds in more
/// than one keymap (a tab's and the page's) is listed once, by the first.
fn module_entries(pages: impl IntoIterator<Item = (ActiveView, Vec<PageBinding>)>) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    for (page, keys) in pages {
        for binding in keys {
            let Palette::Listed { name, description } = binding.palette else {
                continue;
            };
            if entries.iter().any(|entry| entry.name == name) {
                continue;
            }
            entries.push(Entry {
                name,
                group: group(page),
                description,
                key: key(&binding.chord),
                effect: CommandEffect::Module {
                    page,
                    command: binding.command,
                },
            });
        }
    }
    entries
}

/// A live chord as the palette shows it: an empty one is unbound.
fn key(chord: &str) -> Option<String> {
    (!chord.is_empty()).then(|| chord.to_string())
}

/// What the [`COMMANDS`] row named `name` does: the empty state's `open` and `new` links, which
/// no keymap binds.
pub fn find(name: &str) -> Option<CommandEffect> {
    COMMANDS
        .iter()
        .find(|command| command.name == name)
        .map(|command| command.effect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SettingsPage as Page;

    /// The module pages a Ledger open hosts, each with its page keymap's keys under `config`: what
    /// `Root` hands [`entries`], without building the pages.
    fn hosted(config: &KeyBindingConfig) -> Vec<(ActiveView, Vec<PageBinding>)> {
        vec![
            (
                ActiveView::Settings(Page::Accounts),
                PageBinding::all_of(&lib_accounts::keymap::accounts_page_keymap(config)),
            ),
            (
                ActiveView::Bills,
                PageBinding::all_of(&lib_bills::keymap::bills_page_keymap(config)),
            ),
            (
                ActiveView::Budgets,
                PageBinding::all_of(&lib_budgets::keymap::budgets_page_keymap(config)),
            ),
            (
                ActiveView::Settings(Page::Categories),
                PageBinding::all_of(&lib_categories::keymap::categories_page_keymap(config)),
            ),
            (
                ActiveView::Documents,
                PageBinding::all_of(&lib_documents::keymap::documents_page_keymap(config)),
            ),
            (
                ActiveView::Settings(Page::Documents),
                PageBinding::all_of(&lib_documents::keymap::document_types_page_keymap(config)),
            ),
            (
                ActiveView::Settings(Page::Institutions),
                PageBinding::all_of(&lib_institutions::keymap::institutions_page_keymap(config)),
            ),
            (
                ActiveView::Settings(Page::Inventory),
                PageBinding::all_of(&lib_inventory::keymap::inventory_page_keymap(config)),
            ),
            (
                ActiveView::Settings(Page::Payees),
                PageBinding::all_of(&lib_payees::keymap::payees_page_keymap(config)),
            ),
            (
                ActiveView::Settings(Page::Tags),
                PageBinding::all_of(&lib_tags::keymap::tags_page_keymap(config)),
            ),
            (
                ActiveView::Transactions,
                PageBinding::all_of(&lib_transactions::keymap::transactions_page_keymap(config)),
            ),
            (
                ActiveView::Settings(Page::Units),
                PageBinding::all_of(&lib_units::keymap::units_page_keymap(config)),
            ),
        ]
    }

    fn entries_under(config: &KeyBindingConfig) -> Vec<Entry> {
        entries(config, hosted(config))
    }

    fn defaults() -> Vec<Entry> {
        entries_under(&KeyBindingConfig::default())
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
            .chain(module_entries(hosted(&config)))
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
            entries_under(config)
                .into_iter()
                .find(|entry| entry.name == name)
                .and_then(|entry| entry.key)
        };
        let defaults = KeyBindingConfig::default();
        assert_eq!(key(&defaults, "dashboard").as_deref(), Some("g d"));
        assert_eq!(key(&defaults, "units set-base").as_deref(), Some("B"));
        assert_eq!(key(&defaults, "toasts"), None, "toast_history is unbound");

        let config = KeyBindingConfig::overriding("jump_dashboard", "g h");
        assert_eq!(key(&config, "dashboard").as_deref(), Some("g h"));

        let config = KeyBindingConfig::overriding("new", "a").and("toast_history", "ctrl+t");
        assert_eq!(key(&config, "accounts new").as_deref(), Some("a"));
        assert_eq!(key(&config, "messages").as_deref(), Some("ctrl+t"));
        assert_eq!(key(&config, "toasts").as_deref(), Some("ctrl+t"));
    }

    #[test]
    fn data_changes_the_data_folder_beside_the_ledger_commands_with_or_without_a_ledger() {
        crate::locale::init_for_tests();
        let data = defaults()
            .into_iter()
            .find(|entry| entry.name == "data")
            .expect("`data` is listed");

        assert_eq!(
            data.effect,
            CommandEffect::Shell(ShellCommand::ChangeDataFolder)
        );
        assert_eq!((data.description)(), "Change data folder");
        assert_eq!(data.group, CommandGroup::Ledger);
        assert_eq!(data.key, None, "change_data_folder is unbound");
        assert!(!data.effect.needs_ledger());
    }

    #[test]
    fn a_module_command_runs_on_its_page_and_a_global_one_as_its_key() {
        assert_eq!(
            effect("units set-base"),
            Some(CommandEffect::Module {
                page: ActiveView::Settings(SettingsPage::Units),
                command: lib_units::keymap::SET_BASE,
            })
        );
        assert_eq!(
            effect("bills pay"),
            Some(CommandEffect::Module {
                page: ActiveView::Bills,
                command: lib_bills::keymap::PAY,
            })
        );
        assert_eq!(
            effect("tags merge"),
            Some(CommandEffect::Module {
                page: ActiveView::Settings(SettingsPage::Tags),
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
        for entry in module_entries(hosted(&KeyBindingConfig::default())) {
            let CommandEffect::Module { page, command } = entry.effect else {
                panic!("{} is a module's but runs no module Command", entry.name);
            };
            assert_ne!(
                page,
                ActiveView::Settings(SettingsPage::Accounts),
                "Accounts' verbs take a name, so only COMMANDS lists them"
            );
            assert!(!hidden.contains(&command), "{command}");
        }
    }

    #[test]
    fn a_page_lists_its_commands_under_its_live_keys_once_each() {
        let config = KeyBindingConfig::overriding(lib_units::keymap::SET_BASE, "ctrl+b");
        let keymap = lib_units::keymap::units_page_keymap(&config);
        let mut keys = PageBinding::all_of(&keymap);
        // A second keymap holding the same Command, as a tab's beside its page's.
        keys.extend(PageBinding::all_of(&keymap));

        let listed = module_entries([(ActiveView::Settings(Page::Units), keys)]);

        let set_base: Vec<_> = listed
            .iter()
            .filter(|entry| entry.name == "units set-base")
            .collect();
        assert_eq!(set_base.len(), 1, "listed once");
        assert_eq!(set_base[0].key.as_deref(), Some("ctrl+b"));
        assert_eq!(set_base[0].group, CommandGroup::Units);
        assert_eq!(
            set_base[0].effect,
            CommandEffect::Module {
                page: ActiveView::Settings(Page::Units),
                command: lib_units::keymap::SET_BASE,
            }
        );
    }

    #[test]
    fn no_page_hosted_lists_no_module_command() {
        let config = KeyBindingConfig::default();
        let listed = entries(&config, []);
        assert!(
            listed
                .iter()
                .all(|entry| !matches!(entry.effect, CommandEffect::Module { .. })),
            "a module Command is listed with no page hosting it"
        );
        assert!(listed.iter().any(|entry| entry.name == "settings general"));
    }

    #[test]
    fn a_this_device_page_command_runs_without_a_ledger_and_a_module_one_needs_one() {
        let on = |page| CommandEffect::Module {
            page,
            command: lib_viper::keymap::EDIT,
        };
        assert!(!on(ActiveView::Settings(Page::Display)).needs_ledger());
        assert!(on(ActiveView::Settings(Page::Units)).needs_ledger());
        assert!(on(ActiveView::Bills).needs_ledger());
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
    fn toasts_test_raises_the_test_with_or_without_a_ledger() {
        assert_eq!(effect("toasts test"), Some(CommandEffect::TestToasts));
        assert!(!CommandEffect::TestToasts.needs_ledger());
    }

    #[test]
    fn the_budgets_verbs_run_on_the_budgets_page_by_their_keymap_command() {
        use lib_budgets::keymap as budgets;
        for (name, command) in [
            ("budgets switch", budgets::SWITCH),
            ("budgets new", lib_viper::keymap::NEW),
            ("budgets edit", budgets::EDIT_BUDGET),
            ("budgets manage", budgets::MANAGE),
            ("budgets duplicate", budgets::DUPLICATE),
            ("budgets set-default", budgets::SET_DEFAULT),
            ("budgets archive", budgets::ARCHIVE),
            ("budgets restore", budgets::RESTORE),
        ] {
            assert_eq!(
                effect(name),
                Some(CommandEffect::Module {
                    page: ActiveView::Budgets,
                    command,
                }),
                "{name}"
            );
        }
    }

    #[test]
    fn the_moved_noun_commands_alias_their_settings_page_with_no_binding() {
        for (name, section) in [
            ("accounts", SettingsPage::Accounts),
            ("categories", SettingsPage::Categories),
            ("payees", SettingsPage::Payees),
            ("tags", SettingsPage::Tags),
            ("units", SettingsPage::Units),
            ("institutions", SettingsPage::Institutions),
        ] {
            let command = COMMANDS.iter().find(|c| c.name == name).unwrap();
            assert_eq!(command.effect, CommandEffect::OpenSettingsPage(section));
            assert_eq!(command.binding, None, "{name} lost its g binding");
        }
    }

    #[test]
    fn every_module_settings_page_has_a_noun_row_ranking_ahead_of_its_keymap_commands() {
        // The palette ranks by name prefix and keeps `entries` order for ties, so the bare noun
        // must be listed before `<noun> new`, or Enter on `:units` opens a Dialog, not the page.
        let entries = defaults();
        for (index, entry) in entries.iter().enumerate() {
            let CommandEffect::Module {
                page: ActiveView::Settings(page),
                ..
            } = entry.effect
            else {
                continue;
            };
            let noun = entries
                .iter()
                .position(|row| row.name == page.id())
                .unwrap_or_else(|| panic!("{} has no `{}` noun row", entry.name, page.id()));
            assert!(
                noun < index,
                "`{}` ranks ahead of the `{}` noun row",
                entry.name,
                page.id()
            );
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
    fn groups_are_contiguous_dashboard_first_then_alphabetical() {
        // Mirrors bin-tui's own `dashboard_is_first_and_the_rest_are_alphabetical` invariant --
        // the palette's resting-state grouping (`lib_shell::palette`) assumes each domain's commands
        // sit together, never split across two separate runs.
        let mut order: Vec<CommandGroup> = Vec::new();
        for entry in defaults() {
            if order.last() != Some(&entry.group) {
                assert!(
                    !order.contains(&entry.group),
                    "group {:?} is split across non-adjacent commands",
                    entry.group
                );
                order.push(entry.group);
            }
        }
        assert_eq!(order[0], CommandGroup::Dashboard);
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
                    .any(|c| c.effect == CommandEffect::Colour(ColourChoice::Theme(theme.id))),
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
                    |c| c.effect == CommandEffect::Colour(ColourChoice::Appearance(appearance))
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
