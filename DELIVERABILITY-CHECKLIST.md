# Deliverability Status & Action Plan

Last updated: during Test 7 analysis

## Where we are

Forwarding works end-to-end. Gmail accepts with 250 OK. Messages land in **spam** because outbound auth signals are incomplete.

---

## Action items — prioritized

| # | Action | Severity | Impact if skipped | Status |
|---|--------|----------|-------------------|--------|
| **1** | Fix signing domain: either change `report.domain` to `oznakash.com` **or** publish DKIM DNS records for `naka.sh` | 🔴 **Critical** | Every forward shows `dkim=permerror` and `arc=fail` → Gmail keeps flagging as spam | ⏳ waiting on choice |
| **2** | Publish SPF for `cloud-claude.com` | 🟡 **High** | Null-envelope forwards (the most common case) fail SPF at `Received-SPF` check → −1 to −2 mail-tester points, harder to reach inbox | ☐ |
| **3** | Configure ACME / valid TLS cert for all listeners | 🟡 **Medium** | Self-signed cert means IMAP/submission clients get warnings, MTA-STS strict senders may refuse delivery, mail-tester penalizes | ☐ |
| **4** | Verify `sender_domain` actually populates on forwards so DKIM expression picks `oznakash.com` | 🟡 **Medium** | If `sender_domain` is empty on forwards, DKIM falls back to `report.domain` anyway — fixes itself after #1 | ⏳ blocked by #1 |
| **5** | Migrate `naka.sh` off ImprovMX to Stalwart (roadmap Phase 1 requires this) | 🟢 **Low** (for now) | You don't have control or consistency across both domains; `oz@naka.sh` forwarding is handled by a third party | ☐ parked by user |
| **6** | Add DMARC aggregate reporting destination & later tighten `p=quarantine` → `p=reject` | 🟢 **Low** | No visibility into who's forging your domain; current `p=quarantine` is fine for now | ☐ future |
| **7** | Warm up IP reputation (send small volume consistently, mark "Not spam" in Gmail) | 🟢 **Passive** | New IPs take 2–4 weeks to build reputation; spam placement improves naturally with clean auth | ongoing |

---

## Green (already working — don't touch)

- ✅ Forward pipeline (Sieve redirect) — delivers to Gmail with 250 OK
- ✅ MX, A records, PTR for oznakash.com
- ✅ DKIM signatures generated for both domains in Stalwart
- ✅ DKIM DNS records published for oznakash.com (both RSA + Ed25519)
- ✅ SPF on oznakash.com (`-all` strict)
- ✅ DMARC on oznakash.com (`p=quarantine`)
- ✅ Inbound + outbound TLS 1.3 handshakes succeed
- ✅ MTA-STS policy fetched + verified on outbound
- ✅ Hetzner port 25 unblocked
- ✅ DKIM Signing expression in Stalwart is dynamic (`sender_domain` based) — correct pattern

---

## Parked / non-goals (documented for later)

- naka.sh migration from ImprovMX to Stalwart
- Snappymail webmail integration (ROADMAP Phase 2)
- ARC expression going dynamic (operator-domain pattern is industry-standard; OK to leave static)
- DMARC `p=reject` hardening (do after weeks of monitoring with `p=quarantine`)

---

## Decision pending

**User needs to choose on Action 1:**

| Option | Change | DNS work | Stalwart config change |
|--------|--------|----------|------------------------|
| **A. Keep naka.sh as operator** | Publish DKIM TXT for naka.sh at GoDaddy | 2 TXT records | none |
| **B. Switch operator to oznakash.com** | Change `report.domain` → `oznakash.com` | none | 1 field |
