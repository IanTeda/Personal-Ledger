## The Delete account Dialog (17-accounts' 3d), the Account Deletion module's one View. A Client
## composes this layer into its Locale; no Client catalogue keeps a copy. `$name` is the Account's
## name and `$balance` its balance with its Unit, already formatted. The `<strong>` spans are the
## emphasised figures.

account-deletion-title = Delete account — { $name }
account-deletion-submit = Delete account

account-deletion-warning = This account has a balance of <strong>{ $balance }</strong> and <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}.</strong> Deleting it cannot be undone.

account-deletion-reference = { $transactions ->
    [one] { $transactions } transaction
   *[other] { $transactions } transactions
} will be permanently deleted · { $budgets ->
    [0] not referenced by any budget
    [one] referenced in { $budgets } budget
   *[other] referenced in { $budgets } budgets
}

account-deletion-confirm-label = Type { $name } to confirm

## Why the Account can't be deleted yet: Bill Plans pay from it, and the Bills page moves them.

account-deletion-bill-plans = { $plans ->
    [one] { $plans } bill is paid from this account. Move it to another account before deleting this one.
   *[other] { $plans } bills are paid from this account. Move them to another account before deleting this one.
}

account-deletion-error-gone = This account has since been deleted.
