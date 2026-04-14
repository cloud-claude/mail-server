# Handoff: Stalwart Webadmin returning 502 Bad Gateway

> **For the CTO of cloud-claude**: The Stalwart Mail Server webadmin is unreachable (HTTP 502 from Traefik) on the shared infra host. Mail (SMTP 25/465/587/993) is still working. The Stalwart process is running and listeners are bound, but all traffic from Traefik → Stalwart is being dropped by Stalwart's built-in IP auto-ban. Detailed diagnosis and a proposed fix are below. Please review the proposed fix and — if you agree — execute it, or propose a better one. I specifically want a sanity check on the Traefik ↔ Stalwart client-IP propagation fix before we make it permanent.

---

## Environment

- **Host**: Hetzner VM, `5.78.187.44`, hostname `hello-world-1`
- **Affected URL (502)**: https://mail-server-1-e46bb9.cloud-claude.com/
- **Admin panel URL (same)**: https://mail-server-1-e46bb9.cloud-claude.com/login
- **TLS cert**: Valid Let's Encrypt (issued Apr 8 2026, expires Jul 7 2026) — terminates at Traefik
- **Still working**: inbound SMTP on 25, IMAPS 993, submission 587 (direct DNAT host→container, bypasses Traefik)
- **Mailbox tested**: `contact@cloud-claude.com` → received a test mail from `oznakash2@gmail.com` at 04:20 UTC (authentication all passed, message ingested cleanly)

### Container topology

| Container | Image | Role | Networks |
|---|---|---|---|
| `cloud-claude-traefik` | `traefik:v3` | TLS termination + reverse proxy on :80/:443 | `traefik-public` (172.18.0.2) |
| `cc-e70c0625867d4509a8951294bad67d1c-app` | `stalwartlabs/stalwart:v0.15` | Mail server + webadmin | `cc-e70c…-net` (172.20.0.2) **and** `traefik-public` (172.18.0.3) |
| `cloud-claude-error-pages` | `nginx:alpine` | 502/404 pages | — |
| `cloud-claude-web` / `cloud-claude-api` / `cloud-claude-db` | — | Unrelated app stack | — |

Stalwart exposes :443 and :8080 only on the internal docker networks (not DNAT'd to host). Traefik proxies public `mail-server-1-e46bb9.cloud-claude.com` → `cc-e70c…-app` on the `traefik-public` network.

---

## What was happening before the failure

Deliverability work on domain `cloud-claude.com`:
- Added MX record → `mail.cloud-claude.com.`
- Added DKIM RSA + Ed25519 TXT records
- Added HELO SPF TXT (`mail.cloud-claude.com`)
- Removed 1 duplicate SPF TXT and 1 duplicate DMARC TXT at root
- Created a new mailbox `contact@cloud-claude.com` in Stalwart, added a forwarding Sieve rule to Gmail
- Stalwart container was **restarted** by the user shortly before the 502 started (to verify state)

No Stalwart config was edited during the DNS work. No Traefik config changes. No firewall changes.

---

## Initial hypothesis (wrong)

Because the 502 appeared right after the DNS changes, we suspected:
1. GoDaddy SOA update → brief NXDOMAIN window for `*.cloud-claude.com` → Traefik lost its backend
2. ACME cert renewal failure due to DNS inconsistency
3. Stalwart internal `tls.implicit = true` failing because of DNS-dependent cert issuance

We ruled these out:
- `dig +short mail-server-1-e46bb9.cloud-claude.com` → `5.78.187.44` (correct)
- `curl -vk --resolve ...:443:5.78.187.44 https://mail-server-1-e46bb9.cloud-claude.com/` → TLS handshake succeeds with valid LE cert, then HTTP/2 returns `502 Bad Gateway` from the **reverse proxy** (Traefik)
- Stalwart process is running inside the container (PID 1602947, started 04:29 UTC)
- Stalwart's TCP listeners ARE bound inside the container (verified via `/proc/net/tcp6`): `:443`, `:8080`, `:25`, `:465`, `:587`, `:993`, `:143`, `:995`, `:110`, `:4190`

So DNS was **not** the cause.

---

## Actual root cause (confirmed)

**Stalwart's auto-ban feature has banned Traefik's Docker IP `172.18.0.2`.**

Evidence from Stalwart's log (`/opt/stalwart/logs/stalwart.log.2026-04-14` inside container):

```
2026-04-14T04:09:32Z INFO Banned due to scan (security.scan-ban)
  listenerId = "http", localPort = 8080,
  remoteIp = 172.18.0.2, path = "/mail/phpinfo.php"
```

Then thousands of:
```
2026-04-14T04:51:50Z INFO Blocked IP address (security.ip-blocked)
  listenerId = "http", localPort = 8080, remoteIp = 172.18.0.2
```

### Why this is happening

1. An internet scanner hit `https://mail-server-1-e46bb9.cloud-claude.com/mail/phpinfo.php`
2. Traefik terminated TLS and forwarded the HTTP request to Stalwart over the Docker network
3. Stalwart saw the request with `remoteIp = 172.18.0.2` (**Traefik's IP**, not the real attacker's IP — because Traefik is not passing real client IP to Stalwart in a way Stalwart trusts for its scan-ban logic)
4. Stalwart's `security.scan-ban` rule matched `/mail/phpinfo.php` and banned the source (Traefik)
5. All subsequent legitimate traffic via Traefik → Stalwart is now dropped → Traefik returns 502 Bad Gateway

**Bans are persisted in RocksDB** — they survive container restart.

SMTP still works because SMTP listeners on 25/465/587/993 are DNAT'd from the host directly to the container, bypassing Traefik entirely. So Stalwart sees real external client IPs for mail and is unaffected.

---

## Why a restart "caused" it

It didn't. The ban was laid at 04:09 UTC. Once Traefik's IP was banned, the admin panel began 502'ing immediately. The restart later was just the user verifying state. The temporal correlation with DNS changes is coincidence.

---

## Attempted fix (failed)

We tried adding allow-list entries to `/opt/stalwart/etc/config.toml`:

```toml
server.allowed-ip.0000 = "172.18.0.0/16"
server.allowed-ip.0001 = "172.20.0.0/16"
```

Stalwart rejected this at load with:
```
ERROR Configuration parse error
  details = "Failed to parse setting \"server.allowed-ip\": Invalid IP address value \"0001\"."
WARN Configuration build warning
  details = "WARNING for \"server.allowed-ip.0000\": Database key defined in local configuration, this might cause issues."
```

Two problems:
1. `server.allowed-ip.*` is a **database-managed setting** (per Stalwart's config docs). Setting it in `config.toml` is unsupported.
2. Even if it were supported, bans live in RocksDB and would not be invalidated by an allowlist added later.

Those entries have been removed from `config.toml` and the container restarted. Config is back to the known-good state.

---

## Proposed fix

### Short-term (unblock admin panel)

Access the Stalwart management API **from inside the container via `localhost`** (source IP 127.0.0.1 is not banned), then:

1. **Unban Traefik's IP** via `DELETE /api/security/unban/172.18.0.2` (or equivalent Stalwart API endpoint — exact path needs verification against Stalwart v0.15.5 API)
2. **Add `172.18.0.0/16` to the allowed-IP list** via the management API (writes to RocksDB), not via `config.toml`
3. Verify admin panel loads

Concretely:
```bash
# Needs the admin password for fallback admin user
docker exec cc-e70c0625867d4509a8951294bad67d1c-app \
  wget -qO- --user=admin --password="<admin-password>" \
  http://localhost:8080/api/ping
```

If `/api/ping` returns OK from localhost, we can proceed to the unban call. Exact API path for unban/allowlist in v0.15.5: need to confirm from Stalwart docs or source (`crates/http/src/management/`).

### Long-term (prevent recurrence)

The core architectural bug: **Traefik does not propagate real client IP in a way Stalwart's scan-ban honors.** Options:

**Option A — PROXY protocol (recommended)**
- Configure Traefik to send PROXY protocol v2 to Stalwart's HTTP/HTTPS backends
- Configure Stalwart listener to accept PROXY protocol (`server.listener.http.proxy-networks` or similar in Stalwart config)
- Stalwart then sees the real client IP, auto-ban targets the real attacker, and Traefik's IP is never banned

**Option B — Trusted X-Forwarded-For**
- Traefik forwards `X-Forwarded-For` by default
- Configure Stalwart to trust `X-Forwarded-For` from `172.18.0.0/16` only (via `http.request.trusted-networks` or similar setting)
- Stalwart uses the header's client IP for scan-ban

**Option C — Allowlist Traefik's subnet**
- Add `172.18.0.0/16` to Stalwart's allowed-IP list via the webadmin (writes to RocksDB properly, unlike our failed `config.toml` attempt)
- Stalwart will never ban Traefik, but real attackers will still be punished because their IP is masked as 172.18.0.2 — so this does not prevent abuse, it just doesn't break legit traffic. **This is a workaround, not a fix.**

Option A is the clean fix. Option B is acceptable if PROXY protocol is hard to wire. Option C alone is insufficient because Stalwart will have no way to ban real attackers.

---

## Key runtime facts

- Stalwart version: `0.15.5`
- Stalwart process: `PID 1602947`, `/usr/local/bin/stalwart --config /opt/stalwart/etc/config.toml`
- Config file: `/opt/stalwart/etc/config.toml` (verified unchanged now)
- Log file: `/opt/stalwart/logs/stalwart.log.2026-04-14`
- Listeners bound inside container (`/proc/net/tcp6`):
  - `01BB` (443) ✓
  - `1F90` (8080) ✓
  - `0019` (25) ✓
  - `024B` (587) ✓
  - `03E1` (993) ✓
  - others...
- Traefik IP: `172.18.0.2` on network `traefik-public`
- Stalwart IP on `traefik-public`: `172.18.0.3`
- Stalwart IP on `cc-e70c…-net`: `172.20.0.2`
- `ACME authentication error` lines seen — Stalwart's own ACME for `mail.cloud-claude.com` is failing (`tls-alpn-01` challenge). **Unrelated to the 502** but worth noting: Stalwart has no TLS cert for its internal :443 listener (`No TLS certificates available total = 0` warnings). Traefik handles external TLS so this does not break the public URL, but it means internal SMTP STARTTLS is falling back to no cert.

---

## Relevant logs (excerpt)

```
2026-04-14T04:09:32Z INFO Banned due to scan (security.scan-ban) listenerId = "http", localPort = 8080, remoteIp = 172.18.0.2, path = "/mail/phpinfo.php"

2026-04-14T04:29:30Z INFO Starting Stalwart Server v0.15.5 (server.startup) version = "0.15.5"
2026-04-14T04:29:30Z INFO Network listener started (network.listen-start) listenerId = "https", localIp = ::, localPort = 443, tls = true
2026-04-14T04:29:30Z INFO Network listener started (network.listen-start) listenerId = "http", localIp = ::, localPort = 8080, tls = false
...
2026-04-14T04:51:41Z ERROR Configuration parse error (config.parse-error) details = "Failed to parse setting \"server.allowed-ip\": Invalid IP address value \"0001\"."
2026-04-14T04:51:41Z WARN Configuration build warning details = "WARNING for \"server.allowed-ip.0000\": Database key defined in local configuration"
...
2026-04-14T04:51:50Z INFO Blocked IP address (security.ip-blocked) listenerId = "http", localPort = 8080, remoteIp = 172.18.0.2
2026-04-14T04:51:54Z INFO Blocked IP address (security.ip-blocked) listenerId = "http", localPort = 8080, remoteIp = 172.18.0.2
... (many more)
```

Good signal that mail itself is unaffected:
```
2026-04-14T04:20:42Z INFO SMTP EHLO command ... remoteIp = 209.85.219.53 ... domain = "mail-qv1-f53.google.com"
2026-04-14T04:20:42Z INFO DKIM verification passed
2026-04-14T04:20:42Z INFO DMARC check passed
2026-04-14T04:20:46Z INFO Message ingested (message-ingest.ham) from = "oznakash2@gmail.com", to = ["contact@cloud-claude.com"]
2026-04-14T04:20:46Z INFO Delivery completed
```

---

## Request to the CTO

1. Confirm the root-cause analysis above (scan-ban of Traefik's IP).
2. Choose between Options A / B / C for the permanent fix.
3. Execute the unban + allowlist against the Stalwart management API, or grant access so we can.
4. If PROXY protocol (Option A) is the chosen direction, I'd like a pointer to how Traefik is configured in this stack (IaC repo, Docker labels, or dynamic config file) so we can wire it cleanly on both sides.

Thanks.
