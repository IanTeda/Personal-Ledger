---
post_title: 'Telemetry (Logging)'
author1: 'Personal Ledger Team'
post_slug: 'telemetry-logging'
microsoft_alias: 'personal-ledger'
featured_image: ''
categories: ['documentation', 'development']
tags: ['telemetry', 'logging', 'tracing', 'rust']
ai_note: 'AI-assisted documentation'
summary: 'Comprehensive guide to telemetry and logging in the Personal Ledger
application, including configuration, usage patterns, and best practices.'
post_date: '2025-12-06'
---
# Telemetry (Logging)

Because we are using asynchronous functions, we use telemetry to group together log outputs in a meaningful way.

## Overview

The Personal Ledger application uses a structured logging system based on the
[`tracing`](https://docs.rs/tracing/latest/tracing/) crate. This provides
hierarchical, contextual logging that makes it easier to understand application
flow and debug issues in asynchronous code.

The telemetry library will use `WARN` level if no level is set.

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
- **Use Case**: Production monitoring (default level)
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

This is empty for now, as config is a work in progress after telemetry.
