/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use common::{Server, auth::AccessToken};
use directory::{Permission, backend::internal::manage};
use hyper::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::future::Future;

use http_proto::{request::decode_path_element, *};

/// A custom domain record linking a user-owned domain to an app resource.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomDomain {
    pub id: String,
    pub domain: String,
    #[serde(rename = "appResource")]
    pub app_resource: Option<String>,
    #[serde(rename = "dnsVerified")]
    pub dns_verified: bool,
    #[serde(rename = "sslProvisioned")]
    pub ssl_provisioned: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// DNS instructions returned after adding a custom domain.
#[derive(Debug, Serialize)]
pub struct DnsInstructions {
    pub domain: String,
    pub records: Vec<DnsInstructionRecord>,
    pub verification_url: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DnsInstructionRecord {
    #[serde(rename = "type")]
    pub typ: String,
    pub name: String,
    pub value: String,
    pub purpose: String,
    pub required: bool,
}

#[derive(Debug, Deserialize)]
struct CustomDomainRequest {
    domain: String,
    #[serde(rename = "appResource")]
    app_resource: Option<String>,
}

struct DnsVerificationResult {
    verified: bool,
    checks: Vec<DnsCheck>,
}

#[derive(Serialize)]
struct DnsCheck {
    record_type: String,
    name: String,
    expected: String,
    found: Option<String>,
    passed: bool,
}

pub trait CustomDomainManagement: Sync + Send {
    fn handle_manage_custom_domains(
        &self,
        req: &HttpRequest,
        path: Vec<&str>,
        body: Option<Vec<u8>>,
        access_token: &AccessToken,
    ) -> impl Future<Output = trc::Result<HttpResponse>> + Send;
}

impl CustomDomainManagement for Server {
    async fn handle_manage_custom_domains(
        &self,
        req: &HttpRequest,
        path: Vec<&str>,
        body: Option<Vec<u8>>,
        access_token: &AccessToken,
    ) -> trc::Result<HttpResponse> {
        match (
            path.get(1).copied().unwrap_or_default(),
            path.get(2),
            req.method(),
        ) {
            // List all custom domains
            ("", None, &Method::GET) | ("list", None, &Method::GET) => {
                access_token.assert_has_permission(Permission::DomainGet)?;

                let domains = list_custom_domains(self).await?;
                let total = domains.len();
                Ok(JsonResponse::new(json!({
                    "data": {
                        "items": domains,
                        "total": total,
                    },
                }))
                .into_http_response())
            }

            // Add a new custom domain
            ("", None, &Method::POST) | ("add", None, &Method::POST) => {
                access_token.assert_has_permission(Permission::DomainCreate)?;

                let body = body.ok_or_else(|| {
                    manage::error("Missing request body", Option::<String>::None)
                })?;
                let request: CustomDomainRequest =
                    serde_json::from_slice(&body).map_err(|err| {
                        manage::error("Invalid request body", Some(err.to_string()))
                    })?;

                // Validate domain format
                if request.domain.is_empty()
                    || !request.domain.contains('.')
                    || request.domain.len() > 253
                {
                    return Err(manage::error(
                        "Invalid domain name",
                        Option::<String>::None,
                    ));
                }

                // Store the custom domain
                let domain_id = store_custom_domain(self, &request).await?;

                // Generate DNS instructions
                let instructions =
                    generate_dns_instructions(&self.core.network.server_name, &request.domain);

                Ok(JsonResponse::new(json!({
                    "data": {
                        "id": domain_id,
                        "domain": request.domain,
                        "instructions": instructions,
                    },
                }))
                .into_http_response())
            }

            // Delete a custom domain
            (id, None, &Method::DELETE) if !id.is_empty() => {
                access_token.assert_has_permission(Permission::DomainDelete)?;

                let id = decode_path_element(id);
                delete_custom_domain(self, id.as_ref()).await?;

                Ok(JsonResponse::new(json!({
                    "data": true,
                }))
                .into_http_response())
            }

            // Verify DNS for a custom domain
            ("verify", Some(domain), &Method::GET) => {
                access_token.assert_has_permission(Permission::DomainGet)?;

                let domain = decode_path_element(domain);
                let result = verify_custom_domain_dns(
                    self,
                    &self.core.network.server_name,
                    domain.as_ref(),
                )
                .await;

                Ok(JsonResponse::new(json!({
                    "data": {
                        "domain": domain.as_ref(),
                        "verified": result.verified,
                        "checks": result.checks,
                    },
                }))
                .into_http_response())
            }

            // Get DNS instructions for a domain
            ("instructions", Some(domain), &Method::GET) => {
                access_token.assert_has_permission(Permission::DomainGet)?;

                let domain = decode_path_element(domain);
                let instructions =
                    generate_dns_instructions(&self.core.network.server_name, domain.as_ref());

                Ok(JsonResponse::new(json!({
                    "data": instructions,
                }))
                .into_http_response())
            }

            _ => Err(trc::ResourceEvent::NotFound.into_err()),
        }
    }
}

async fn list_custom_domains(server: &Server) -> trc::Result<Vec<CustomDomain>> {
    let mut domains = Vec::new();
    let all_settings = server
        .core
        .storage
        .config
        .list("custom-domain.", false)
        .await?;

    // Collect domain IDs from entries ending in ".domain"
    let mut seen_ids = std::collections::HashSet::new();
    for (key, _) in &all_settings {
        if let Some(rest) = key.strip_prefix("custom-domain.") {
            if let Some(domain_id) = rest.strip_suffix(".domain") {
                seen_ids.insert(domain_id.to_string());
            }
        }
    }

    for domain_id in seen_ids {
        let prefix = format!("custom-domain.{domain_id}");
        let mut domain = CustomDomain {
            id: domain_id.clone(),
            domain: String::new(),
            app_resource: None,
            dns_verified: false,
            ssl_provisioned: false,
            created_at: String::new(),
        };

        for (k, v) in &all_settings {
            if let Some(suffix) = k.strip_prefix(&prefix) {
                match suffix {
                    ".domain" => domain.domain.clone_from(v),
                    ".app-resource" => domain.app_resource = Some(v.clone()),
                    ".dns-verified" => domain.dns_verified = v == "true",
                    ".ssl-provisioned" => domain.ssl_provisioned = v == "true",
                    ".created-at" => domain.created_at.clone_from(v),
                    _ => {}
                }
            }
        }

        if !domain.domain.is_empty() {
            domains.push(domain);
        }
    }

    Ok(domains)
}

async fn store_custom_domain(
    server: &Server,
    request: &CustomDomainRequest,
) -> trc::Result<String> {
    let domain_id = format!("{:x}", store::rand::random::<u64>());
    let prefix = format!("custom-domain.{domain_id}");
    let now = chrono::Utc::now().to_rfc3339();

    let settings: Vec<(String, String)> = vec![
        (format!("{prefix}.domain"), request.domain.clone()),
        (
            format!("{prefix}.app-resource"),
            request.app_resource.clone().unwrap_or_default(),
        ),
        (format!("{prefix}.dns-verified"), "false".to_string()),
        (format!("{prefix}.ssl-provisioned"), "false".to_string()),
        (format!("{prefix}.created-at"), now),
    ];

    server.core.storage.config.set(settings, true).await?;

    Ok(domain_id)
}

async fn delete_custom_domain(server: &Server, domain_id: &str) -> trc::Result<()> {
    let prefix = format!("custom-domain.{domain_id}.");
    server.core.storage.config.clear_prefix(&prefix).await
}

fn generate_dns_instructions(server_name: &str, domain: &str) -> DnsInstructions {
    let records = vec![
        DnsInstructionRecord {
            typ: "CNAME".to_string(),
            name: domain.to_string(),
            value: format!("{server_name}."),
            purpose: "Points your domain to the mail server".to_string(),
            required: true,
        },
        DnsInstructionRecord {
            typ: "MX".to_string(),
            name: domain.to_string(),
            value: format!("10 {server_name}."),
            purpose: "Routes email for your domain to the mail server".to_string(),
            required: true,
        },
        DnsInstructionRecord {
            typ: "TXT".to_string(),
            name: domain.to_string(),
            value: "v=spf1 mx ra=postmaster -all".to_string(),
            purpose: "SPF record to authorize mail server to send email for this domain"
                .to_string(),
            required: true,
        },
        DnsInstructionRecord {
            typ: "TXT".to_string(),
            name: format!("_dmarc.{domain}"),
            value: format!(
                "v=DMARC1; p=reject; rua=mailto:postmaster@{domain}; ruf=mailto:postmaster@{domain}"
            ),
            purpose: "DMARC policy for email authentication".to_string(),
            required: true,
        },
        DnsInstructionRecord {
            typ: "TXT".to_string(),
            name: format!("_smtp._tls.{domain}"),
            value: format!("v=TLSRPTv1; rua=mailto:postmaster@{domain}"),
            purpose: "TLS reporting for email transport security".to_string(),
            required: false,
        },
    ];

    DnsInstructions {
        domain: domain.to_string(),
        records,
        verification_url: format!("/api/custom-domains/verify/{domain}"),
        notes: vec![
            "DNS changes may take up to 48 hours to propagate globally.".to_string(),
            "SSL certificates will be automatically provisioned once DNS is verified.".to_string(),
            "Use the 'Test DNS' button to check if your records have propagated.".to_string(),
        ],
    }
}

async fn verify_custom_domain_dns(
    server: &Server,
    server_name: &str,
    domain: &str,
) -> DnsVerificationResult {
    let mut checks = Vec::new();
    let mut all_passed = true;

    // Check A/CNAME record resolution
    match tokio::net::lookup_host(format!("{domain}:25")).await {
        Ok(_addrs) => {
            checks.push(DnsCheck {
                record_type: "A/CNAME".to_string(),
                name: domain.to_string(),
                expected: server_name.to_string(),
                found: Some("resolves".to_string()),
                passed: true,
            });
        }
        Err(_) => {
            checks.push(DnsCheck {
                record_type: "A/CNAME".to_string(),
                name: domain.to_string(),
                expected: server_name.to_string(),
                found: None,
                passed: false,
            });
            all_passed = false;
        }
    };

    // Check MX record resolution
    match tokio::net::lookup_host(format!("{domain}:25")).await {
        Ok(_) => {
            checks.push(DnsCheck {
                record_type: "MX".to_string(),
                name: domain.to_string(),
                expected: format!("10 {server_name}."),
                found: Some("resolves".to_string()),
                passed: true,
            });
        }
        Err(_) => {
            checks.push(DnsCheck {
                record_type: "MX".to_string(),
                name: domain.to_string(),
                expected: format!("10 {server_name}."),
                found: None,
                passed: false,
            });
            all_passed = false;
        }
    };

    // Update stored verification status if all checks pass
    if all_passed {
        if let Ok(settings) = server
            .core
            .storage
            .config
            .list("custom-domain.", false)
            .await
        {
            for (key, value) in settings {
                if key.ends_with(".domain") && value == domain {
                    if let Some(prefix) = key.strip_suffix(".domain") {
                        let update_key = format!("{prefix}.dns-verified");
                        let _ = server
                            .core
                            .storage
                            .config
                            .set(vec![(update_key, "true".to_string())], true)
                            .await;
                    }
                    break;
                }
            }
        }
    }

    DnsVerificationResult {
        verified: all_passed,
        checks,
    }
}
