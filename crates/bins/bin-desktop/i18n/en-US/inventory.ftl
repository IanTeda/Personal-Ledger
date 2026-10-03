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

## The Add, Edit and Remove property dialogs (#490's variant B). The cover section is always shown:
## an empty Insurer means no cover.

desktop-inventory-dialog-add-title = Add property
desktop-inventory-dialog-add-submit = Add property
desktop-inventory-dialog-edit-title = Edit property
desktop-inventory-dialog-save = Save
desktop-inventory-dialog-remove-title = Remove property
desktop-inventory-dialog-remove-submit = Remove property
desktop-inventory-field-name = Name
desktop-inventory-field-name-placeholder = e.g. Holiday house
desktop-inventory-field-address = Address (optional)
desktop-inventory-field-address-placeholder = Street, suburb
desktop-inventory-field-unit = Unit
desktop-inventory-field-unit-hint = Every value in this property is in this Unit. It can't be changed later.
desktop-inventory-field-unit-fixed = { $unit } · fixed when the property was added
desktop-inventory-unit-option = { $code } · { $name }
desktop-inventory-section-cover = Insurance cover
desktop-inventory-field-insurer = Insurer
desktop-inventory-field-insurer-hint = Leave the insurer empty for no cover
desktop-inventory-field-policy-no = Policy number
desktop-inventory-field-renews-on = Renews on
desktop-inventory-field-renews-on-placeholder = e.g. 2027-03-01
desktop-inventory-field-sum-insured = Sum insured
desktop-inventory-field-item-limit = Single-item limit
desktop-inventory-field-amount-placeholder = 0.00

## What is wrong with a field.

desktop-inventory-error-name-empty = Give the property a name.
desktop-inventory-error-name-taken = Another property already has that name.
desktop-inventory-error-amount = Enter an amount, such as 25000 or 25000.50.
desktop-inventory-error-sum-missing = Enter the sum insured, or empty the insurer for no cover.
desktop-inventory-error-sum-not-positive = The sum insured must be more than 0.
desktop-inventory-error-limit-not-positive = The single-item limit must be more than 0.
desktop-inventory-error-limit-above-sum = The single-item limit can't be more than the sum insured.
desktop-inventory-error-unit-missing = Add a fiat Unit in Settings › Units first.

## Remove property. The counts are Messages above; `$name` is the property's name.

desktop-inventory-count-documents = { $count ->
    [one] { $count } document
   *[other] { $count } documents
}
desktop-inventory-remove-plain = Remove { $name }? It has no rooms or items.
desktop-inventory-remove-warning = Removing { $name } also removes its { $rooms } and { $items }. { $documents } will lose a link to them. The documents and the transactions that bought the items are kept.
desktop-inventory-remove-confirm-label = Type { $name } to confirm

## Toasts.

desktop-inventory-toast-property-added = Added property "{ $name }"
desktop-inventory-toast-property-saved = Saved property "{ $name }"
desktop-inventory-toast-property-removed = Removed property "{ $name }"
