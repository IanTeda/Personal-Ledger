## The Inventory module's Messages, read by its Presenters. Each Client composes this layer into its
## Locale; no Client catalogue keeps a copy. Column labels other screens share (Name) come from the
## shared layer.

## The Settings › Inventory page (`docs/ux/desktop-mockups/16-settings/`, frame 16q, redrawn on
## #698): Locations, each with the Areas of its Inventory register.

## The heading's meta, "3 locations · 14 areas · 241 items". Zeros are shown.

inventory-count-locations = { $count ->
    [one] { $count } location
   *[other] { $count } locations
}
inventory-count-areas = { $count ->
    [one] { $count } area
   *[other] { $count } areas
}
inventory-count-items = { $count ->
    [one] { $count } item
   *[other] { $count } items
}
inventory-scope = { $locations } · { $areas } · { $items }

## The kicker, with how many Locations, and the square button beside it that adds one.

inventory-kicker = Locations · { $count }
inventory-add-location = +

## A Location row's note: its Area and Item counts, each a Message above.

inventory-location-note = { $areas } · { $items }

## The empty state, shown in place of the table when there are no Locations. `$key` is the key that
## opens Add location.

inventory-empty = No locations yet. Press { $key } to add one.

## The note beneath the table.

inventory-note = Each location is one register in Inventory (switch with o); its areas are the area tabs, in this order. A location's address and insurance cover are kept on its register in Inventory; this is just the list you manage. Removing an area that still has items asks which area to move them to; removing a location keeps its items' transactions and documents.

## A row's context menu, opened by right-click. The remove entries are drawn last, in red.

inventory-menu-add-area = Add area…
inventory-menu-edit-location = Edit location…
inventory-menu-remove-location = Remove location…
inventory-menu-edit-area = Edit area…
inventory-menu-move-up = Move up
inventory-menu-move-down = Move down
inventory-menu-remove-area = Remove area…

## The status line's keys by selected row, and the Toast when the add-area key is pressed with no
## Location.

inventory-hint-move = move
inventory-hint-expand = expand
inventory-hint-location = location
inventory-hint-reorder = reorder
inventory-hint-edit = edit
inventory-hint-remove = remove
inventory-hint-new-location = new location
inventory-hint-new-area = new area
inventory-toast-no-location = Add a location first

## The disclosure control at the start of a Location row.

inventory-disclosure-open = ▾
inventory-disclosure-closed = ▸

## The Add, Edit and Remove location dialogs. Settings asks only for the name and Unit: a location's
## address and cover are edited on its register in Inventory.

inventory-dialog-add-title = Add location
inventory-dialog-add-submit = Add location
inventory-dialog-edit-title = Edit location
inventory-dialog-save = Save
inventory-dialog-remove-title = Remove location
inventory-dialog-remove-submit = Remove location
inventory-field-name = Name
inventory-field-name-placeholder = e.g. Holiday house, Hilux
inventory-field-unit = Unit
inventory-field-unit-hint = Every value in this location is in this Unit. It can't be changed later.
inventory-field-unit-fixed = { $unit } · fixed when the location was added
inventory-unit-option = { $code } · { $name }

## What is wrong with a field.

inventory-error-name-empty = Give the location a name.
inventory-error-name-taken = Another location already has that name.
inventory-error-unit-missing = Add a fiat Unit in Settings › Units first.

## Why a Dialog's confirm was refused, when the row it acts on went while it was open.

inventory-refusal-gone = That location or area has gone or changed, so nothing was saved.

## Remove location. The counts are Messages above; `$name` is the location's name.

inventory-count-documents = { $count ->
    [one] { $count } document
   *[other] { $count } documents
}
inventory-remove-plain = Remove { $name }? It has no areas or items.
inventory-remove-warning = Removing { $name } also removes its { $areas } and { $items }. { $documents } will lose a link to them. The documents and the transactions that bought the items are kept.
inventory-remove-confirm-label = Type { $name } to confirm

## Toasts.

inventory-toast-location-added = Added location "{ $name }"
inventory-toast-location-saved = Saved location "{ $name }"
inventory-toast-location-removed = Removed location "{ $name }"

## The Add, Edit and Remove area dialogs (#490, #487). An area's name is unique within its location.

inventory-area-add-title = Add area
inventory-area-add-submit = Add area
inventory-area-edit-title = Edit area
inventory-area-edit-submit = Save
inventory-area-field-name = Name
inventory-area-field-name-placeholder = e.g. Garage, Tray
inventory-area-error-name-empty = Give the area a name.
inventory-area-error-name-taken = Another area in this location already has that name.
inventory-area-in-location = In { $location }. Its items stay with it if you rename it.
inventory-area-remove-title = Remove area
inventory-area-remove-submit = Remove area
inventory-area-remove-plain = Remove { $name }? It has no items.
inventory-area-remove-with-items = { $name } holds { $items }. Choose the area that takes them.
inventory-area-remove-destination = Move items to
inventory-area-blocked-title = Can't remove area
inventory-area-blocked-body = { $name } is the only area in { $location } and it holds { $items }. Add another area first, or remove the location.
inventory-area-blocked-close = Close
inventory-toast-area-added = Added area "{ $name }"
inventory-toast-area-saved = Saved area "{ $name }"
inventory-toast-area-removed = Removed area "{ $name }"
inventory-toast-area-removed-moved = Removed area "{ $name }" and moved its { $items } to "{ $destination }"
