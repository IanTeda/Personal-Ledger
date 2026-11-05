## Settings › Tracing (Logs): the level radios, the log box's empty states, Clear logs and its Dialog.
## The log line format itself is diagnostic output, not a Message (docs/localisation-design.md).

settings-tracing-level-error = error
settings-tracing-level-warn = warn
settings-tracing-level-info = info
settings-tracing-level-debug = debug
settings-tracing-clear = Clear logs
settings-tracing-empty = No log entries yet.
settings-tracing-all-hidden = No entries at { $level } or above. Choose a lower level to see { $hidden ->
    [one] { $hidden } hidden entry
   *[other] { $hidden } hidden entries
}.
settings-tracing-clear-title = Clear logs
settings-tracing-clear-warning = Remove every captured log entry from this page? New entries keep arriving, and the console and log file keep their own copies.
settings-tracing-toast-cleared = Logs cleared

## The page's status-line hints.

settings-tracing-hint-level = level
settings-tracing-hint-scroll = scroll
settings-tracing-hint-clear-logs = clear logs
