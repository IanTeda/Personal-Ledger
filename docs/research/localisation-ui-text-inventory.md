# Research: Inventory of hardcoded UI text in bin-desktop and bin-tui

**Question.** Issue [#215](https://github.com/IanTeda/Personal-Ledger/issues/215), part of the [Localisation of the Desktop and TUI UX](https://github.com/IanTeda/Personal-Ledger/issues/211) map, asks for an inventory of user-visible text in `crates/bins/bin-desktop/src` and `crates/bins/bin-tui/src` (plus `lib-*` text that can reach the screen), classified by how hard each kind of string is to turn into a Message, with counts per category and per file, the worst patterns to migrate, and where the two bins already share wording. This replaces the map's rough figure of "about 180 literal-string UI call sites". It is research input for the message-scope decision and the per-screen migration tickets; it makes no decision. Vocabulary (Locale, Message, Catalogue, Account Kind, Transaction Status) follows `CONTEXT.md`.

**Method and caveats.** The audit ran against the `concept` branch at `ba0b53e`, because `main` still carries the old six-screen feasibility demo. A script extracted every string literal from non-test code (everything before the first `#[cfg(test)]` in each file) under both bins and the `lib-*` crates, then classified each literal with regular expressions on the literal and its line, and the results were spot-checked by reading. Counts are therefore **string literals, not call sites**, and the category boundaries are heuristic: expect roughly ten per cent of literals to land in a neighbouring category. Multi-line SQL literals in `lib-database` were excluded because they are not UI. `lib-database`, `lib-core` and the other libs were audited by reading their error enums and enum label methods rather than by literal count.

## 1. Headline numbers

The map's estimate of about 180 call sites is low by roughly an order of magnitude. Counting literals, there are about 1,690 translatable-looking strings across the two bins, plus about 460 lowercase single-word literals that are a mix of labels and machine tokens and need per-site review.

| Bucket | bin-desktop | bin-tui | Total |
|---|---|---|---|
| Static labels and headings | 249 | 602 | 851 |
| Text with runtime arguments (`{}` placeholders) | 81 | 136 | 217 |
| Keybinding hints and key names | 110 | 164 | 274 |
| Palette command names, usage and help | 45 | 190 | 235 |
| Error, validation and status messages | 25 | 85 | 110 |
| **Subtotal: candidate Messages** | **510** | **1,177** | **1,687** |
| Single lowercase tokens (labels or machine tokens) | 181 | 276 | 457 |
| Mock or fixture data (dummy Ledger data, not Messages) | 288 | 120 | 408 |
| Glyphs, separators and layout literals | 23 | 25 | 48 |
| Non-UI: element ids, icon paths, config keys, file names | 149 | 36 | 185 |
| Non-UI: tracing, panics, attributes, `expect` text | 25 | 80 | 105 |
| **Total literals inspected** | **1,176** | **1,714** | **2,890** |

The TUI carries more than twice the desktop's text because it has a screen and popup for every entity (Accounts, Categories, Payees, Tags, Units, Budgets, Balance Checks, Reports, Transactions) and spends prose on inline key hints, whereas the desktop has so far built the Accounts, Transactions, Settings and Dashboard pages plus the shell chrome. Expect the desktop count to grow towards the TUI's as its remaining pages are built, which argues for migrating text to Messages as each desktop page is built rather than as a later sweep.

## 2. Classification and worked examples

### 2.1 Static labels and headings (851)

Column headers, section headings, form field labels, button labels, tab and rail names. In the desktop these are mostly `.child("...")` and `Column::new(..., "...")` literals, for example `"Add account"`, `"Opening balance"` (`view/accounts/add_dialog.rs`), `"NAME"`, `"INSTITUTION"` (`view/accounts/mod.rs`). In the TUI they are `Span::raw`, `Line::from` and `field_line("Kind", ...)` arguments, for example `"Balance Checks"`, `" Edit Transaction "` (`screen/transaction_detail.rs`).

Two migration hazards sit inside this category. First, **case is baked in at the call site**: headings such as `"NAME"`, `"NET POSITION"` and `"NEEDS ATTENTION"` are stored upper-case, while the TUI lower-cases dates and upper-cases labels at render time (98 calls to `to_uppercase()`/`to_lowercase()`/`to_ascii_*case()` across the two bins: 25 desktop, 73 TUI). A Message should carry its own casing, or the presentation layer should apply it with Locale-aware rules. Second, **padded layout depends on English length**: 18 TUI sites use width specifiers such as `{label:<16}` or `{label:<SUMMARY_LABEL_WIDTH$}`, and the desktop hardcodes pixel widths beside column labels (`.w(px(140.0))`), so a longer translation, or the `en-XA` pseudo-Locale, will truncate or overflow.

### 2.2 Text with runtime arguments (217)

Strings with `{}` or named placeholders, for example `"Delete account \u{2014} {}"`, `"{} of {total} \u{b7} newest first"`, `"Failed to load Categories: {message}"`, `":{command_name} \u{2014} no account named \"{argument}\""`. Fluent variables map onto these directly, but four things need care: the argument order is fixed by `format!` (translators need named variables, and most of these use positional `{}`), the arguments are frequently pre-formatted numbers, dates or money (see section 3.3), the arguments are sometimes themselves English fragments (`"{} {name}"` in `format.rs` combines a glyph with an English status word), and about 35 of these strings combine a count with a noun (see section 3.1).

### 2.3 Keybinding hints and key names (274)

Two different things live here and they localise differently.

- **Key names and chords as data**: `"esc"`, `"enter"`, `"tab"`, `"^s"`, `"g d"`, single letters in the router table (`bin-desktop/src/key_router.rs`, 24 hits in `shell.rs` alone), and the help overlay's shortcut table (`bin-desktop/src/help.rs`, `GLOBAL_SHORTCUTS`: `("Command palette", ":")`, `("Open / confirm", "enter")`). The key itself is not translated, but the mapping from chord to human-readable name may be, and the chords come from `lib-config`'s key bindings (`lib-config/src/keybindings.rs`), so the hint should be generated from the binding, not written beside it.
- **Hint prose mixing keys and words**: `"esc close command window"` (`bin-desktop/src/shell.rs:358`), `"\u{2191}\u{2193} select \u{b7} tab complete \u{b7} enter run \u{b7} ^r history \u{b7} esc close"` (`palette.rs:420`), `" : command \u{b7} / search \u{b7} ? help \u{b7} esc close command"` (`bin-tui/src/shell.rs:1615`), and TUI screen footers such as `"Tab/Shift+Tab: next/prev field  \u{2190}/\u{2192}: change Category/Unit/Period/Active  Enter: save  Esc: cancel"` (`screen/budget_detail.rs:450`). These are one Message each only if the key glyphs are passed in as arguments; as written they interleave key tokens and English verbs in a single string, so a translator would be shown the key names as part of the text.

The TUI has a separate family of near-duplicate hint strings: `bin-tui/src/shell.rs:1618-1632` builds `" esc close unit form "`, `" esc close edit form "`, `" esc close account form "`, `" esc close payee form "`, `" esc close tag form "`, `" esc close dialog "` and `" esc close category form "` as separate literals. That is one Message with a noun argument, written seven times.

### 2.4 Palette commands, usage and help (235)

The palette command registry is where names and descriptions are most tightly coupled to English. Desktop: `bin-desktop/src/command.rs` (45 literals) defines each command's title, one-line description and `g x` shortcut, for example `"accounts new"` with `"add an account: accounts new <account name>"`. TUI: `bin-tui/src/popup/command/commands/*.rs` (190 literals across 12 files) defines, per command, a usage line (`"account delete <acct> [into <acct>]"`), a description, an argument placeholder (`"<acct>"`) and a worked example (`"Everyday Spending \u{b7} 1 284 txns \u{2014} needs [into <acct>]"`).

Decisions this forces for the message-scope ticket: are the command **names** themselves typed by the user (in which case they are an interface, closer to config keys, and probably stay English or need aliases per Locale) or only their **descriptions** (Messages)? Argument placeholders such as `<acct>` and example values such as `Woolworths` or `Everyday Spending` embed Ledger-data-shaped English. The usage lines also embed grammar (`[into <acct>]`) that is part of the command syntax, not prose. The TUI also uses the display string as a **dispatch key**: `bin-tui/src/shell.rs:356-502` matches on `selected_command_name()` against literals such as `Some("unit new <code> <type>")` and `Some("account edit <acct>")` (about 12 such arms), so the usage text shown to the user is the identifier that routes the action. Localising the usage line without first separating a stable command id from its label would silently break dispatch.

### 2.5 Error, validation and status messages (110)

There are three sources.

- **App-local errors with Display text**: `thiserror` enums in the bins, for example `bin-tui/src/account/mod.rs:113-123` (`"the transfer target must share {name}'s unit ({unit})"`), `category/mod.rs:72-80`, `payee/mod.rs:145-159`, `tag/mod.rs:79-81`, and the wrapper enums in `bin-desktop/src/error.rs` and `bin-tui/src/error.rs` (`"Configuration error: {0}"`, `"Telemetry error: {0}"`, `"Database error: {0}"`). That is 22 `#[error(...)]` strings in the two bins, of which the account, category, payee and tag ones are written for the screen (they are shown in popups) and the wrapper ones are effectively developer-facing.
- **Validation and status prose written in views**: `"Date must be in YYYY-MM-DD format"`, `"Amount must be a valid decimal amount"`, `"Name is required"`, `"name cannot be empty"`, `"No transactions match these filters."`, `"Failed to load Accounts: {message}"`, `":{name} \u{2014} not yet built"`. Several embed an input format (`YYYY-MM-DD`) that will change with the Locale.
- **Library errors that reach the screen unlocalised**: the bins interpolate `err.to_string()` of a `lib-*` error into a localised-looking prefix. `bin-desktop/src/feasibility_demo.rs` does `LiveCategoriesStatus::Failed(err.to_string())` and then renders `"Failed to load: {message}"`; the TUI list screens render `"Failed to load Budgets: {message}"` and `"Error: {error}"`. The resulting sentence is half Message, half English library text.

Placeholder text such as `"not yet built"` and `"-- not yet built (see issue #153)"` (`bin-desktop/src/shell.rs:1806`, `:3050`; `bin-tui/src/popup/command/mod.rs:247`) appears about 20 times and is temporary; it is better deleted as features land than migrated.

### 2.6 Strings that are Ledger data or `CONTEXT.md` vocabulary

The map's out-of-scope note is that Account, Payee, Category and Tag names are Ledger data. Two things complicate the line.

**Enumerated vocabulary is Message-shaped, but the domain types expose only storage tokens.** The `lib-core` enums have `as_str()` methods that return the lowercase persisted token, not a display label: `AccountType` (`cash`, `bank`, `credit_card`, `investment`, `loan`), `TransactionStatus` (`open`, `cleared`, `reconciled`), `CategoryTypes` (`asset`, `liability`, `income`, `expense`, `equity`), `UnitKind` (`fiat`, `crypto`, `stock`, `precious_metal`, `other`), `BudgetPeriod` (`weekly`, `monthly`, `quarterly`, `yearly`) (`crates/libs/lib-core/src/{account_type,transaction_status,category_types,unit_kind,budget_period}.rs`). These are stored values and must stay stable. The UI then renders them one of three ways:

- Renders the token directly: 9 TUI sites call `.as_str()` on a domain enum for display (`screen/transactions_list.rs:278` prints the Transaction Status token; `screen/units_list.rs:221`, `screen/budgets_list.rs:341`, `screen/unit_detail.rs:266`, `screen/transaction_detail.rs:630`, `screen/budget_detail.rs:423`, `view/categories.rs:712`, `popup/category/edit_popup.rs:240`, `popup/category/new_popup.rs:224`), plus and the desktop demo `feasibility_demo.rs` (Category type).
- Derives a label by string surgery: `account_type.as_str().replace('_', " ")` and `.to_uppercase()` in `view/accounts.rs:533` and `:767`, `popup/account/new.rs:358` and `popup/account/edit.rs:389`, which turns `credit_card` into `credit card` or `CREDIT CARD`.
- Hand-writes a label match: `bin-desktop/src/accounts.rs:29-37` (`type_label`, giving `"Cash"`, `"Bank"`, `"Credit card"`, `"Loan"`, `"Investment"`) and the status legend in `bin-desktop/src/format.rs:134-145` (`"open"`, `"cleared"`, `"reconciled"`, `"flagged"`).

The needed change is a Message id per variant (for example `account-kind-credit-card`), owned by the localisation crate or the bins, looked up from the enum, and the raw-token call sites in the TUI are the ones most likely to be missed because they do not contain a literal. Roughly 22 variants across five enums, plus `Flagged`, need Messages.

**Seed and mock content is not a Message.** 288 desktop and 120 TUI literals are dummy Ledger data used while pages are prototyped: payee names (`"Woolworths"`, `"Sunrise Payroll"`), account names (`"ANZ Everyday"`), category names (`"Groceries"`, `"Household"`), memos, institutions, unit names, sample dates (`"12 sep 2026"`) and log lines. Examples: `bin-desktop/src/transactions.rs` (about 93 literals), `payees.rs`, `tags.rs`, `categories.rs:37-48`, `rail/context.rs:29-68`, the mock blocks in `settings.rs` and `view/dashboard.rs`, `feasibility_demo.rs` (77), and the TUI's `*/fixture.rs` files. These disappear or become database reads as the pages are wired to `lib-database`, so migrating them would be wasted work. One caution: the system-seeded `NO_INSTITUTION = "No institution"` (`bin-desktop/src/accounts.rs:17`) and the demo seed category `"Demo Seed Category"` are strings that ship into real data, so a seeded row that must appear in the user's Locale needs a decision (store a stable key and render a Message, or accept that the seed is written in the Locale at creation time).

### 2.7 Glyph-plus-label pairs

About 25 literals use a Unicode glyph as the sole carrier of meaning or attach one to a label (`\u{25cb}` open, `\u{25d0}` cleared, `\u{25cf}` reconciled, `\u{2691}` flagged, `\u{25c0}`/`\u{25b6}` on chart legends, `[\u{d7}]` chip removers, `(\u{2022})` radio marks, `\u{258c}` input carets, `\u{2713}` matched). The pairing is assembled in code: `format!("{} {name}", status_glyph(...))` in `bin-desktop/src/format.rs`, `"\u{25cb} open \u{b7} \u{2713} reconciled"` in `bin-tui/src/view/payees.rs:776`, `"\u{25c0} expense"` and `"income \u{25b6}"` in `bin-desktop/src/view/dashboard.rs:263-264`, `"{deactivate_glyph} deactivate"` in `bin-tui/src/popup/payee/delete.rs:237`. Glyph choice is a Preference (Display setting `StatusGlyphs` with an ASCII fallback, in `bin-desktop/src/settings.rs` and `format.rs`), not a Locale concern, and the directional arrows in the dashboard legend (`\u{25c0} expense`, `income \u{25b6}`) encode left/right meaning that would flip in a right-to-left Locale (out of scope for this map, but worth a note). The migration shape is: the label is a Message, the glyph is a separate presentation argument, and the join is done by a Message with two variables rather than by `format!`.

### 2.8 Text that is not UI

105 literals are tracing events, `expect` and panic text, attributes and `unreachable!` messages, and 185 are element ids (`"accounts-row-{id}"`, `"primary-rail-{noun:?}"`), icon paths, config keys, file names and URLs (`"desktop-state.json"`, `"icons/wallet.svg"`, `"general.base_unit"`). These should stay untranslated. Tracing text stays English by convention (`/tracing` skill); The desktop's application id and font name (`"Archivo"`) are likewise not Messages. A `lib-config` value such as `"general.negatives"` echoed back in the TUI settings popup (`popup/settings/edit.rs:265-275`) is presentation of a config key and should not be translated.

### 2.9 Library crates

Outside the bins, the user-visible text is limited to error `Display` strings and the enum tokens in section 2.6.

| Crate | `#[error(...)]` strings | Notes |
|---|---|---|
| `lib-core` | 21 | Parse and validation errors (`"Invalid account type: {0}"`, `"Slug cannot be empty"`, `"Hex colour must contain exactly six hexadecimal digits: {0}"`). Reachable from forms that parse user input. |
| `lib-database` | 19 | `"Cannot move Transaction {0} to Account {1}: it is denominated in a different Unit"`, `"Category {0} is not an Expense Category; a Budget can only cap Expense spending"`, plus `"Database error: {0}"`, `"Not found: {0}"`. The first two are domain rules likely to be shown to users. |
| `lib-config` | 4 | Configuration parsing and key binding errors, shown at startup. |
| `lib-rpc` | 6 | Transport and gRPC errors, developer-facing. |
| `lib-tracing` | 2 | Level parsing, developer-facing. |

None of these carry a Locale, and none can, because the libs are meant to stay free of UI concerns (the map keeps `lib-localisation` free of `gpui` and `ratatui`). The usual answer is that libs return typed errors and the bin maps the variant to a Message id; that is a cost the message-scope ticket should price in for the roughly 10 to 15 lib errors that users can actually trigger from a form (`lib-core` parse errors, `lib-database` rule violations), because today the bins mostly interpolate `err.to_string()`.

## 3. Worst patterns to migrate

Ranked by how badly a naive "wrap the literal in a Message lookup" migration would work.

### 3.1 Hidden plurals (about 36 sites)

Every English plural in the code is an `if n == 1` branch or a hardcoded plural noun. There are no plural rules and no `ngettext`-style helper.

- **Explicit singular/plural branches (6 sites):** `bin-desktop/src/accounts.rs:585-590` (`"1 account"` versus `"{count} accounts"`, `NetWorth::count_text`), a duplicate of the same pair at `bin-desktop/src/shell.rs:2700-2701`, `bin-desktop/src/view/accounts/delete_dialog.rs:64-65` and `:81-83` (`"1 transaction"`/`"{count} transactions"`, `"referenced in 1 budget"`/`"referenced in {count} budgets"`), `bin-desktop/src/transaction_chips.rs:167-169` (`"{count} {one}"`/`"{count} {many}"`, with the noun forms passed in), `bin-desktop/src/explorer.rs:268-269` (`let plural = if value == 1 { "" } else { "s" }; format!("{value} {unit}{plural} ago")`, which concatenates a suffix onto a unit word), `bin-desktop/src/feasibility_demo.rs:645-647` (`"categor{}"` with `"y"`/`"ies"`, a demo but the worst form: a stem plus suffix), and `bin-tui/src/screen/csv_import.rs:220-222` (`"Imported {} Balance Check{}"`).
- **Count-blind plurals (about 30 sites):** a number followed by a plural noun that is never singularised, so `1 txns`, `1 matches`, `1 children`, `1 transactions` are all possible. About 20 TUI sites: `bin-tui/src/popup/account/delete.rs:229,238,295`, `popup/payee/delete.rs:212,285,297`, `popup/category/move_popup.rs:165,170,398`, `popup/payee/edit.rs:463`, `view/payees.rs:611,693,812,1003`, `view/tags.rs:427,491,578`, `view/settings.rs:737`. About 8 desktop sites: `view/settings/mod.rs:178-180` (`"{} units"`, `"{} institutions"`), `explorer.rs:398` (`"{entry_count} items"`), `view/settings/delete_unit_dialog.rs:44`. Unlike the branches above, these are latent bugs in English as well as localisation blockers, and the abbreviated `"txns"` is a second English-only shortcut (the abbreviation would need its own translation).
- **Fixed counts inside prose:** the TUI unit popups embed literal numbers in sentences (`"1 account \u{b7} 412 txns \u{b7} 52 prices"`, `popup/unit/edit.rs:124`) that are mock values today, and become real plurals when wired to data.

Fluent's plural selectors handle all of these once each becomes a Message with a count variable. English (`en-AU`, `en-US`, `en-GB`) needs only one/other, so the immediate deliverable is correctness in the source Locale and a structure that survives the addition of a language with more plural categories.

### 3.2 Concatenated and split sentences (about 15 confirmed sites)

The screen renders text in styled fragments, and some sentences are split across fragments so a command name or key can be styled differently. The result is a sentence that cannot be translated as a unit because word order is fixed by the code.

- `bin-desktop/src/shell.rs:3111-3115`: `"Run "`, then a clickable `open`, then `" to load a ledger file, or "`, then `new`, then `" to start one."`, as five children of one element. The natural translation reorders the verb and the commands.
- `bin-tui/src/popup/account/new.rs:288-289` (`"unit cannot change afterwards \u{2014} to hold a"` + `"second unit, make a second account"`), `popup/account/edit.rs:291-307` (a single sentence hard-wrapped across four literals: `"name and type are labels \u{2014} nothing derives"`, `"from them, so renaming is always safe."`, `"correct a wrong opening balance with an"`, `"adjusting transaction, not by rewriting it"`), `popup/settings/guard.rs:121-125` (a paragraph split at `"are"` / `"not touched. Every reported total is re-"`), `popup/payee/delete.rs:256-268` and `:321-324`, `popup/category/edit_popup.rs:265-271` (`"archiving keeps all {} transactions and totals;"` + `"it only stops the category being offered."`). The line breaks are a layout decision made in the source, so these need to be rejoined into paragraphs and wrapped by the renderer.
- `bin-desktop/src/view/accounts/mod.rs:143-155` builds the net-worth line from `"\u{b7} net worth"`, `"{figure} {base}"` and `"\u{b7} {held}"` fragments; `bin-desktop/src/transaction_chips.rs:213-215` builds `"{} of {} {category}transactions shown"` with an empty-or-noun infix.
- Descriptive text stitched with the separator `" \u{b7} "` (middle dot) appears in most TUI detail rows (`"{} \u{b7} not stored"`, `"{} \u{b7} enter ledger"`, `"{} \u{b7} {} \u{b7} {} txns"`): the separator is a presentation choice but the ordering of clauses is language-specific.
- Library-error concatenation (section 2.5).

### 3.3 Hand-rolled number, date and money formatting

This is the single largest piece of duplicated logic and is in scope for the map (formatting moves into the localisation restructure). The findings for the formatting ticket:

- **Money.** Thousands grouping is reimplemented as `fn group_thousands` five times in the TUI (`view/accounts.rs:870`, `view/payees.rs:882`, `view/categories.rs:1112`, `popup/account/edit.rs:474`, `popup/account/delete.rs:452`) and money formatting appears in eleven named variants (`format_money`, `format_money_at`, `format_money_whole`, `format_money_at_f64`, `format_amount`, `format_preview_amount`, and others) across both bins. The desktop has its own path (`format::amount`, `accounts::format_amount`, `settings::format_preview_amount`) driven by the `DecimalSeparator` Preference. The negative sign is written as the literal U+2212 minus in 31 places, and a `+` prefix is added by hand for positive amounts (`format.rs:103`).
- **Dates.** 15 `chrono` `.format("...")` calls in the TUI use `%d %b`, `%b %y`, `%d %b %Y`, `%d %b %y` and `%Y-%m-%d` (`view/accounts.rs:861,999`, `view/categories.rs:1090,1217`, `view/payees.rs:873,1057`, `view/tags.rs:727,762,782`, `view/units.rs:627`, `popup/account/edit.rs:459,463`, `popup/payee/edit.rs`), each immediately followed by `.to_lowercase()`. The desktop has its own date formatting in `format.rs:20-30` and `settings.rs:312-314` driven by a `MONTH_ABBREVIATIONS` table of English month names (`settings.rs:298`), and a separate mock of month labels in `view/dashboard.rs:109-110`. Chrono's `%b` produces English month names unless its optional locale support is used, and `docs/research/chrono-alternatives.md` is the existing note on date-library options.
- **Parsing.** Date input has an English-shaped parser and error text (`bin-desktop/src/transaction_filter_form.rs:178` `parse_day_month_year`, `:213-215` `"use {iso} or today"`, and the TUI's `"Date must be in YYYY-MM-DD format"`), so the input side needs the Locale too, not just display.
- **Currency and units.** Unit codes are written into strings as English-cased fragments (`"{} u"`, `"aud"`, `"bank \u{b7} aud"`, `"{amount} u"`). Currency symbol, position and code display are not handled anywhere.
- **The Preferences overlap.** `bin-desktop/src/settings.rs` defines its own `DateFormat`, `DecimalSeparator`, `RowDensity` and `StatusGlyphs` enums, while `lib-core` defines `DateFormat` (`date_format.rs`) and `NumberFormat` (`number_format.rs`) per ADR-0014. The map already notes these are to be reconciled with the Locale; this inventory confirms there are two parallel definitions in the code base, not one.

### 3.4 Layout-coupled strings

Beyond the padded specifiers in section 2.1: the TUI sizes columns and boxes to English text (for example `WHERE_VALUES_LABEL_WIDTH`, `SUMMARY_LABEL_WIDTH`, `left:<left_budget$`, `truncated:<width$`), the desktop hardcodes label column widths in pixels, and the desktop's help and rail layouts assume short English nouns. These are what the `en-XA` pseudo-Locale is intended to catch and are best fixed at migration time, not before.

## 4. Where the two bins already share wording

There is no shared text source today: both bins hold their own copies. Comparing normalised literals gives 46 exact matches (of more than 1,600 candidates), and most are the shortest words. They fall into these groups.

- **Navigation nouns and page titles**, identical or near-identical in both bins: `Dashboard`, `Accounts`, `Categories`, `Payees`, `Tags`, `Budgets`, `Reports`, `Settings`, `Transactions`, `Units`, `Help`, plus the `Ledger` and `Command` prefixes. The desktop's noun labels live in `key_router.rs:92-101` and the primary rail (`rail/primary.rs`); the TUI's in `popup/command/commands/mod.rs:93-141` and the screens' titles.
- **Table and form column labels**: `Date`, `Payee`, `Category`, `Amount`, `Status`, `Name`, `Code`, `Type`, `Unit`, `Balance`, `From`, `Account`. The desktop writes these in title case in forms and upper case in table headers; the TUI writes them title case in `Cell::from` and upper case in view boxes, so one Message id with a casing rule is the right target.
- **Dashboard box titles**: `NET POSITION`, `NEEDS ATTENTION`, `BUDGETS THIS PERIOD`, `Financial position` (the desktop's dashboard mirrors the TUI's `view/dashboard.rs`).
- **Concept vocabulary**: `mixed units`, `credit card`, `base unit`, `edit account`, `next field`, and the Transaction Status words `open`/`cleared`/`reconciled`.
- **Wrapper error text**: `"Configuration error: {0}"`, `"Telemetry error: {0}"`, `"Database error: {0}"` are byte-identical in `bin-desktop/src/error.rs` and `bin-tui/src/error.rs` (and in `bin-sync-server`, which is out of scope here).
- **Mirrored, not identical**: palette commands describe the same operations but are worded differently (`desktop: "accounts new"`, TUI: `"account new <name> <type> <unit>"`); the desktop status-bar glyph legend and the TUI's `"\u{25cb} open \u{b7} \u{2713} reconciled"` express the same key in different glyph sets. The desktop's `rail/context.rs` and `settings.rs` mock data even copy the TUI demo's payee and unit names, which is Ledger data, not shared Messages.

Shared Messages therefore come to a few hundred strings at most, concentrated in navigation, column labels and entity vocabulary. Everything with a hint, popup prose or a command is per-bin. Two consequences for the scope decision: the shared Catalogue is worth having for the vocabulary layer and the formatting, but a shared Catalogue that only holds these will leave 70 to 80 per cent of the TUI text in bin-specific Messages, and the duplicated wrapper errors are candidates to move out of the bins altogether.

## 5. Per-file counts

The tables count candidate Messages per file (the five buckets that make up the subtotal in section 1, excluding the ambiguous lowercase tokens, mock data, glyphs and non-UI), for the 22 heaviest files in each bin. Columns: static labels and headings, runtime-argument text, keybinding hints, errors and status messages, palette commands and help, total candidates, then mock or fixture literals in the same file for reference.

### 5.1 bin-desktop

| File | Static | Args | Keys | Errors | Palette | Total | Mock |
|---|---|---|---|---|---|---|---|
| `shell.rs` | 7 | 5 | 60 | 17 | 0 | 89 | 0 |
| `command.rs` | 0 | 0 | 3 | 0 | 45 | 48 | 0 |
| `key_router.rs` | 10 | 1 | 24 | 0 | 0 | 35 | 0 |
| `settings.rs` | 16 | 5 | 2 | 0 | 0 | 23 | 29 |
| `rail/primary.rs` | 12 | 0 | 10 | 0 | 0 | 22 | 0 |
| `help.rs` | 15 | 3 | 2 | 0 | 0 | 20 | 0 |
| `transaction_chips.rs` | 3 | 13 | 0 | 0 | 0 | 16 | 0 |
| `explorer.rs` | 11 | 4 | 1 | 0 | 0 | 16 | 0 |
| `view/transactions/filter_popover.rs` | 12 | 2 | 1 | 0 | 0 | 15 | 0 |
| `view/settings/units.rs` | 14 | 0 | 0 | 0 | 0 | 14 | 0 |
| `view/accounts/edit_dialog.rs` | 13 | 1 | 0 | 0 | 0 | 14 | 0 |
| `view/accounts/mod.rs` | 7 | 4 | 0 | 2 | 0 | 13 | 0 |
| `view/accounts/add_dialog.rs` | 10 | 3 | 0 | 0 | 0 | 13 | 0 |
| `view/dashboard.rs` | 12 | 0 | 0 | 0 | 0 | 12 | 25 |
| `view/accounts/delete_dialog.rs` | 5 | 5 | 0 | 1 | 0 | 11 | 0 |
| `statusline.rs` | 9 | 1 | 1 | 0 | 0 | 11 | 0 |
| `view/transactions/table.rs` | 8 | 1 | 0 | 0 | 0 | 9 | 0 |
| `view/settings/display.rs` | 9 | 0 | 0 | 0 | 0 | 9 | 0 |
| `view/settings/add_unit_dialog.rs` | 7 | 2 | 0 | 0 | 0 | 9 | 0 |
| `accounts.rs` | 7 | 2 | 0 | 0 | 0 | 9 | 13 |
| `view/settings/general.rs` | 7 | 1 | 0 | 0 | 0 | 8 | 0 |
| `view/help.rs` | 7 | 1 | 0 | 0 | 0 | 8 | 0 |

The `Keys` column for `shell.rs` and `key_router.rs` is inflated by single-letter key tokens in match arms and the router table, which are key names, not prose. By directory, `view/` holds 226 of the desktop's candidate and lowercase-token literals, `rail/` 32, and the top-level files (`shell.rs`, `command.rs`, `settings.rs`, `help.rs`, `key_router.rs`, and so on) 433.

### 5.2 bin-tui

| File | Static | Args | Keys | Errors | Palette | Total | Mock |
|---|---|---|---|---|---|---|---|
| `screen/reports.rs` | 49 | 5 | 2 | 7 | 0 | 63 | 0 |
| `shell.rs` | 31 | 1 | 25 | 1 | 0 | 58 | 0 |
| `view/settings.rs` | 53 | 2 | 2 | 0 | 0 | 57 | 3 |
| `view/units.rs` | 41 | 6 | 5 | 0 | 0 | 52 | 10 |
| `view/categories.rs` | 28 | 12 | 2 | 1 | 0 | 43 | 0 |
| `view/payees.rs` | 22 | 15 | 2 | 1 | 0 | 40 | 0 |
| `popup/command/commands/payees.rs` | 0 | 0 | 0 | 3 | 34 | 37 | 0 |
| `view/tags.rs` | 17 | 14 | 1 | 1 | 0 | 33 | 0 |
| `view/accounts.rs` | 15 | 14 | 0 | 2 | 0 | 31 | 0 |
| `popup/unit/delete.rs` | 21 | 2 | 6 | 1 | 0 | 30 | 0 |
| `popup/payee/matches.rs` | 12 | 7 | 10 | 1 | 0 | 30 | 0 |
| `popup/unit/new.rs` | 20 | 1 | 7 | 1 | 0 | 29 | 0 |
| `popup/command/commands/categories.rs` | 0 | 0 | 0 | 1 | 27 | 28 | 0 |
| `screen/transaction_detail.rs` | 19 | 1 | 2 | 5 | 0 | 27 | 0 |
| `popup/account/edit.rs` | 15 | 5 | 7 | 0 | 0 | 27 | 0 |
| `popup/settings/guard.rs` | 20 | 0 | 5 | 0 | 0 | 25 | 0 |
| `popup/command/commands/accounts.rs` | 0 | 0 | 0 | 0 | 25 | 25 | 0 |
| `popup/settings/edit.rs` | 19 | 0 | 4 | 0 | 0 | 23 | 0 |
| `popup/payee/delete.rs` | 11 | 6 | 5 | 1 | 0 | 23 | 0 |
| `popup/category/move_popup.rs` | 6 | 9 | 8 | 0 | 0 | 23 | 0 |
| `screen/budget_detail.rs` | 16 | 1 | 1 | 4 | 0 | 22 | 0 |
| `popup/payee/edit.rs` | 8 | 5 | 7 | 2 | 0 | 22 | 0 |

By directory, `popup/` holds 749 candidate and lowercase-token literals, `view/` 354, `screen/` 245, the crate root files 81 and the small entity modules (`account/`, `category/`, `payee/`, `tag/`) 24. Two structural notes matter for scoping migration tickets. `screen/` is the older layer: `screen/dashboard.rs` contains the literal `"moved to the new Shell/View screen"` for several screens, and `view/` is the current one, so migrating `screen/` text that is about to be deleted is a risk; the migration tickets should be sliced from the current `view/` and `popup/` layers first and check `screen/` liveness before touching it. And `popup/` is the single largest block (about 750 literals, of which 190 are palette commands): popup prose is long, conditional and often split across fragments (section 3.2), so it will dominate the migration effort per string.

## 6. Suggested migration slices

These are input to the per-screen build tickets, not a decision.

- **Formatting first.** Replace `group_thousands`, the eleven money formatters and the 15 `strftime` calls with one Locale-aware formatter behind a single API. This removes about 30 duplicated helpers, and it is independent of the Catalogues.
- **Enum vocabulary second.** Add display Messages for the five `lib-core` enums and Flagged (about 23 ids) and route the 9 raw `.as_str()` display sites and the `type_label`/`status_legend` helpers through them.
- **Plurals and split sentences as the tickets that touch each file**, not as a separate sweep, because both need the surrounding sentence rewritten anyway; the 36 plural sites and 15 split-sentence sites are listed in sections 3.1 and 3.2.
- **Per-screen migration in the order that matches the desktop build** (shell chrome, Accounts, Transactions, Settings, Dashboard), with the TUI's `view/` and `popup/` layers sliced by entity (Accounts, Categories, Payees, Tags, Units), and `screen/` last.
- **Delete rather than migrate** the "not yet built" placeholders (about 20), the `feasibility_demo.rs` demo strings (about 77, plus its chart labels) if that file is retired, and mock data (408 literals).
- **Decide first** whether palette command names (not only descriptions) are localised, whether library errors are mapped to Message ids in the bins, and whether seeded Ledger rows (`No institution`) store a key or a rendered name, because each of these changes the shape of a large group of tickets.

## 7. Limits of this inventory

- Counts are literals, not distinct Messages: many literals repeat (`"Cancel"`, `"next field"`, month names) and several distinct literals would collapse into one Message with variables (the seven `" esc close ... form "` strings). Expect the final Catalogue to hold fewer ids than the 1,687 candidate literals, plausibly 900 to 1,200 before pluralisation adds variants.
- The classifier is regular-expression based. Mock-data detection in view files used hand-picked line ranges for the desktop and pattern rules for the TUI, and the 457 lowercase tokens were not individually reviewed.
- Text built through helpers that return `&'static str` from a match (for example `status_glyph`, `type_label`) is counted at the literal, but text produced by non-literal paths (`Display` impls, `to_string()` on enums, values read from the database) is not visible to a literal scan and is only covered where this note names it.
- Strings in `docs/ux/**` design prototypes, the `.dc.html` handoffs and `README.md` files are outside the scope of this ticket.
