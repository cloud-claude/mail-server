# Deliverability Checklist

Goal: move forwarded mail from Gmail spam → inbox.

Server: `mail.cloud-claude.com` (5.78.187.44)
Domains: `oznakash.com`, `naka.sh`, `cloud-claude.com`

## Status

| # | Step | Status |
|---|------|--------|
| 1 | Publish DKIM DNS records for `naka.sh` (2 records) | ☐ |
| 2 | Publish DKIM DNS records for `oznakash.com` (2 records) | ✓ done |
| 3 | Publish SPF TXT records for `oznakash.com`, `naka.sh`, `cloud-claude.com` | ⏳ oznakash.com done; naka.sh + cloud-claude.com pending |
| 4 | Publish DMARC TXT records for `oznakash.com`, `naka.sh` | ⏳ oznakash.com done (p=quarantine); naka.sh pending |
| 5 | Fix `No TLS certificates available` warning (ACME for all listeners) | ☐ |
| 6 | Configure DKIM signing rules so forwards for `oznakash.com` sign as `oznakash.com` (not `naka.sh`) | ☐ |

## Verification targets

- [ ] `dig TXT 202604r._domainkey.naka.sh` returns value
- [ ] `dig TXT 202604e._domainkey.naka.sh` returns value
- [x] `dig TXT 202604r._domainkey.oznakash.com` returns value (published, verify propagation)
- [x] `dig TXT 202604e._domainkey.oznakash.com` returns value (published, verify propagation)
- [x] `dig TXT oznakash.com` contains `v=spf1` (published: `v=spf1 ip4:5.78.187.44 -all`)
- [ ] `dig TXT naka.sh` contains `v=spf1`
- [ ] `dig TXT cloud-claude.com` contains `v=spf1`
- [x] `dig TXT _dmarc.oznakash.com` returns DMARC (published: `p=quarantine`)
- [ ] `dig TXT _dmarc.naka.sh` returns DMARC
- [ ] mail-tester.com score ≥ 9/10 from `oz@oznakash.com`
- [ ] Gmail test: from external sender → `oz@oznakash.com` → lands in **Inbox** (not Spam)
- [ ] Received headers show `dkim=pass`, `spf=pass`, `dmarc=pass`, `arc=pass`
