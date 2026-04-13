# Mail Server Deliverability — Consolidated Status

Goal: `oz@oznakash.com` forwards to Gmail Inbox (not Spam), clean auth, production-ready operations.

---

## ✅ Done

- [x] Forwarding pipeline working (Sieve trusted script `forward-to-gmail`)
- [x] Stalwart DKIM signatures created: RSA + Ed25519 for both oznakash.com and naka.sh
- [x] oznakash.com DNS complete: MX, A, SPF (`-all`), DKIM (both selectors), DMARC (`p=quarantine`)
- [x] Hetzner port 25 unblocked (outbound SMTP works)
- [x] DKIM Signing expression in Stalwart is dynamic (`is_local_domain` pattern) — Google Workspace-style
- [x] `report.domain` changed from `naka.sh` → `oznakash.com` (Test 8: `dkim=pass header.d=oznakash.com` 🎉)
- [x] DMARC passes for forwarded mail
- [x] Google Postmaster Tools DNS verification record added (pending Google side)
- [x] URL map of Stalwart admin pages saved at `STALWART-ADMIN-URLS.md`

---

## 🔴 Critical — blocking Inbox placement

| # | Action | Status | Next step |
|---|--------|--------|-----------|
| A | **Confirm Test 8 landed in Inbox vs. Spam** | ⏳ pending | User checks Gmail and reports |
| B | **Fix HELO SPF: change hostname to `mail.oznakash.com`** (Option B over SPF for cloud-claude.com) | ☐ | 1. Add A record `mail.oznakash.com → 5.78.187.44` at GoDaddy<br>2. Change Stalwart hostname at `/settings/network/edit`<br>3. Update Hetzner PTR to `mail.oznakash.com`<br>4. Save & Reload |
| C | **ARC seal not firing from mail.cloud-claude.com** | ☐ | Go to `/settings/arc/edit`, click Save & Reload explicitly. If still no ARC-Seal in test headers, investigate whether signatures need type=ARC flag |

---

## 🟡 High — strong deliverability/security gains

| # | Action | Status | Notes |
|---|--------|--------|-------|
| D | **TLS certificate via ACME (Let's Encrypt)** | ☐ | `/settings/acme`. Needs port 80 reachable. After hostname change to mail.oznakash.com, include it in ACME domains list |
| E | **RFC 2142 aliases**: `postmaster@`, `abuse@`, `hostmaster@` → forward to oz@oznakash.com | ☐ | Many receivers expect these; bounces/abuse reports go here |
| F | **Verify DMARC `rua=postmaster@oznakash.com` mailbox actually receives** | ☐ | Tied to E above |
| G | **Admin panel IP whitelist** (`/settings/allowed-ip`) | ☐ | Currently publicly reachable — credential-stuffing risk |
| H | **Auto-ban on failed auth** (`/settings/auto-ban/edit`) | ☐ | Blocks brute-force after N failures |
| I | **Backup strategy** for Stalwart RocksDB + configs | ☐ | Daily rsync or snapshot — catastrophic loss risk otherwise |

---

## 🟢 Medium — nice hygiene

| # | Action | Status | Notes |
|---|--------|--------|-------|
| J | **Fix Ed25519 DKIM "no key"** — drop `h=sha256` tag from DNS record | ☐ | Currently `v=DKIM1; k=ed25519; h=sha256; p=...` — try without `h=sha256`. RSA already passes so not blocking |
| K | **MTA-STS policy publication** for oznakash.com | ☐ | Pairs with TLS-RPT. Signals legit operator status |
| L | **TLS-RPT record** | ☐ | TXT at `_smtp._tls.oznakash.com` — gets reports of TLS failures |
| M | **Enable Stalwart metrics** (`/settings/metrics/edit`) | ☐ | Visibility into queue, auth failures, delivery rate |
| N | **Rate-limit inbound SMTP** (`/settings/smtp-in-throttle`) | ☐ | Usually defaults are fine — just verify |
| O | **Disable unused listeners** (POP3/JMAP if unused) | ☐ | Reduce attack surface |

---

## 🟢 Low / Future

| # | Action | Status | Notes |
|---|--------|--------|-------|
| P | **Migrate naka.sh from ImprovMX to Stalwart** | 🅿️ parked | User explicitly paused until oznakash.com is fully stable |
| Q | **Tighten DMARC `p=quarantine` → `p=reject`** | ☐ | After 1-2 weeks of clean aggregate reports |
| R | **DMARC report parser** (Dmarcian, Postmark, or similar) | ☐ | Makes rua= XML reports readable |
| S | **DKIM key rotation plan** (annual) | ☐ | Add new selector, dual-sign for 30 days, retire old |
| T | **DANE/TLSA records** | ☐ | Advanced; only if you care about nation-state MITM |
| U | **Reputation warm-up** (mark "Not spam" in Gmail, send steady low volume) | 🔄 passive | Takes 2-4 weeks on new IPs |

---

## Parked / non-goals

- Snappymail webmail integration (ROADMAP Phase 2 — deferred)
- BIMI logo display (requires $1,500/yr VMC — overkill)
- ARC expression going dynamic (industry standard is static operator domain; keep as-is)

---

## Recommended execution order

**Today / this session:**
1. Confirm Test 8 result (A)
2. Fix HELO via hostname change (B) — combines SPF pass + cleaner identity + sets up ACME later
3. ARC seal firing (C) — click Save & Reload explicitly
4. TLS cert via ACME (D) — after B so cert includes the right hostname
5. Postmaster aliases (E) + DMARC rua verify (F)
6. Admin IP whitelist (G) + auto-ban (H)

**This week:**
- Backup strategy (I)
- MTA-STS + TLS-RPT (K, L)
- Fix Ed25519 (J)

**Next 1-2 weeks:**
- Monitor DMARC reports
- Tighten to `p=reject` (Q)

**Later:**
- naka.sh migration (P) when ready
- Everything else
