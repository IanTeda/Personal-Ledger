# Inventory

Inventory is where you keep a register of the things you own, place by place: a house's kitchen and garage, a ute's cab and toolbox, a storage unit. Each place is a Location, split into Areas, and the things in it are Items with a replacement value.

> **Heads up:** in the desktop app, the Locations and Areas you set up in Settings are saved, so they are still there when you reopen it. A Ledger file made before this was saved starts with an empty Inventory (see Tips), so start a fresh Ledger to get the sample Locations. The Inventory register itself (adding Items, editing a Location's address and insurance cover, area tabs and totals) isn't built yet, so you can't add or edit an Item on the desktop. The terminal app has no Inventory screen.

## What you can do

- Add a Location, such as a house, a storage unit, a car or a boat, with its name and Unit
- Rename a Location
- Add Areas to a Location, rename them and put them in the order you want their tabs
- Remove an Area, choosing where its Items go
- Remove a Location, with its Areas and Items
- See how many Areas and Items each Location holds
- Link a Document, such as a receipt, to an Item
- Keep your Locations and Areas after you close the desktop app

## Terminology

- **Location:** A place whose contents you keep as one register, such as a house, a storage unit, a car or a boat. It has a name, one Unit and any number of Areas.
- **Area:** A named part of one Location, such as a Kitchen or a Garage, or a ute's Cab and Toolbox. Areas are one level deep and never move between Locations.
- **Item:** A physical possession you record for its replacement value, in one Area of one Location.
- **Unit:** The currency every value in a Location is in, such as AUD. You choose it when you add the Location and it can't change.
- **Cover:** A Location's insurance details: insurer, policy number, renewal date, sum insured and single-item limit.
- **Link:** A Document's reference to an Item it proves you own.

## Inventory concept

Personal Ledger keeps one register per Location rather than one long list, because a house, a storage unit and a car are looked after separately and are often insured separately. A Location's Areas become the tabs of its register, in the order you set. An Item sits in exactly one Area and its replacement value is in the Location's Unit. A Location is the register of contents, not the house or the car itself.

## Approach

1. In Settings, open the Inventory page (press `g` `s` and pick Inventory, or run `:settings inventory`).
2. Press `n` to add a Location: give it a name and a Unit.
3. Press `a` to add its Areas, then `J` and `K` to put them in the order you want the tabs.
4. Later, the register will hold your Items. Until it is built, the Items you see are the sample ones.

Remove an Area when you no longer need it. If it holds Items you choose another Area to take them. Remove a Location when you are done with it.

## Worked example

You buy a holiday house and want to keep track of its contents. On the Inventory page you press `n`, name the Location "Holiday house", choose AUD as its Unit and press `enter`. The page shows "Holiday house" with "0 areas · 0 items". With it selected you press `a` and add "Kitchen", then "Garage", then "Deck". The Deck was a mistake, so you select it and press `x`. It holds no Items, so you confirm and it goes. Later you decide the Garage should come first, so you select it and press `K` to move it up one place. Close the app and reopen it: the Location and its two Areas are still there, in that order.

## Screens

The desktop app's Inventory page is one list headed "Locations", with a count beside it and a square **+** button that adds a Location. Above it a summary reads, for example, "3 locations · 14 areas · 241 items". Each Location shows its name in bold, with a note of how many Areas and Items it holds. Locations are listed in the order you added them and start collapsed. Expand one with `right` or the arrow beside its name to see its Areas indented underneath, in tab order, and fold it again with `left`. With no Locations the list says "No locations yet. Press n to add one."

The Add and Edit location dialogs ask for a name and, on Add, a Unit. The Unit is chosen from your fiat Units and shows "fixed when the location was added" on Edit. Add and Edit area ask for a name. The Remove area dialog asks for the Area that takes its Items when it holds any. If the Area is the Location's only one and holds Items, a notice explains that you need to add another Area first, or remove the Location. The Remove location dialog tells you how many Areas and Items go and how many Documents lose a Link to them, and asks you to type the Location's name to confirm. A Location with no Areas or Items is removed with a plain confirm.

Rows have no buttons. Double-click a row to edit it, and right-click it for its menu. A Location's menu has Add area, Edit location and Remove location. An Area's has Edit area, Move up, Move down and Remove area. `Esc` or a click elsewhere closes the menu.

## Rules to know

- Location names are unique, ignoring case. Area names are unique within a Location, ignoring case, but two Locations can each have a Kitchen.
- A Location's Unit is fixed when you add it, and it must be a fiat currency.
- An Area never moves to another Location.
- Removing an Area that holds Items moves them to another Area of the same Location, and you choose which. A Location's last Area can't be removed while it holds Items.
- Removing a Location removes its Areas and Items and drops the Document Links to those Items. The Documents themselves, and the Transactions that bought the Items, are kept. It can't be undone.
- A Location may have no Areas, but then it holds no Items.
- An insurance Cover needs an insurer and a sum insured above zero. A single-item limit must be above zero and no more than the sum insured.
- An Item needs a name, and its replacement value can't be below zero.

## Tips

- **Start with the Areas you would search by:** Kitchen and Garage are easier to browse than one Area called Everything.
- **A fresh Ledger gets the sample Inventory:** the sample Locations, Areas and Items are written when a Ledger is first created. A Ledger file made before Inventory was saved opens with none, and its Documents lose the Item chips they link to. Start a fresh Ledger to get them.
- **A car can be a Location or an Item:** make it a Location if it has contents of its own, or an Item in another Location's Area if it doesn't.

## Scope

- The Inventory register, where Items are added and edited and a Location's address and Cover are changed, isn't built on the desktop yet.
- A Location's address and Cover are kept on its register, not in Settings, so they can't be edited on the desktop yet.
- Real estate and vehicles as assets aren't covered; a Location names a register of contents.
- The terminal app has no Inventory screen.
- Attaching a file to an Item from Inventory isn't built; see [Documents](documents.md) for linking an Item from a Document.

## Related

- **Documents:** a Document can link to an Item to prove you own it. Removing a Location drops those links.
- **Units:** every Location has one fiat Unit, which its values are in.
- **Transactions:** the purchase of an Item is a Transaction. Removing a Location keeps it.
- **Settings:** the Inventory page is one of its pages; see [Settings](settings.md).

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Inventory | not in the terminal app yet | Settings › Inventory (`:settings inventory`); `g` `o` (not yet built) |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Inventory. Ticked means built in at least one app; the tag says which.

- [x] INV-001 (desktop): Add a Location with a name and a fiat Unit
- [x] INV-002 (desktop): Rename a Location
- [x] INV-003 (desktop): Add, rename and reorder a Location's Areas
- [x] INV-004 (desktop): Remove an Area, moving its Items to another Area
- [x] INV-005 (desktop): Remove a Location with its Areas and Items, dropping the Document Links to them
- [x] INV-006 (desktop): See how many Areas and Items each Location holds
- [x] INV-007 (desktop): Keep your Locations, Areas and Items after you close the app
- [ ] INV-008: Add, edit and remove Items on a Location's register
- [ ] INV-009: Edit a Location's address and insurance Cover on its register
- [ ] INV-010: See the total replacement value of a Location, an Area and an Item list
- [ ] INV-011: Show Inventory in the terminal app

## For developers

Curious how Inventory is structured in the codebase, or planning to change it? See the [Inventory development documentation](development/inventory.md).
