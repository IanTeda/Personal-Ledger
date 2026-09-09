# Configuration

The Personal Ledger application will look for configuration files in multiple locations. It uses a layered configuration system with a defined precedence order. This provides flexibility for different deployment scenarios, from development to production.

Per [ADR-0014](adr/0014-preferences-table-and-leaner-sync-server-config.md), Configuration covers only settings needed before the app (or its database) can run: database connection/pool settings, the telemetry level, and (for the Sync Server only) the gRPC bind address. Everything else the user might tweak from inside a running Client — colour theme, date format, decimal/thousands separator, default Unit — is a Preference instead, stored in the Ledger's own database, not here.

`lib-config` is shared by all three consumers (`bin-tui`, `bin-desktop`, `bin-sync-server`), but they don't all see the same sections or search the same locations — see [Configuration Hierarchy](#configuration-hierarchy) and [Sync Server Section](#sync-server-section) below.

## Explicit Config File via CLI

All three binaries accept a `--config`/`-c` flag to point at an explicit config file, taking the same precedence as "Explicit Configuration File" below:

```sh
tui --config ./my-personal-ledger.conf
sync-server -c /etc/personal-ledger/sync-server.conf
```

## INI Configuration Format

Personal Ledger uses the INI (Initialisation) file format for configuration files. INI is a simple, human-readable format consisting of sections, keys, and values.

### Basic Syntax

- **Sections**: Enclosed in square brackets `[]`, e.g., `[Telemetry]`
- **Keys and Values**: `key = value`, e.g., `telemetry_level = "debug"`
- **Comments**: Lines starting with `#` or `;` are comments
- **Case Sensitivity**: Section names are case-insensitive (e.g., `[Telemetry]` and `[telemetry]` are equivalent)

### Example Structure

```ini
# This is a comment
[SectionName]
key1 = "string value"
key2 = 42
key3 = true
```

### Rules

- Section names should be descriptive and contain only alphanumeric characters, underscores, and hyphens
- Keys should use lowercase with underscores (snake_case)
- String values should be quoted when they contain spaces or special characters
- Boolean values: `true` or `false`
- Numeric values: integers or floats as appropriate

### Section Names

Configuration sections group related settings together. The application currently supports:

- `[Telemetry]`: Logging and telemetry settings (read by all three consumers)
- `[Database]`: Database connection and pool settings (read by all three consumers)
- `[Sync-Server]`: Sync Server-only settings (bind address) -- only `bin-sync-server` reads it; `bin-tui`/`bin-desktop` ignore it. Hyphens or underscores both work (`[Sync-Server]`/`[sync_server]`), unlike the other sections, which are plain words.

## Configuration Hierarchy

`bin-tui` and `bin-desktop` (the Clients) load configuration from multiple sources in the following precedence order (highest to lowest):

1. **Environment Variables** (highest precedence)

   - Prefix: `PERSONAL_LEDGER_`
   - Example: `PERSONAL_LEDGER_TELEMETRY__TELEMETRY_LEVEL=debug`
   - Example: `PERSONAL_LEDGER_DATABASE__URL=sqlite:/tmp/test.db`
   - Use double underscores (`__`) to separate nested keys

2. **Explicit Configuration File**

   - Passed via the `--config`/`-c` CLI flag
   - Useful for custom configurations in specific deployments

3. **Current Working Directory**

   - File: `config/personal-ledger.conf`
   - Allows project-specific overrides when running from a directory

4. **Executable Directory**

   - Configuration file in the same directory as the binary
   - Useful for portable applications

5. **User Configuration**

   - Platform-specific user config directory
   - Linux/macOS: `~/.config/personal-ledger/personal-ledger.conf`
   - Windows: `%APPDATA%\personal-ledger\personal-ledger.conf`

6. **System Configuration**

   - Platform-specific system-wide config directory
   - Linux: `/etc/personal-ledger/personal-ledger.conf`
   - macOS: `/Library/Preferences/personal-ledger/personal-ledger.conf`
   - Windows: `%ALLUSERSPROFILE%\personal-ledger\personal-ledger.conf`

7. **Built-in Defaults** (lowest precedence)

## Sync Server Section

`bin-sync-server` uses a **reduced** precedence chain instead (ADR-0014): **Environment Variables** → **Explicit Configuration File** (`--config`/`-c`) → **Built-in Defaults**. The Current Working Directory/Executable Directory/User/System tiers don't apply -- they don't correspond to anything meaningful inside a Docker container, the Sync Server's only deployment target.

The `[Sync-Server]` section currently has one setting:

### bind_address

The socket address the Sync Server's gRPC/HTTP listener binds to.

- **Type**: String
- **Default**: `"0.0.0.0:50051"`

Example:

```ini
[Sync-Server]
bind_address = "0.0.0.0:50051"
```

   - Hardcoded default values in the application code

Higher precedence sources override lower precedence ones. For example, an environment variable will override any configuration file setting.

## Telemetry Section

The `[Telemetry]` section controls logging and telemetry output for the application.

### telemetry_level

Controls the verbosity of logging output.

- **Type**: String
- **Valid Values**:
  - `"trace"`: Most verbose, includes all internal debugging information
  - `"debug"`: Detailed debugging information
  - `"info"`: General information messages (default)
  - `"warn"`: Warning messages only
  - `"error"`: Error messages only
  - `"off"`: No logging output
- **Default**: `"info"`

Example:

```ini
[Telemetry]
telemetry_level = "debug"
```

## Database Section

The `[Database]` section controls database connection and connection pool settings for the application.

### url

The database connection URL.

- **Type**: String
- **Valid Values**: SQLite URLs in the format `sqlite:path/to/database.db`
- **Default**: `"sqlite:./personal-ledger.sqlite"`

Examples:

- `sqlite:./personal-ledger.sqlite` (relative path)
- `sqlite:/tmp/personal-ledger.db` (absolute path)
- `sqlite::memory:` (in-memory database)

### max_connections

Maximum number of connections in the connection pool.

- **Type**: Integer
- **Valid Values**: Positive integers
- **Default**: `10`

Higher values allow more concurrent database operations but use more resources.

### min_connections

Minimum number of connections to maintain in the connection pool.

- **Type**: Integer
- **Valid Values**: Non-negative integers, must be ≤ `max_connections`
- **Default**: `1`

The pool will try to keep at least this many connections ready to improve performance.

### acquire_timeout_seconds

Timeout for acquiring a connection from the pool (in seconds).

- **Type**: Integer
- **Valid Values**: Positive integers
- **Default**: `30`

If no connection becomes available within this time, an error is returned.

### idle_timeout_seconds

Idle timeout for connections (in seconds).

- **Type**: Integer
- **Valid Values**: Non-negative integers (0 to disable)
- **Default**: `600`

Connections that remain unused beyond this time may be closed to free resources.

### max_lifetime_seconds

Maximum lifetime for connections (in seconds).

- **Type**: Integer
- **Valid Values**: Non-negative integers (0 to disable)
- **Default**: `1800`

Connections older than this will be closed and replaced to prevent stale connections.

Example:

```ini
[Database]
url = "sqlite:./personal-ledger.sqlite"
max_connections = 20
min_connections = 2
acquire_timeout_seconds = 60
idle_timeout_seconds = 300
max_lifetime_seconds = 3600
```

## Example Configuration File

```ini
# Personal Ledger Configuration File
#
# This file contains configuration settings for the Personal Ledger application.
# It uses INI format with sections and key-value pairs.
#
# Section names are case-insensitive (e.g., [Telemetry] or [telemetry] both work).
# Values should be quoted strings where appropriate.
#
# For more information, see the documentation at docs/configuration.md

[Telemetry]
# Logging level for telemetry output
# Valid values: "trace", "debug", "info", "warn", "error", "off"
telemetry_level = "trace"

[Database]
# Database connection URL
url = "sqlite:./personal-ledger.sqlite"

# Connection pool settings
max_connections = 10
min_connections = 1

# Timeout settings (in seconds)
acquire_timeout_seconds = 30
idle_timeout_seconds = 600
max_lifetime_seconds = 1800

# Only read by bin-sync-server -- bin-tui/bin-desktop ignore this section.
[Sync-Server]
bind_address = "0.0.0.0:50051"
```
