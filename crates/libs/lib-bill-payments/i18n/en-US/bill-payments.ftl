## The Pay Dialog (8d), the Bill Payments module's one View. A Client composes this layer into its
## Locale; no Client catalogue keeps a copy. `$name` is the Bill Plan's name and `$due` the entry's
## due date, both already formatted; `$amount` is the Plan's planned amount, formatted with its
## unit.

bill-payments-pay-title = Pay — { $name }, { $due }
bill-payments-pay-mode-direct = Pay it directly
bill-payments-pay-mode-match = Match existing transaction
bill-payments-pay-candidates = Unmatched transactions
bill-payments-pay-none-of-these = None of these — pay it directly instead
bill-payments-pay-no-candidates = No unmatched expense within 14 days of the due date.
bill-payments-pay-match-notice = Matching marks this entry Paid using that transaction's real date and amount — the plan's { $amount } estimate is discarded for this cycle. It won't create a duplicate transaction.
bill-payments-pay-direct-notice = Creates a transaction from the plan's category, payee and account, and marks this entry Paid.
bill-payments-pay-field-amount = Amount
bill-payments-pay-field-date = Date
bill-payments-pay-amount-placeholder = 0.00
bill-payments-pay-no-payee = No payee
bill-payments-pay-submit-direct = Create transaction & mark paid
bill-payments-pay-submit-match = Match & mark paid
bill-payments-pay-error-amount = Enter an amount greater than zero.
bill-payments-pay-error-gone = This bill can no longer be paid.

## One Match candidate: its Payee (or the Transaction's description), date and signed amount, each
## already formatted.

bill-payments-pay-candidate = { $payee } · { $date } · { $amount }
