# Local meets (MoQ) dev setup

Meets requires **HTTPS** for pages and a **MoQ relay** listener for media (QUIC/WebTransport with WebSocket fallback).

## Architecture

```
Browser https://<host>:42071  (pages, HTMX)
    |
    +--> Caddy :42071 ------> Lariv HTTP (dev/lariv.sock)

Browser https://<host>:4433/meets/{code}?jwt=...  (media)
    |
    +--> Lariv embedded moq-relay on :4433 (not via Caddy)
```

Pages and media use **different ports**. The call page mints a MoQ JWT; the browser passes it as `?jwt=` when dialing the relay.

## One-time setup

```bash
# From deployments/kds_tagore-rs/
./dev/setup-certs.sh
```

Install system packages if needed:

- **mkcert** — trusted local TLS certificates
- **caddy** — HTTPS reverse proxy for pages

## Run

```bash
# From deployments/kds_tagore-rs/
./dev/serve.sh
```

Open **https://localhost:42071** on this machine.

## LAN / other devices

1. Re-run `./dev/setup-certs.sh` (includes your LAN IP in the cert).
2. Set `meets.transport.publicUrl = "https://<your-lan-ip>:4433"` in `config.toml`.
3. Open `https://<your-lan-ip>:42071` in the browser.

## Verify

1. Lariv logs: `meets: MoQ relay listening addr=[::]:4433`
2. Join a meeting call — no MoQ connection errors in the console
3. Camera preview works; remote tiles appear with a second browser/tab

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| MoQ connection fails after restart | Hard-refresh the call page to get a fresh JWT. Ensure `authKeyFile` exists (`./dev/setup-certs.sh`). |
| `SSL received a record that exceeded...` on `:42072` | That port is unused. Use `https://localhost:42071` for pages. |
| Connection refused on `:42071` | Start `./dev/serve.sh` (Caddy must be running). |
| Connection refused on `:4433` | Check Lariv logs for MoQ relay; ensure UDP/TCP 4433 is not blocked. |
| Auth fails on relay | Refresh the call page to get a new `?jwt=` URL. |
| Cert warnings on LAN | Re-run `./dev/setup-certs.sh`; trust mkcert CA on the client device. |
| Firefox / Safari | MoQ client falls back to WebSocket automatically when WebTransport is unavailable. |

## Config reference

See [`../config.toml`](../config.toml):

- `uds = "dev/lariv.sock"` — internal HTTP (Caddy only)
- `[meets.transport]` — moq-relay on `[::]:4433` with `dev/certs/*`
- `publicUrl = "https://127.0.0.1:4433"` — browser MoQ relay origin
- `authKeyFile = "dev/certs/moq-auth.jwk"` — stable JWT signing key for local dev
