#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

release_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
app_dir=$(cd -- "$release_dir/../.." && pwd)
exec 9>"$app_dir/deploy.lock"
flock -w 300 9 || { echo "Another deployment is still running" >&2; exit 1; }

# Docker must already be installed by the one-time EC2 bootstrap.
docker info >/dev/null
docker compose version
registry_config=$(mktemp -d)
# Keep user-installed Compose plugins discoverable with the temporary credentials directory.
printf '{"cliPluginsExtraDirs":["%s/.docker/cli-plugins"]}\n' "$HOME" > "$registry_config/config.json"
export DOCKER_CONFIG=$registry_config
trap 'rm -rf -- "$registry_config"; rm -f -- "$release_dir/bundle.tgz"' EXIT
IFS= read -r registry_token
registry_user=$(sed -n 's/^REGISTRY_USER=//p' "$release_dir/release.env")
printf '%s' "$registry_token" | docker login ghcr.io --username "$registry_user" --password-stdin
unset registry_token

compose() {
  local directory=$1
  shift
  docker compose --project-name secure-chat --project-directory "$directory" \
    --env-file "$directory/release.env" -f "$directory/compose.yml" "$@"
}

compose "$release_dir" config --quiet
# Pull everything before replacing running containers.
compose "$release_dir" pull
previous=''
if [[ -L "$app_dir/current" ]]; then previous=$(readlink -f "$app_dir/current"); fi

rollback() {
  echo "Deployment failed; restoring the previous release" >&2
  compose "$release_dir" ps >&2 || true
  # Do not dump container logs or config: either may contain credentials.
  if [[ -n "$previous" && -d "$previous" ]]; then
    compose "$previous" up -d --remove-orphans --wait --wait-timeout 180 || \
      echo "Rollback also failed; inspect Docker Compose on EC2" >&2
  else
    echo "No earlier release is available to restore" >&2
  fi
}

if ! compose "$release_dir" up -d --remove-orphans --wait --wait-timeout 180; then
  rollback
  exit 1
fi

app_origin=$(sed -n 's/^APP_ORIGIN=//p' "$release_dir/release.env")
# Validate public routing as well as individual container health.
# resolve keeps the health check on this EC2 instance even with external DNS/proxies.
address=${app_origin#*://}
port=443
[[ "$app_origin" == http://* ]] && port=80
if ! curl --fail --silent --show-error --retry 10 --retry-delay 6 --retry-all-errors \
  --connect-timeout 5 --max-time 15 --resolve "$address:$port:127.0.0.1" "$app_origin/ping" >/dev/null || \
  ! curl --fail --silent --show-error --connect-timeout 5 --max-time 15 \
    --resolve "$address:$port:127.0.0.1" "$app_origin/" >/dev/null; then
  rollback
  exit 1
fi

ln -sfn -- "$release_dir" "$app_dir/current"
echo "Deployed $(basename -- "$release_dir") to $app_origin"
