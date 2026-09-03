#!/bin/sh
# docker-entrypoint.sh -- runs as root (the image's default), adjusts the baked-in
# `syncserver` account to match the operator-supplied PUID/PGID (the same runtime
# env-var convention linuxserver.io images use), fixes up ownership of the durable
# store at /data, then drops privileges via `setpriv` before exec'ing the real Sync
# Server process -- so the *running* process is always non-root (NFR.2), while the
# UID/GID it runs as is configurable without an image rebuild. This matters once #49
# mounts a host directory at /data: the container's user needs to match whatever UID
# already owns that host path, which varies per self-hoster and can't be baked in at
# build time.
#
# `setpriv` (not `su`/`gosu`) is used to drop privileges because it's already present
# in debian:bookworm-slim (part of util-linux, Debian's "required" priority package
# set) -- no extra package install or third-party binary download needed.
set -eu

PUID="${PUID:-10001}"
PGID="${PGID:-10001}"

if [ "$(id -u)" = "0" ]; then
    current_gid="$(getent group syncserver | cut -d: -f3)"
    if [ "$current_gid" != "$PGID" ]; then
        groupmod -o -g "$PGID" syncserver
    fi

    current_uid="$(id -u syncserver)"
    if [ "$current_uid" != "$PUID" ]; then
        usermod -o -u "$PUID" syncserver
    fi

    chown -R syncserver:syncserver /data

    exec setpriv --reuid syncserver --regid syncserver --clear-groups "$@"
fi

# Already running as non-root (e.g. `docker run --user`) -- nothing to adjust.
exec "$@"
