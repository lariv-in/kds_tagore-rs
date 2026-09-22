#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
CERT_DIR="$ROOT/certs"
mkdir -p "$CERT_DIR"

if ! command -v mkcert >/dev/null 2>&1; then
  echo "mkcert is required. Install it, then re-run this script."
  echo "  Arch:   sudo pacman -S mkcert"
  echo "  macOS:  brew install mkcert"
  exit 1
fi

mkcert -install

CERT_NAMES=(localhost 127.0.0.1 ::1)
LAN_IP="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{for (i=1;i<=NF;i++) if ($i=="src") print $(i+1)}' || true)"
if [[ -n "${LAN_IP:-}" ]]; then
  CERT_NAMES+=("$LAN_IP")
fi

mkcert -cert-file "$CERT_DIR/cert.pem" -key-file "$CERT_DIR/key.pem" "${CERT_NAMES[@]}"

MOQ_KEY="$CERT_DIR/moq-auth.jwk"
if [[ ! -f "$MOQ_KEY" ]]; then
  python3 - <<'PY' >"$MOQ_KEY"
import base64, json, secrets

def b64url(n: bytes) -> str:
    return base64.urlsafe_b64encode(n).decode().rstrip("=")

kid = b64url(secrets.token_bytes(8))
k = b64url(secrets.token_bytes(32))
print(json.dumps({"kty": "oct", "kid": kid, "k": k, "alg": "HS256"}))
PY
  chmod 600 "$MOQ_KEY"
  echo "MoQ auth key written to $MOQ_KEY"
fi

echo ""
echo "Certificates written to $CERT_DIR"
echo "Trust is handled by mkcert (mkcert -install)."
