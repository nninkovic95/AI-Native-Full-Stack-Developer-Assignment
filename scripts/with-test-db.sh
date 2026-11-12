#!/usr/bin/env bash
# Runs a command against a throwaway local Postgres, e.g.:
#   scripts/with-test-db.sh cargo test
# Needs the Postgres server binaries (initdb, pg_ctl) but no running service.
# TCP only (no Unix socket) so it also works in sandboxes that block sockets.
set -euo pipefail

PGBIN="${PGBIN:-$(dirname "$(command -v pg_ctl || echo /usr/bin/pg_ctl)")}"
PORT="${TEST_PG_PORT:-55432}"
DATA="${TMPDIR:-/tmp}/ajaia-test-pg"

if [ ! -f "$DATA/PG_VERSION" ]; then
  rm -rf "$DATA"
  "$PGBIN/initdb" -D "$DATA" -U ajaia --auth=trust -E UTF8 >/dev/null
fi

"$PGBIN/pg_ctl" -D "$DATA" -l "$DATA/server.log" -w \
  -o "-h 127.0.0.1 -p $PORT -k '' -c fsync=off -c dynamic_shared_memory_type=mmap" \
  start >/dev/null
trap '"$PGBIN/pg_ctl" -D "$DATA" -m fast -w stop >/dev/null' EXIT

export DATABASE_URL="postgres://ajaia@127.0.0.1:$PORT/postgres"
"$@"
