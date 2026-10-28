# Institutions

An institution is the bank, lender or broker that holds one of your accounts: ANZ, American Express, Vanguard. Personal Ledger keeps a list of them so each account can say where it is held.

## What you can do

- See every institution in one list, in the order you added them, with its products
- Add an institution with a name and the products it offers
- Edit an institution's name and products
- Delete an institution no account is held with
- Choose an institution when you create an account
- Keep your institutions after you close the app

## Terminology

- **Institution:** The bank, lender or broker an account is held with (for example ANZ Banking Group or Vanguard Investments).
- **Products:** A label for what the institution offers: savings, offset, credit card, loan, investment, crypto, retirement or cash. The list shows the products you picked joined together.
- **Personal institution:** The institution that stands for you, the person keeping the ledger. Every new ledger starts with one, named Personal, and a new cash account starts on it.

## Institutions concept

Every account names exactly one institution, so you add the institutions you use before you add accounts, and an institution an account names can't be deleted. You don't have to add them all at once: the account dialog lists whatever you have so far. Physical cash has no bank, but you hold it, so a new ledger starts with a Personal institution that stands for you, and a new cash account starts on it. It is an ordinary institution: you can rename it, edit it, and pick another for a cash account if you prefer.

An institution's name is how you tell it apart, so two institutions can't share one. "ANZ" and "anz" count as the same name, and a name with only spaces in it isn't a name.

## Approach

In the desktop app, open the Institutions page in Settings (press `g` `s` and pick Institutions, or run `:settings institutions`). On the page:

- Move up and down the list with `j` and `k` or the arrow keys, jump to the top or bottom with `g` `g` and `G` (or `Home` and `End`), and move half a page with `ctrl+d` and `ctrl+u` (or `Page Down` and `Page Up`). Clicking a row selects it.
- Press `n`, or click Add institution, to add one. If you've changed these keys under `[keybindings]` in your settings, the page follows your keys.
- In the add dialog, type the name and click at least one product, then press `ctrl+Enter` to add it, or `Esc` to close without adding.
- If the name is blank or already taken, the dialog says so under the name and won't add until you change it.
- Press `e`, or click a row's `edit` link, to edit the selected institution. You can also right-click a row for a menu with Edit institution and Delete institution. The edit dialog opens with the name filled in and its products ticked; change either, then press `ctrl+Enter` to save or `Esc` to close without saving.
- The edit dialog follows the same rules as the add dialog. You can change only the capitals of a name ("anz" to "ANZ"), but not take a name another institution holds.
- Press `d`, click a row's `delete` link, or choose Delete institution from the row's menu, to delete the selected institution. The dialog asks you to type the institution's name back exactly, capitals included, before Delete institution turns on. `Esc` closes it with nothing deleted.
- After a delete, the next row is selected, or the one before it if you deleted the last.
- If an account is held with the institution, the dialog says "Used by N accounts" instead, lists the first five account names and then "and N more", and offers only Close. Change those accounts' institution, then delete it. If an account comes to use the institution while the dialog is open, pressing Delete institution turns the dialog into this refusal instead of deleting.

A message at the bottom of the window confirms each institution you add, save or delete. If it can't be saved, a message says so straight away and the list goes back to what was last saved.

## Worked example

You're setting up your ledger and bank with Commonwealth Bank. You open Settings, choose Institutions and press `n`. You type "Commonwealth Bank", tick savings and offset and press `ctrl+Enter`. The list now ends with Commonwealth Bank, shown as "savings · offset", and a message says it was added. You close the desktop app. When you open it again Commonwealth Bank is still there, and the account dialog offers it when you create your savings account.

You later rename it to "CBA": you right-click Commonwealth Bank, choose Edit institution, type the new name and press `ctrl+Enter`. The account dialog offers CBA from then on, and the accounts held there show the new name.

Later you close your savings account at CBA and move your other accounts elsewhere. You select CBA and press `d`. The dialog asks you to type CBA, you do, and Delete institution turns on. You press `ctrl+Enter`, CBA leaves the list and the row above it is selected. Had your savings account still been held at CBA, the dialog would have said "Used by 1 account", named it and offered only Close.

## Screens

The desktop app's Institutions page lists every institution in the order you added them, with its name and products. The heading says how many there are. The selected row is highlighted, and each row has `edit` and `delete` links. Right-clicking a row selects it and opens its menu. An Add institution button sits at the top. The add dialog asks for a name, the products and, optionally, a usual unit. `Tab` moves between the name and the usual unit. The edit dialog is the add dialog, titled Edit institution, with a Save institution button. The delete dialog has a red frame, the name to type back and a Delete institution button; for an institution accounts use, it shows the refusal and a Close button instead.

## Rules to know

- A name must be set and can't match another institution's name, ignoring capitals.
- The products are a label for the list. They don't limit which accounts you can create with the institution.
- An institution can have a usual unit, the unit its accounts are normally held in. Pick one from the active units, or None, which most institutions keep. It is only a hint: in the desktop app a new account with that institution starts on its usual unit instead of the base unit, and you can pick another unit before saving. It never changes an existing account, and your base unit stays the one default unit.
- In the desktop app, deleting a unit that institutions name as their usual unit asks which unit each of them uses instead, starting on None (see [Units](units.md)). In the TUI, such a unit can't be deleted: clear or change those institutions' usual unit first.
- A new ledger starts with the Personal institution, named in your language. A cash account can name any institution; Personal is only where it starts.
- An institution an account names can't be deleted, the Personal institution included. Deleting is permanent and can't be undone.

## Scope

- Institutions don't carry an address or contact details.
- Institutions aren't synced between devices yet.
- The terminal app doesn't have an Institutions screen.

## Related

- **Accounts:** each account is held with one institution, chosen when you create it.
- **Setting up your ledger:** adding institutions is step 3 of the [new ledger workflow](new-ledger-workflow.md).

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Institutions | not in the terminal app yet | Settings › Institutions (`:settings institutions`) |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Institutions. Ticked means built in at least one app; the tag says which.

- [x] INS-001 (desktop): List institutions in the order they were added, with their products
- [x] INS-002 (desktop): Add an institution with a name and products, refusing a blank or taken name
- [x] INS-003 (desktop): Keep institutions after the app closes
- [x] INS-004 (desktop): Choose an institution when creating an account
- [x] INS-005 (desktop): Edit an institution
- [x] INS-006 (desktop): Delete an institution no account names, refusing one an account names

## For developers

Curious how institutions are structured in the codebase, or planning to change them? See the [Institutions development documentation](development/institutions.md).
