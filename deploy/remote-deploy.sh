#!/usr/bin/env bash
# Runs on the server: pulls the image built in CI, restarts the landing, and
# installs this app's Caddy snippet. Touches only /opt/apps/delta_sync and one
# file in /opt/apps/edge/sites. IMAGE_TAG is the commit SHA pushed to GHCR.
set -euo pipefail

APP_DIR="$(cd "$(dirname "$0")/.." && pwd)"
EDGE_CONTAINER="edge-caddy"
EDGE_SITES="/opt/apps/edge/sites"
SITE_NAME="deltasync.simplysoftware.app.caddy"
LANDING_CONTAINER="delta-sync-landing"
LANDING_PORT=3600

cd "${APP_DIR}"

if [[ -z "${IMAGE_TAG:-}" ]]; then
  echo "IMAGE_TAG is required" >&2
  exit 1
fi
export IMAGE_TAG

# edge-caddy uses host networking and reaches the landing on the loopback port
# published by compose.prod.yaml. Fail early if something else owns that port.
if [[ -n "$(ss -ltnH "sport = :${LANDING_PORT}")" ]]; then
  if ! docker port "${LANDING_CONTAINER}" 3000/tcp 2>/dev/null | grep -q "^127.0.0.1:${LANDING_PORT}\$"; then
    echo "ERROR: 127.0.0.1:${LANDING_PORT} is already in use by something other than ${LANDING_CONTAINER}" >&2
    ss -ltnp "sport = :${LANDING_PORT}" >&2 || true
    exit 1
  fi
fi

docker compose -f compose.prod.yaml pull
docker compose -f compose.prod.yaml up -d --remove-orphans

landing_ok=0
for _ in 1 2 3 4 5 6 7 8 9 10; do
  if curl -fsS --max-time 5 -o /dev/null "http://127.0.0.1:${LANDING_PORT}/"; then
    landing_ok=1
    break
  fi
  sleep 3
done
if [[ "${landing_ok}" -ne 1 ]]; then
  echo "ERROR: landing did not answer on 127.0.0.1:${LANDING_PORT}" >&2
  docker compose -f compose.prod.yaml logs --tail 50 landing >&2 || true
  exit 1
fi
echo "Landing healthy on 127.0.0.1:${LANDING_PORT}"

# One file. Do not rewrite the shared Caddyfile or any other site.
install -d "${EDGE_SITES}"
install -m 644 deploy/caddy/deltasync.simplysoftware.app.caddy "${EDGE_SITES}/${SITE_NAME}"
docker exec "${EDGE_CONTAINER}" caddy reload --config /etc/caddy/Caddyfile

