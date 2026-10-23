#!/usr/bin/env bash
#
# Local/self-hosted backup for Atlas's own Postgres (docker-compose.atlas.yml).
# Not a substitute for whatever the eventual production host provides -
# see docs/atlas/SystemDesignDocument.md Section 16.2.1 for the full
# disaster-recovery plan this is one piece of.
#
# Commented out on purpose: this hasn't been wired into an actual routine
# (a cron job, a pre-deploy step) yet, so it stays inert rather than
# runnable-but-unused. Uncomment the body below once it is.

# set -euo pipefail
#
# CONTAINER="atlas-sms-postgres-1"
# BACKUP_DIR="${BACKUP_DIR:-./docker/backups}"
# TIMESTAMP="$(date +%Y%m%d%H%M%S)"
# OUT_FILE="${BACKUP_DIR}/atlas-${TIMESTAMP}.sql.gz"
#
# mkdir -p "$BACKUP_DIR"
#
# docker exec "$CONTAINER" pg_dump -U atlas -d atlas | gzip > "$OUT_FILE"
#
# echo "Backup written to $OUT_FILE"
