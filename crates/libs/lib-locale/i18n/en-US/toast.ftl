## Toasts: the outcome of an action or event, shared by the desktop and the TUI. See docs/toasts-design.md.

## Deletes. The UI change is not enough feedback for a delete, so each reports what went with it.

toast-account-deleted = Deleted account { $name } · { $transactions ->
    [0] no transactions
    [one] { $transactions } transaction deleted
   *[other] { $transactions } transactions deleted
}

toast-category-deleted = Deleted category { $name } · { $splits ->
    [0] no splits
    [one] { $splits } split moved to Uncategorised
   *[other] { $splits } splits moved to Uncategorised
}

## The TUI refuses to delete an account with transactions until they have a transfer target, so they move rather than go.

toast-account-deleted-moved = Deleted account { $name } · { $transactions ->
    [one] { $transactions } transaction moved to { $target }
   *[other] { $transactions } transactions moved to { $target }
}

toast-payee-deleted = Deleted payee { $name }
toast-payee-deactivated = Deactivated payee { $name }
toast-payee-reactivated = Reactivated payee { $name }
toast-tag-deleted = Deleted tag { $name }
toast-tag-merged = Merged tag { $source } into { $target } · { $transactions ->
    [0] no transactions
    [one] { $transactions } transaction retagged
   *[other] { $transactions } transactions retagged
}

## A store refusal. $entity is the thing being saved ("account", "tag"), $reason the store's own words.

toast-save-failed = Couldn't save { $entity }: { $reason }

## The $entity of toast-save-failed.

toast-entity-account = account
toast-entity-category = category
toast-entity-payee = payee
toast-entity-tag = tag

## Ledger files.

toast-ledger-opened = Opened ledger { $name }
toast-ledger-created = Created ledger { $name }
toast-ledger-open-failed = Couldn't open ledger { $name }: { $reason }
toast-ledger-create-failed = Couldn't create ledger { $name }: { $reason }

## The session Toast history popup.

toast-history-title = Toast history
toast-history-empty = No Toasts yet this session

## The Settings "Toasts" row. Errors always toast (ADR-0027), which the note says beside the switch.

toast-setting-label = Toasts
toast-setting-on = On
toast-setting-off = Off
toast-setting-note = Errors still show

## The Settings Toasts row's Test, and the note beside it once a Client has it. It raises one Toast of each Kind, a second apart (DESKTOP-184).

toast-setting-test = Test
toast-setting-test-note = Errors still show. Test raises one of each kind.

## The four test Toasts, one per Kind.

toast-test-success = Test: a Success Toast
toast-test-info = Test: an Info Toast
toast-test-warning = Test: a Warning Toast
toast-test-error = Test: an Error Toast, which stays until dismissed
