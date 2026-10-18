## The Settings › Inventory page (`docs/ux/desktop-mockups/16-settings/`, frame 16q, redrawn on #698):
## Locations, each with the Areas of its Inventory register.

## The heading's meta, "3 locations · 14 areas · 241 items". Zeros are shown.

desktop-inventory-count-locations = { $count ->
    [one] { $count } location
   *[other] { $count } locations
}
desktop-inventory-count-areas = { $count ->
    [one] { $count } area
   *[other] { $count } areas
}
desktop-inventory-count-items = { $count ->
    [one] { $count } item
   *[other] { $count } items
}
desktop-inventory-scope = { $locations } · { $areas } · { $items }

## The kicker, with how many Locations, and the square button beside it that adds one.

desktop-inventory-kicker = Locations · { $count }
desktop-inventory-add-location = +

## A Location row's note: its Area and Item counts, each a Message above.

desktop-inventory-location-note = { $areas } · { $items }

## The empty state, shown in place of the table when there are no Locations.

desktop-inventory-empty = No locations yet. Press n to add one.

## The note beneath the table.

desktop-inventory-note = Each location is one register in Inventory (switch with o); its areas are the area tabs, in this order. A location's address and insurance cover are kept on its register in Inventory; this is just the list you manage. Removing an area that still has items asks which area to move them to; removing a location keeps its items' transactions and documents.

## A row's context menu, opened by right-click. The remove entries are drawn last, in red.

desktop-inventory-menu-add-area = Add area…
desktop-inventory-menu-edit-location = Edit location…
desktop-inventory-menu-remove-location = Remove location…
desktop-inventory-menu-edit-area = Edit area…
desktop-inventory-menu-move-up = Move up
desktop-inventory-menu-move-down = Move down
desktop-inventory-menu-remove-area = Remove area…

## The status line's keys by selected row, and the Toast when `a` is pressed with no Location.

desktop-inventory-hint-move = move
desktop-inventory-hint-location = location
desktop-inventory-hint-new-location = new location
desktop-inventory-hint-new-area = new area
desktop-inventory-toast-no-location = Add a location first

## The disclosure control at the start of a Location row.

desktop-inventory-disclosure-open = ▾
desktop-inventory-disclosure-closed = ▸

## The Add, Edit and Remove location dialogs. Settings asks only for the name and Unit: a location's
## address and cover are edited on its register in Inventory.

desktop-inventory-dialog-add-title = Add location
desktop-inventory-dialog-add-submit = Add location
desktop-inventory-dialog-edit-title = Edit location
desktop-inventory-dialog-save = Save
desktop-inventory-dialog-remove-title = Remove location
desktop-inventory-dialog-remove-submit = Remove location
desktop-inventory-field-name = Name
desktop-inventory-field-name-placeholder = e.g. Holiday house, Hilux
desktop-inventory-field-unit = Unit
desktop-inventory-field-unit-hint = Every value in this location is in this Unit. It can't be changed later.
desktop-inventory-field-unit-fixed = { $unit } · fixed when the location was added
desktop-inventory-unit-option = { $code } · { $name }

## What is wrong with a field.

desktop-inventory-error-name-empty = Give the location a name.
desktop-inventory-error-name-taken = Another location already has that name.
desktop-inventory-error-unit-missing = Add a fiat Unit in Settings › Units first.

## Remove location. The counts are Messages above; `$name` is the location's name.

desktop-inventory-count-documents = { $count ->
    [one] { $count } document
   *[other] { $count } documents
}
desktop-inventory-remove-plain = Remove { $name }? It has no areas or items.
desktop-inventory-remove-warning = Removing { $name } also removes its { $areas } and { $items }. { $documents } will lose a link to them. The documents and the transactions that bought the items are kept.
desktop-inventory-remove-confirm-label = Type { $name } to confirm

## Toasts.

desktop-inventory-toast-location-added = Added location "{ $name }"
desktop-inventory-toast-location-saved = Saved location "{ $name }"
desktop-inventory-toast-location-removed = Removed location "{ $name }"

## The Add, Edit and Remove area dialogs (#490, #487). An area's name is unique within its location.

desktop-inventory-area-add-title = Add area
desktop-inventory-area-add-submit = Add area
desktop-inventory-area-edit-title = Edit area
desktop-inventory-area-edit-submit = Save
desktop-inventory-area-field-name = Name
desktop-inventory-area-field-name-placeholder = e.g. Garage, Tray
desktop-inventory-area-error-name-empty = Give the area a name.
desktop-inventory-area-error-name-taken = Another area in this location already has that name.
desktop-inventory-area-in-location = In { $location }. Its items stay with it if you rename it.
desktop-inventory-area-remove-title = Remove area
desktop-inventory-area-remove-submit = Remove area
desktop-inventory-area-remove-plain = Remove { $name }? It has no items.
desktop-inventory-area-remove-with-items = { $name } holds { $items }. Choose the area that takes them.
desktop-inventory-area-remove-destination = Move items to
desktop-inventory-area-blocked-title = Can't remove area
desktop-inventory-area-blocked-body = { $name } is the only area in { $location } and it holds { $items }. Add another area first, or remove the location.
desktop-inventory-area-blocked-close = Close
desktop-inventory-toast-area-added = Added area "{ $name }"
desktop-inventory-toast-area-saved = Saved area "{ $name }"
desktop-inventory-toast-area-removed = Removed area "{ $name }"
desktop-inventory-toast-area-removed-moved = Removed area "{ $name }" and moved its { $items } to "{ $destination }"
