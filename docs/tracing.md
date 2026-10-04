# Tracing (Logging)

Because we are using asynchronous functions, we use telemetry to group together log outputs in a meaningful way.

## Overview

The Personal Ledger application uses a structured logging system based on the
[`tracing`](https://docs.rs/tracing/latest/tracing/) crate. This provides
hierarchical, contextual logging that makes it easier to understand application
flow and debug issues in asynchronous code.

The console and log file use the `log` Configuration (see [Settings](settings.md)), which defaults to `info`; `lib_tracing::init` also falls back to `INFO` when it is given no level. `RUST_LOG` overrides both for the console and log file.

## Telemetry Levels

The Personal Ledger uses the following telemetry levels, ordered from most
verbose to least verbose:

### TRACE

- **Purpose**: Maximum verbosity for detailed debugging
- **Use Case**: Development and troubleshooting
- **Performance Impact**: High (may impact performance)
- **Example Output**: Function entry/exit points, detailed state changes

### DEBUG

- **Purpose**: Debug information for troubleshooting
- **Use Case**: Development and staging environments
- **Performance Impact**: Moderate
- **Example Output**: Variable values, API call details, intermediate results

### INFO

- **Purpose**: Informational messages about application flow
- **Use Case**: Production monitoring
- **Performance Impact**: Low
- **Example Output**: Application startup, successful operations, user actions

### WARN

- **Purpose**: Warning messages for potential issues
- **Use Case**: Production monitoring
- **Performance Impact**: Low
- **Example Output**: Deprecated API usage, recoverable errors, configuration
  issues

### ERROR

- **Purpose**: Error conditions that may require attention
- **Use Case**: Production monitoring and alerting
- **Performance Impact**: Low
- **Example Output**: Failed operations, invalid inputs, system errors

### OFF

- **Purpose**: Completely disable telemetry output
- **Use Case**: Performance-critical environments, testing
- **Performance Impact**: None
- **Example Output**: No telemetry output

## Configuration

The `log` level and `log_file_path` live in the `[Personal-Ledger]` section; see [Settings](settings.md).

## Live capture for the Desktop

The Desktop's Settings › Tracing page shows this run's own events as they happen, without reading the log file back. `lib_tracing::init` takes an optional `LogBuffer`: when one is passed, a capture layer also records events into it, a ring of the last 1000 (`LOG_CAPACITY`). The Desktop passes one; the TUI and Sync Server pass `None`, so they never capture.

- Every layer has its own filter. The console and log file share the `EnvFilter` (the `log` level, `RUST_LOG`, and the `calloop=warn` cap). The capture layer uses the same filter, or our own crates (`bin_*`, `lib_*`) at `debug`: it holds everything the terminal shows, plus our `debug` lines even when `log` is quieter. Dependencies stay at the terminal's level, so their `debug` chatter bridged from `log` doesn't push our events out of the ring.
- Each entry is a structured `LogEntry`: sequence number, local capture time, level, target (the original target for `log`-bridged records), message and fields. The viewer decides the format.
- After each push the buffer calls a waker set with `set_waker`, outside its lock. The Desktop's waker signals a `tokio::sync::Notify`; the page waits about 100 ms for a burst to settle, then pulls `since(seq)` and redraws once.
- Clear logs empties the buffer for every reader; sequence numbers keep counting.

Design and research: [#497](https://github.com/IanTeda/Personal-Ledger/issues/497) and `docs/research/tracing-ring-buffer-capture.md` (on the `research/tracing-capture` branch).
