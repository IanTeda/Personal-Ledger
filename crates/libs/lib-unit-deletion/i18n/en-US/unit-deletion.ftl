## The Delete unit Dialog's Usual unit section (16-settings' Units), the Unit Deletion module's
## additions to `lib-units`' Delete unit Dialog. A Client composes this layer into its Locale beside
## `lib-units`' own. `$code` is the Unit's code and `$count` how many Institutions name it.

unit-deletion-institutions-help = { $count ->
    [one] { $count } institution uses { $code } as its usual unit. Choose the unit it uses instead, or leave it on None.
   *[other] { $count } institutions use { $code } as their usual unit. Choose the unit each uses instead, or leave them on None.
}

## The choice that leaves an Institution with no Usual unit.

unit-deletion-usual-unit-none = None

unit-deletion-refusal-replacement-gone = A usual unit you chose has since been deleted or deactivated. Choose another.
