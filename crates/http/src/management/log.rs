/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use std::{
    fs::{self, File},
    io,
    path::Path,
};

use chrono::DateTime;
use common::{Server, auth::AccessToken};
use directory::{Permission, backend::internal::manage};
use rev_lines::RevLines;
use serde::Serialize;
use serde_json::json;
use std::future::Future;
use tokio::sync::oneshot;
use utils::url_params::UrlParams;

use http_proto::*;

#[derive(Serialize)]
struct LogEntry {
    timestamp: String,
    level: String,
    #[serde(rename = "levelColor")]
    level_color: &'static str,
    event: String,
    event_id: String,
    details: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    service: Option<String>,
}

/// Filter parameters for log queries.
struct LogFilter {
    text: String,
    level: Option<String>,
    service: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
}

pub trait LogManagement: Sync + Send {
    fn handle_view_logs(
        &self,
        req: &HttpRequest,
        access_token: &AccessToken,
    ) -> impl Future<Output = trc::Result<HttpResponse>> + Send;
}

impl LogManagement for Server {
    async fn handle_view_logs(
        &self,
        req: &HttpRequest,
        access_token: &AccessToken,
    ) -> trc::Result<HttpResponse> {
        // Validate the access token
        access_token.assert_has_permission(Permission::LogsView)?;

        let path = self
            .core
            .metrics
            .log_path
            .clone()
            .ok_or_else(|| manage::unsupported("Tracer log path not configured"))?;

        let params = UrlParams::new(req.uri().query());
        let filter_text = params.get("filter").unwrap_or_default().to_string();
        let level = params.get("level").map(|s| s.to_uppercase());
        let service = params.get("service").map(|s| s.to_string());
        let from = params
            .get("from")
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp());
        let to = params
            .get("to")
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp());
        let page: usize = params.parse("page").unwrap_or(0);
        let limit: usize = params.parse("limit").unwrap_or(100);
        let offset = page.saturating_sub(1) * limit;

        let log_filter = LogFilter {
            text: filter_text,
            level,
            service,
            from,
            to,
        };

        // Clone the path for service discovery before moving it
        let services_path = self.core.metrics.log_path.clone();

        let (tx, rx) = oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let _ = tx.send(read_log_files(path, &log_filter, offset, limit));
        });

        let (total, items) = rx
            .await
            .map_err(|err| {
                trc::EventType::Server(trc::ServerEvent::ThreadError)
                    .reason(err)
                    .caused_by(trc::location!())
            })?
            .map_err(|err| {
                trc::ManageEvent::Error
                    .reason(err)
                    .details("Failed to read log files")
                    .caused_by(trc::location!())
            })?;

        // List available log services/components for the filter dropdown
        let services = discover_log_services(&services_path);

        Ok(JsonResponse::new(json!({
            "data": {
                "items": items,
                "total": total,
                "services": services,
            },
        }))
        .into_http_response())
    }
}

/// Discover unique service/component names from recent log entries.
fn discover_log_services(path: &Option<String>) -> Vec<String> {
    let Some(path) = path else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut services = std::collections::BTreeSet::new();
    let mut logs: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    logs.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    // Only scan the most recent log file for service names
    if let Some(log) = logs.first() {
        if let Ok(file) = File::open(log.path()) {
            let rev_lines = RevLines::new(file);
            let mut count = 0;
            for line in rev_lines {
                if let Ok(line) = line {
                    if let Some(entry) = LogEntry::from_line(&line) {
                        if let Some(svc) = &entry.service {
                            services.insert(svc.clone());
                        }
                        // Also use the event category as a service
                        if let Some(category) = entry.event.split('.').next() {
                            services.insert(category.to_string());
                        }
                    }
                    count += 1;
                    if count >= 500 {
                        break;
                    }
                }
            }
        }
    }

    services.into_iter().collect()
}

fn read_log_files(
    path: impl AsRef<Path>,
    filter: &LogFilter,
    mut offset: usize,
    limit: usize,
) -> io::Result<(usize, Vec<LogEntry>)> {
    let mut logs = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    let mut total = 0;

    // Sort the entries by file name in reverse order (newest first).
    logs.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    let mut entries = Vec::with_capacity(limit);
    let mut logs = logs.into_iter();
    while let Some(log) = logs.next() {
        if log.file_type()?.is_file() {
            let mut rev_lines = RevLines::new(File::open(log.path())?);

            while let Some(line) = rev_lines.next() {
                let line = line.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                // Pre-filter by text before parsing
                if !filter.text.is_empty() && !line.contains(&filter.text) {
                    continue;
                }

                if let Some(entry) = LogEntry::from_line(&line) {
                    // Apply level filter
                    if let Some(ref level) = filter.level {
                        if entry.level != *level {
                            continue;
                        }
                    }

                    // Apply service filter
                    if let Some(ref service) = filter.service {
                        let matches_service = entry
                            .service
                            .as_deref()
                            .is_some_and(|s| s.eq_ignore_ascii_case(service))
                            || entry
                                .event
                                .split('.')
                                .next()
                                .is_some_and(|cat| cat.eq_ignore_ascii_case(service));
                        if !matches_service {
                            continue;
                        }
                    }

                    // Apply date/time range filter
                    if let Some(from_ts) = filter.from {
                        if let Ok(ts) = DateTime::parse_from_rfc3339(&entry.timestamp) {
                            if ts.timestamp() < from_ts {
                                // Since we read in reverse chronological order,
                                // once we pass the 'from' boundary we can stop
                                break;
                            }
                        }
                    }
                    if let Some(to_ts) = filter.to {
                        if let Ok(ts) = DateTime::parse_from_rfc3339(&entry.timestamp) {
                            if ts.timestamp() > to_ts {
                                continue;
                            }
                        }
                    }

                    total += 1;
                    if offset == 0 {
                        entries.push(entry);
                        if entries.len() == limit {
                            if rev_lines.next().is_some() || logs.next().is_some() {
                                total += limit;
                            }
                            return Ok((total, entries));
                        }
                    } else {
                        offset -= 1;
                    }
                }
            }
        }
    }

    Ok((total, entries))
}

impl LogEntry {
    fn from_line(line: &str) -> Option<Self> {
        let (timestamp, rest) = line.split_once(' ')?;
        let timestamp = DateTime::parse_from_rfc3339(timestamp).ok()?;
        let (level, rest) = rest.trim().split_once(' ')?;
        let (event, rest) = rest.trim().split_once(" (")?;
        let (event_id, details) = rest.split_once(")")?;

        // Extract service/component from event name (e.g., "smtp.delivery" -> "smtp")
        let service = event.split('.').next().map(|s| s.to_string());

        let level_upper = level.to_uppercase();
        let level_color = match level_upper.as_str() {
            "ERROR" => "#ff4444",
            "WARN" => "#ffaa00",
            "INFO" => "#44bb44",
            "DEBUG" => "#4488ff",
            "TRACE" => "#888888",
            _ => "#cccccc",
        };

        Some(Self {
            timestamp: timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            level: level_upper,
            level_color,
            event: event.to_string(),
            event_id: event_id.to_string(),
            details: details.trim().to_string(),
            service,
        })
    }
}
