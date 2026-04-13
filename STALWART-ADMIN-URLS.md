# Stalwart Webadmin Settings URL Map

Base: `https://mail-server-1-e46bb9.cloud-claude.com/settings`

| Path | Label | Settings key prefix |
|---|---|---|
| `/acme` | ACME Providers | `acme.*` |
| `/alerts` | Alerts | `metrics.alerts.*` |
| `/allowed-ip` | Allowed IPs | `server.allowed-ip.*` |
| `/arc/edit` | ARC Sealing | `auth.arc.*` |
| `/authentication/edit` | Authentication | `storage.directory`, `authentication.*` |
| `/auto-ban/edit` | Automatic Ban | `server.auto-ban.*` |
| `/blocked-ip` | Blocked IPs | `server.blocked-ip.*` |
| `/cache/edit` | Cache | `cache.*` |
| `/calendar/edit` | Calendar Settings | `ical.*` |
| `/certificate` | Certificates | `certificate.*` |
| `/cluster/edit` | Cluster | `cluster.*` |
| `/contacts/edit` | Contacts Settings | `carddav.*` |
| `/directory` | Directories | `directory.*` |
| `/dkim/edit` | DKIM Signing | `auth.dkim.*` |
| `/dmarc/edit` | DMARC Verification | `auth.dmarc.*` |
| `/email-folders/edit` | Default Folders | `email.default-folders.*` |
| `/enterprise/edit` | Enterprise | `enterprise.*` |
| `/http-form/edit` | Form submission | `http.request.*` |
| `/http-lookup` | HTTP lists | `http-lookup.*` |
| `/http-rate-limit/edit` | Rate Limits | `http.request.*` |
| `/http-security/edit` | Security | `http.request.*` |
| `/http-settings/edit` | HTTP Settings | `http.request.*` |
| `/imap-settings/edit` | IMAP Settings | `imap.*` |
| `/jmap-limits/edit` | JMAP Protocol Limits | `jmap.request.*` |
| `/jmap-push/edit` | JMAP Push Notifications | `jmap.push-subscription.*` |
| `/jmap-web-sockets/edit` | JMAP Web Sockets | `jmap.web-socket.*` |
| `/listener` | Listeners | `server.listener.*` |
| `/metrics/edit` | Metrics | `metrics.*` |
| `/milter` | Milters | `session.milter.*` |
| `/mta-hooks` | MTA Hooks | `session.hook.*` |
| `/network/edit` | Network | `server.*` |
| `/oauth/edit` | OAuth | `oauth.*` |
| `/openid/edit` | OpenID Connect | `oauth.oidc.*` |
| `/report-analysis/edit` | Inbound Report Analysis | `report.analysis.*` |
| `/report-dkim/edit` | DKIM Reporting | `report.dkim.*` |
| `/report-dmarc/edit` | DMARC Failure Reporting | `report.dmarc.*` |
| `/report-dsn/edit` | DSN Signing | `report.dsn.*` |
| `/report-outbound/edit` | **Outbound Reports** (`report.domain`, `report.submitter`) | `report.domain`, `report.submitter` |
| `/report-spf/edit` | SPF Failure Reporting | `report.spf.*` |
| `/report-tls/edit` | TLS Aggregate Reporting | `report.tls.*` |
| `/scheduling/edit` | Scheduling | `ical.scheduling.*` |
| `/sharing/edit` | CalDAV Sharing | `caldav.sharing.*` |
| `/sieve-limits/edit` | Sieve Limits | `sieve.limits.*` |
| `/sieve-settings/edit` | Sieve Settings | `sieve.*` |
| `/signature` | DKIM Signatures | `signature.*` |
| `/smtp-in-asn/edit` | ASN & GeoIP | `smtp.session.*` |
| `/smtp-in-auth/edit` | AUTH Stage | `smtp.session.auth.*` |
| `/smtp-in-connect/edit` | Connect Stage | `smtp.session.connect.*` |
| `/smtp-in-data/edit` | DATA Stage | `smtp.session.data.*` |
| `/smtp-in-ehlo/edit` | EHLO Stage | `smtp.session.ehlo.*` |
| `/smtp-in-extensions/edit` | Extensions | `smtp.session.extensions.*` |
| `/smtp-in-limits/edit` | Session Limits | `smtp.session.limits.*` |
| `/smtp-in-mail/edit` | MAIL FROM Stage | `smtp.session.mail.*` |
| `/smtp-in-mta-sts/edit` | MTA-STS | `smtp.session.mta-sts.*` |
| `/smtp-in-rcpt/edit` | RCPT TO Stage | `smtp.session.rcpt.*` |
| `/smtp-in-throttle` | Inbound Rate Limits | `queue.limiter.inbound.*` |
| `/smtp-out-connection` | Outbound Connection | `queue.connection.*` |
| `/smtp-out-queues` | Virtual Queues | `queue.virtual.*` |
| `/smtp-out-quota` | Queue Quotas | `queue.quota.*` |
| `/smtp-out-resolver/edit` | DNS Resolver | `resolver.*` |
| `/smtp-out-routing` | Outbound Routing | `queue.route.*` |
| `/smtp-out-scheduling` | Outbound Scheduling | `queue.schedule.*` |
| `/smtp-out-strategy/edit` | Outbound Strategies | `queue.strategy.*` |
| `/smtp-out-throttle` | Outbound Rate Limits | `queue.limiter.outbound.*` |
| `/smtp-out-tls` | Outbound TLS | `queue.tls.*` |
| `/spam-*` (many) | Spam filter pages | `spam-filter.*`, `lookup.*` |
| `/spf/edit` | SPF Verification | `auth.spf.*` |
| `/storage/edit` | Storage | `storage.*` |
| `/store` | Stores | `store.*` |
| `/system/edit` | System | `config.local-keys`, `global.thread-pool` |
| `/telemetry-history/edit` | Telemetry History | `telemetry.http-push.*` |
| `/tls/edit` | TLS Defaults | `server.tls.*` |
| `/tracing` | Logging & Tracing | `tracer.*` |
| `/trusted-script` | Trusted Sieve Scripts | `sieve.trusted.scripts.*` |
| `/untrusted-script` | User Sieve Scripts | `sieve.untrusted.scripts.*` |
| `/web-hooks` | Webhooks | `webhook.*` |
| `/webdav/edit` | WebDAV | `webdav.*` |
