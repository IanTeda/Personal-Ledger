## The Settings › Inventory page (`docs/ux/desktop/16-settings/`, frame 16q, as settled on #491):
## Properties, each with the Rooms of its Inventory register.

## The heading's meta, "2 properties · 11 rooms · 232 items". Zeros are shown.

desktop-inventory-count-properties = { $count ->
    [one] { $count } property
   *[other] { $count } properties
}
desktop-inventory-count-rooms = { $count ->
    [one] { $count } room
   *[other] { $count } rooms
}
desktop-inventory-count-items = { $count ->
    [one] { $count } item
   *[other] { $count } items
}
desktop-inventory-scope = { $properties } · { $rooms } · { $items }

## The kicker, the Add buttons and the column headings. `$glyph` is the leading plus sign.

desktop-inventory-kicker = Properties
desktop-inventory-add-property = { $glyph } Add property
desktop-inventory-add-room = { $glyph } Add room
desktop-inventory-column-property = Property
desktop-inventory-column-policy = Policy
desktop-inventory-column-sum-insured = Sum insured
desktop-inventory-column-item-limit = Item limit
desktop-inventory-column-items = Items
desktop-inventory-column-room = Room
desktop-inventory-column-value = Value

## Cells. A dash stands for "not set": no cover, or no item limit.

desktop-inventory-none = —
desktop-inventory-row-edit = edit
desktop-inventory-row-remove = remove

## The empty state, shown in place of the table when there are no Properties.

desktop-inventory-empty = No properties yet. Press n to add one.

## The three notes beneath the table.

desktop-inventory-note-registers = Each property is one Inventory register; switch between them with o. Its rooms are the register's room tabs, in this order.
desktop-inventory-note-cover = Sum insured and Item limit drive the cover check and the over-limit flag. A property with no insurer has no cover.
desktop-inventory-note-removal = Removing a room that holds items asks which room to move them to. Removing a property removes its rooms and items, but keeps the transactions and documents they were linked to.

## The status line's keys by selected row, and the Toast when `r` is pressed with no Property.

desktop-inventory-hint-move = move
desktop-inventory-hint-property = property
desktop-inventory-hint-new-property = new property
desktop-inventory-hint-new-room = new room
desktop-inventory-toast-no-property = Add a property first

## The disclosure control at the start of a Property row.

desktop-inventory-disclosure-open = ▾
desktop-inventory-disclosure-closed = ▸
