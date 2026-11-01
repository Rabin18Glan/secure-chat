#!/usr/bin/env bash
# Run once on the target Amazon Linux EC2 instance as ec2-user.
set -euo pipefail

if ! command -v docker >/dev/null; then
  if command -v dnf >/dev/null; then
    sudo dnf install -y docker curl tar util-linux
  elif command -v yum >/dev/null; then
    sudo yum install -y docker curl tar util-linux
  else
    echo "This bootstrap targets Amazon Linux. Install Docker Engine and Compose v2 for your OS." >&2
    exit 1
  fi
fi
sudo systemctl enable --now docker
sudo usermod -aG docker "$(id -un)"

# Install Compose system-wide, including when deploy.sh uses a temporary Docker config.
if ! docker compose version >/dev/null 2>&1; then
  case "$(uname -m)" in
    x86_64) architecture=x86_64 ;;
    aarch64) architecture=aarch64 ;;
    *) echo "Unsupported CPU architecture" >&2; exit 1 ;;
  esac
  compose_version=v2.40.3
  plugin_dir=/usr/local/lib/docker/cli-plugins
  sudo mkdir -p "$plugin_dir"
  temporary_dir=$(mktemp -d)
  trap 'rm -rf -- "$temporary_dir"' EXIT
  base_url="https://github.com/docker/compose/releases/download/$compose_version"
  curl --fail --location --silent --show-error "$base_url/docker-compose-linux-$architecture" -o "$temporary_dir/docker-compose-linux-$architecture"
  curl --fail --location --silent --show-error "$base_url/checksums.txt" -o "$temporary_dir/checksums.txt"
  (cd "$temporary_dir" && awk -v file="docker-compose-linux-$architecture" '$2 == file || $2 == "*" file' checksums.txt | sha256sum --check --strict -)
  sudo install -m 0755 "$temporary_dir/docker-compose-linux-$architecture" "$plugin_dir/docker-compose"
fi
mkdir -p "$HOME/secure-chat/releases"
chmod 700 "$HOME/secure-chat"
echo "Bootstrap complete. Disconnect and reconnect so Docker group membership takes effect."
