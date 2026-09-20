## Formatting Messages: where a Unit's code sits beside a quantity that is not a fiat amount.

unit-quantity = { $quantity } { $code }

## Words accepted in a typed date, besides the Locale's numeric form and ISO. English is always
## accepted as well, so a missing translation cannot stop `today` working.

date-word-today = today
date-word-yesterday = yesterday
date-word-tomorrow = tomorrow

## The part of a date a validation error points at.

date-field-day = day
date-field-month = month
date-field-year = year

## Typed date validation. `$example` is today's date in the Locale's field order with a four-digit year, `$iso` today's
## ISO date and `$words` the accepted words.

date-error-empty = Enter a date.
date-error-unrecognised = Enter a date like { $example } or { $iso }, or a word such as { $words }.
date-error-unrecognised-iso = Enter a date like { $iso }, or a word such as { $words }.
date-error-out-of-range = That is not a valid { $field } for a date.
date-error-bad-year = Use a four-digit year.
date-error-year-required = Include the year.
