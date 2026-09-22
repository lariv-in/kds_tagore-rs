#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEV="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

if [[ ! -f "$DEV/certs/cert.pem" || ! -f "$DEV/certs/key.pem" ]]; then
  echo "Missing dev TLS certs. Run: ./dev/setup-certs.sh"
  exit 1
fi

if [[ ! -f "$DEV/certs/moq-auth.jwk" ]]; then
  echo "Missing MoQ auth key. Run: ./dev/setup-certs.sh"
  exit 1
fi

if ! command -v caddy >/dev/null 2>&1; then
  echo "caddy is required. Install it, then re-run."
  echo "  Arch:   sudo pacman -S caddy"
  echo "  macOS:  brew install caddy"
  exit 1
fi

export LARIV_SOCK="$(realpath "$DEV/lariv.sock")"
export DEV_CERT_DIR="$DEV/certs"
rm -f "$LARIV_SOCK"

echo "Validating Caddy config ..."
if ! caddy adapt --config "$DEV/Caddyfile" >/dev/null; then
  echo "Caddy config is invalid. Fix dev/Caddyfile and re-run."
  exit 1
fi

echo "Starting Caddy (https://0.0.0.0:42071) ..."
caddy run --config "$DEV/Caddyfile" &
CADDY_PID=$!

cleanup() {
  kill "$CADDY_PID" 2>/dev/null || true
  rm -f "$LARIV_SOCK"
}
trap cleanup EXIT INT TERM

echo "Starting kds-tagore-rs (HTTP unix socket, MoQ relay [::]:4433) ..."
LAN_IP="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{for (i=1;i<=NF;i++) if ($i=="src") print $(i+1)}' || true)"
echo ""
echo "  Pages:  https://localhost:42071"
echo "  MoQ:    https://127.0.0.1:4433/meets/{code}  (direct, not via Caddy)"
if [[ -n "${LAN_IP:-}" ]]; then
  echo "  LAN pages: https://${LAN_IP}:42071"
  echo "  LAN MoQ:   set publicUrl = \"https://${LAN_IP}:4433\" in config.toml"
fi
echo ""
cargo run -- serve
