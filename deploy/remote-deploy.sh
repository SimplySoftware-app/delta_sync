#!/usr/bin/env bash
# Runs on the server: pulls the image built in CI, rolls the landing over to the
# idle colour (blue/green, no downtime) and installs this app's Caddy snippet.
# Touches only /opt/apps/delta_sync and one file in /opt/apps/edge/sites.
# IMAGE_TAG is the commit SHA pushed to GHCR.
#
# Rollout: start the idle colour, wait until Docker reports it healthy, make sure
# edge-caddy lists both ports and has marked the new colour healthy, then stop
# the old colour with a graceful drain. If the new colour never becomes healthy
# it is removed and the live one is left untouched (exit non-zero).
set -euo pipefail

APP_DIR="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE_FILE="compose.prod.yaml"
EDGE_CONTAINER="edge-caddy"
EDGE_SITES="/opt/apps/edge/sites"
SITE_NAME="deltasync.simplysoftware.app.caddy"
SNIPPET="deploy/caddy/deltasync.simplysoftware.app.caddy"

SERVICE_PREFIX="landing"                 # compose services landing-blue / landing-green
CONTAINER_PREFIX="delta-sync-landing"    # containers delta-sync-landing-blue / -green
LEGACY_CONTAINER="delta-sync-landing"    # single container of the old layout
BLUE_PORT=3650
GREEN_PORT=3651
INNER_PORT=3000
HEALTH_TIMEOUT=180                       # seconds to wait for "healthy"
CADDY_SETTLE=6                           # one Caddy health interval (2s) + margin
DRAIN_SECONDS=30                         # matches stop_grace_period

STATE_FILE="${APP_DIR}/.active-color"
ENV_FILE="${APP_DIR}/.env"

cd "${APP_DIR}"

if [[ -z "${IMAGE_TAG:-}" ]]; then
  echo "IMAGE_TAG is required" >&2
  exit 1
fi

compose() { docker compose -f "${COMPOSE_FILE}" "$@"; }
port_of() { [[ "$1" == blue ]] && echo "${BLUE_PORT}" || echo "${GREEN_PORT}"; }
container_of() { echo "${CONTAINER_PREFIX}-$1"; }
is_running() { [[ "$(docker inspect -f '{{.State.Running}}' "$1" 2>/dev/null || true)" == "true" ]]; }
exists() { docker inspect "$1" >/dev/null 2>&1; }

# Set KEY=VALUE in the server .env (created if missing), keeping other lines.
set_env() {
  local key="$1" value="$2" tmp
  tmp="$(mktemp)"
  if [[ -f "${ENV_FILE}" ]]; then
    grep -v "^${key}=" "${ENV_FILE}" > "${tmp}" || true
  fi
  printf '%s=%s\n' "${key}" "${value}" >> "${tmp}"
  cat "${tmp}" > "${ENV_FILE}"
  rm -f "${tmp}"
}
get_env() { # last value of KEY in .env, empty if unset
  [[ -f "${ENV_FILE}" ]] || return 0
  grep "^$1=" "${ENV_FILE}" | tail -n 1 | cut -d= -f2- || true
}

# --- Which colour is live? ---------------------------------------------------
LIVE=""
saved="$(cat "${STATE_FILE}" 2>/dev/null || true)"
if [[ "${saved}" == blue || "${saved}" == green ]] && is_running "$(container_of "${saved}")"; then
  LIVE="${saved}"
elif is_running "$(container_of blue)"; then
  LIVE=blue
elif is_running "$(container_of green)"; then
  LIVE=green
fi
if [[ "${LIVE}" == blue ]]; then IDLE=green; else IDLE=blue; fi
LEGACY=0
if [[ -z "${LIVE}" ]] && exists "${LEGACY_CONTAINER}"; then
  LEGACY=1 # first deploy after the move to blue/green: the old single container is live
fi
IDLE_CONTAINER="$(container_of "${IDLE}")"
IDLE_PORT="$(port_of "${IDLE}")"
IDLE_SERVICE="${SERVICE_PREFIX}-${IDLE}"
echo "Live: ${LIVE:-none}$([[ ${LEGACY} -eq 1 ]] && echo ' (legacy container)'); deploying to ${IDLE}"

# Fail early if something else owns the idle colour's loopback port.
if [[ -n "$(ss -ltnH "sport = :${IDLE_PORT}")" ]]; then
  if ! docker port "${IDLE_CONTAINER}" "${INNER_PORT}/tcp" 2>/dev/null | grep -q "^127.0.0.1:${IDLE_PORT}\$"; then
    echo "ERROR: 127.0.0.1:${IDLE_PORT} is already in use by something other than ${IDLE_CONTAINER}" >&2
    ss -ltnp "sport = :${IDLE_PORT}" >&2 || true
    exit 1
  fi
fi

# edge-caddy must run on the host network: the colours are only on the loopback.
NET_MODE="$(docker inspect "${EDGE_CONTAINER}" --format '{{.HostConfig.NetworkMode}}')"
if [[ "${NET_MODE}" != "host" ]]; then
  echo "ERROR: ${EDGE_CONTAINER} is not on the host network (${NET_MODE}); the snippet proxies 127.0.0.1" >&2
  exit 1
fi

# Both tags live in .env so manual compose commands stay coherent. The live
# colour keeps its tag (or gets the new one if it is not known). compose reads
# this file itself.
live_tag=""
[[ -n "${LIVE}" ]] && live_tag="$(get_env "IMAGE_TAG_$(echo "${LIVE}" | tr '[:lower:]' '[:upper:]')")"
live_tag="${live_tag:-${IMAGE_TAG}}"
if [[ "${IDLE}" == blue ]]; then
  set_env IMAGE_TAG_BLUE "${IMAGE_TAG}"; set_env IMAGE_TAG_GREEN "${live_tag}"
else
  set_env IMAGE_TAG_GREEN "${IMAGE_TAG}"; set_env IMAGE_TAG_BLUE "${live_tag}"
fi

# --- Start the idle colour -----------------------------------------------------
compose pull "${IDLE_SERVICE}"
compose up -d --no-deps --no-build --force-recreate "${IDLE_SERVICE}"

healthy=0
deadline=$((SECONDS + HEALTH_TIMEOUT))
while (( SECONDS < deadline )); do
  status="$(docker inspect -f '{{.State.Health.Status}}' "${IDLE_CONTAINER}" 2>/dev/null || echo missing)"
  if [[ "${status}" == healthy ]]; then healthy=1; break; fi
  if [[ "${status}" == missing || "${status}" == unhealthy ]] || ! is_running "${IDLE_CONTAINER}"; then break; fi
  sleep 2
done
if [[ "${healthy}" -ne 1 ]]; then
  echo "ERROR: ${IDLE_CONTAINER} did not become healthy; leaving ${LIVE:-the legacy container} untouched" >&2
  docker logs --tail 50 "${IDLE_CONTAINER}" >&2 || true
  docker stop -t 10 "${IDLE_CONTAINER}" >/dev/null 2>&1 || true
  docker rm "${IDLE_CONTAINER}" >/dev/null 2>&1 || true
  set_env "IMAGE_TAG_$(echo "${IDLE}" | tr '[:lower:]' '[:upper:]')" "${live_tag}"
  exit 1
fi
if ! curl -fsS --max-time 5 -o /dev/null "http://127.0.0.1:${IDLE_PORT}/"; then
  echo "ERROR: ${IDLE_CONTAINER} is healthy but does not answer on 127.0.0.1:${IDLE_PORT}" >&2
  docker stop -t 10 "${IDLE_CONTAINER}" >/dev/null 2>&1 || true
  docker rm "${IDLE_CONTAINER}" >/dev/null 2>&1 || true
  set_env "IMAGE_TAG_$(echo "${IDLE}" | tr '[:lower:]' '[:upper:]')" "${live_tag}"
  exit 1
fi
echo "${IDLE_CONTAINER} healthy on 127.0.0.1:${IDLE_PORT}"

# --- Make sure edge lists both ports (first deploy) and has seen the new colour ---
# One file. Do not rewrite the shared Caddyfile or any other site. Installed
# before the old container goes away: on the first deploy the previous snippet
# only knows the legacy port.
install -d "${EDGE_SITES}"
install -m 644 "${SNIPPET}" "${EDGE_SITES}/${SITE_NAME}"
docker exec "${EDGE_CONTAINER}" caddy reload --config /etc/caddy/Caddyfile
sleep "${CADDY_SETTLE}"

# --- Retire the old colour -----------------------------------------------------
if [[ -n "${LIVE}" ]]; then
  OLD_CONTAINER="$(container_of "${LIVE}")"
elif [[ "${LEGACY}" -eq 1 ]]; then
  OLD_CONTAINER="${LEGACY_CONTAINER}"
else
  OLD_CONTAINER=""
fi
if [[ -n "${OLD_CONTAINER}" ]]; then
  echo "Draining ${OLD_CONTAINER} (up to ${DRAIN_SECONDS}s)"
  docker stop -t "${DRAIN_SECONDS}" "${OLD_CONTAINER}" >/dev/null
  docker rm "${OLD_CONTAINER}" >/dev/null
fi
printf '%s\n' "${IDLE}" > "${STATE_FILE}"
echo "Landing live on ${IDLE} (127.0.0.1:${IDLE_PORT}), tag ${IMAGE_TAG}"
