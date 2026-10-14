#!/bin/sh
# Drop privileges after the data volume is writable by uid 10001.
set -eu
data_dir="${DATA_DIR:-/data}"
if [ "$(id -u)" = "0" ]; then
    mkdir -p "$data_dir"
    chown -R 10001:10001 "$data_dir"
    exec setpriv --reuid=10001 --regid=10001 --init-groups -- "$0" "$@"
fi
exec /usr/local/bin/delta-sync-server
