## The Shell's own status-line hints (`lib_shell::ShellViewModel`). Each label sits beside its key,
## which the Client styles.

hint-command = command
hint-search = search
hint-help = help
hint-toggle-sidebar = toggle sidebar

## The `?` Help card's shortcut cheat-sheet (`lib_shell::HelpPresenter`).
help-shortcut-palette = Command palette
help-shortcut-help = This help
help-shortcut-focus = Move focus
help-shortcut-confirm = Open / confirm
help-shortcut-close = Close overlay
help-shortcut-undo = Undo
help-shortcut-redo = Redo

## The status line when `u` or `ctrl+r` finds an empty History (`lib_shell::ShellPresenter`).
status-nothing-to-undo = Nothing to undo
status-nothing-to-redo = Nothing to redo

## The status line once `u`, `ctrl+r` or a click in the header's History list has stepped through
## History, `$count` entries, `$label` the last entry stepped.
status-undid = { $count ->
    [1] Undid { $label }
   *[other] Undid { $count } changes, back to before { $label }
}
status-redid = { $count ->
    [1] Redid { $label }
   *[other] Redid { $count } changes, through { $label }
}

## A multi-step Undo or Redo that failed partway: `$count` entries stepped first, `$failure` why the
## next didn't.
status-undid-then-failed = { $count ->
    [1] Undid 1 change, then: { $failure }
   *[other] Undid { $count } changes, then: { $failure }
}
status-redid-then-failed = { $count ->
    [1] Redid 1 change, then: { $failure }
   *[other] Redid { $count } changes, then: { $failure }
}

## Why an Undo or Redo of a Ledger change didn't save. The domain is on the Error Toast.
status-step-not-saved = Couldn't save that, so the Ledger is back as last saved
status-step-not-saved-unsaved = Couldn't save that or reload the Ledger, so what you see isn't saved

## Why an Undo or Redo of a device setting didn't go to personal-ledger.conf (ADR-0053). `$key` is the
## setting's key in the file; `$source` is the environment variable or configuration file that sets it.
settings-device-shadowed = { $key } is set by { $source }, which outranks your personal-ledger.conf: change it there
settings-device-no-file = { $key } can't be saved: there's no home folder to keep personal-ledger.conf in
settings-device-write-failed = Couldn't save { $key } to personal-ledger.conf: the log says why

## When a History entry was recorded, back from now, in the header's Undo list.
history-just-now = just now
history-minutes-ago = { $minutes ->
   *[other] { $minutes } min ago
}
history-hours-ago = { $hours ->
   *[other] { $hours } h ago
}
history-days-ago = { $days ->
   *[other] { $days } d ago
}
