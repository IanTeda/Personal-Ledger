# Institutions

An institution is the bank, lender or broker that holds one of your accounts: ANZ, American Express, Vanguard. Personal Ledger keeps a list of them so each account can say where it is held.

> **Heads up:** in the desktop app, the institutions you add are saved. Editing and deleting an institution isn't built yet: the `edit` and `delete` links, and the `e` and `d` keys, only tell you so. The terminal app has no Institutions screen.

## What you can do

- See every institution in one list, in the order you added them, with its account types
- Add an institution with a name and the account types it offers
- Choose an institution when you create an account
- Keep your institutions after you close the app

## Terminology

- **Institution:** The bank, lender or broker an account is held with (for example ANZ Banking Group or Vanguard Investments).
- **Account type:** A label for what the institution offers, such as savings, credit card, offset, loan or investment. The list shows the types you picked joined together.
- **No institution:** A built-in entry for cash accounts, which have no bank. It never appears on the Institutions page.

## Institutions concept

Every account names exactly one institution, so you add the institutions you use before you add accounts. You don't have to add them all at once: the account dialog lists whatever you have so far. Cash is the one exception. Physical cash has no bank, so cash accounts use the built-in "No institution" entry instead.

An institution's name is how you tell it apart, so two institutions can't share one. "ANZ" and "anz" count as the same name, and a name with only spaces in it isn't a name.

## Approach

In the desktop app, open the Institutions page in Settings (press `g` `s` and pick Institutions, or run `:settings institutions`). On the page:

- Move up and down the list with `j` and `k` or the arrow keys, jump to the top or bottom with `g` `g` and `G` (or `Home` and `End`), and move half a page with `ctrl+d` and `ctrl+u` (or `Page Down` and `Page Up`). Clicking a row selects it.
- Press `n`, or click Add institution, to add one. If you've changed these keys under `[keybindings]` in your settings, the page follows your keys.
- In the add dialog, `Tab` and `Shift+Tab` move between the fields. Type the name, choose at least one account type, choose a default unit, then press `ctrl+Enter` to add it, or `Esc` to close without adding.
- If the name is blank or already taken, the dialog says so under the name and won't add until you change it.
- `e` and `d`, and each row's `edit` and `delete` links, say the action isn't built yet.

A message at the bottom of the window confirms each institution you add. If it can't be saved, a message says so straight away and the list goes back to what was last saved.

## Worked example

You're setting up your ledger and bank with Commonwealth Bank. You open Settings, choose Institutions and press `n`. You type "Commonwealth Bank", tick savings and offset, leave the default unit as AUD and press `ctrl+Enter`. The list now ends with Commonwealth Bank, shown as "savings · offset", and a message says it was added. You close the desktop app. When you open it again Commonwealth Bank is still there, and the account dialog offers it when you create your savings account.

## Screens

The desktop app's Institutions page lists every institution in the order you added them, with its name and account types. The heading says how many there are. The selected row is highlighted, and each row has `edit` and `delete` links. An Add institution button sits at the top.

## Rules to know

- A name must be set and can't match another institution's name, ignoring capitals.
- The default unit in the add dialog helps you fill in the form, but it isn't saved with the institution yet.
- The account types are a label for the list. They don't limit which accounts you can create with the institution.

## Scope

- Editing an institution isn't built yet.
- Deleting an institution isn't built yet. When it is, an institution that an account names will be refused, as a payee in use is.
- Institutions don't carry a default unit, address or contact details.
- Institutions aren't synced between devices yet.
- The terminal app doesn't have an Institutions screen.

## Related

- **Accounts:** each account is held with one institution, chosen when you create it.
- **Units:** the add dialog offers your units as a default unit.
- **Setting up your ledger:** adding institutions is step 3 of the [new ledger workflow](new-ledger-workflow.md).

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Institutions | not in the terminal app yet | Settings › Institutions (`:settings institutions`) |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Institutions. Ticked means built in at least one app; the tag says which.

- [x] INS-001 (desktop): List institutions in the order they were added, with their account types
- [x] INS-002 (desktop): Add an institution with a name and account types, refusing a blank or taken name
- [x] INS-003 (desktop): Keep institutions after the app closes
- [x] INS-004 (desktop): Choose an institution when creating an account
- [ ] INS-005: Edit an institution
- [ ] INS-006: Delete an institution no account names

## For developers

Curious how institutions are structured in the codebase, or planning to change them? See the [Institutions development documentation](development/institutions.md).
