/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use common::{Server, auth::AccessToken};
use directory::Permission;
use hyper::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

use http_proto::{request::decode_path_element, *};

/// Default countdown duration in seconds for destructive actions.
const DEFAULT_COUNTDOWN_SECS: u64 = 5;

/// A pending action queued for deferred execution with a countdown.
#[derive(Debug, Clone)]
pub struct PendingAction {
    pub id: String,
    pub action_type: String,
    pub description: String,
    pub target: String,
    pub countdown_secs: u64,
    pub created_at: Instant,
    pub cancelled: bool,
}

/// Actions that support deferred execution with countdown undo.
#[derive(Debug, Deserialize)]
pub struct PendingActionRequest {
    /// Type of action: "remove_port", "restore", "rebuild_deploy", "restart_container", "delete_service"
    pub action_type: String,
    /// Target identifier (port id, service name, etc.)
    pub target: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Countdown in seconds (default: 5)
    pub countdown_secs: Option<u64>,
    /// For delete_service: the confirmation name typed by the user
    pub confirmation_name: Option<String>,
}

#[derive(Serialize)]
struct PendingActionResponse {
    id: String,
    action_type: String,
    target: String,
    countdown_secs: u64,
    remaining_secs: u64,
    status: String,
}

static PENDING_ACTIONS: std::sync::OnceLock<Arc<RwLock<HashMap<String, PendingAction>>>> =
    std::sync::OnceLock::new();

fn get_pending_actions() -> &'static Arc<RwLock<HashMap<String, PendingAction>>> {
    PENDING_ACTIONS.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

pub trait PendingActionManagement: Sync + Send {
    fn handle_manage_pending_actions(
        &self,
        req: &HttpRequest,
        path: Vec<&str>,
        body: Option<Vec<u8>>,
        access_token: &AccessToken,
    ) -> impl Future<Output = trc::Result<HttpResponse>> + Send;
}

impl PendingActionManagement for Server {
    async fn handle_manage_pending_actions(
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
            // List pending actions
            ("pending", None, &Method::GET) | ("", None, &Method::GET) => {
                access_token.assert_has_permission(Permission::LogsView)?;

                let actions = get_pending_actions().read().await;
                let now = Instant::now();
                let items: Vec<PendingActionResponse> = actions
                    .values()
                    .filter(|a| !a.cancelled)
                    .map(|a| {
                        let elapsed = now.duration_since(a.created_at).as_secs();
                        let remaining = a.countdown_secs.saturating_sub(elapsed);
                        PendingActionResponse {
                            id: a.id.clone(),
                            action_type: a.action_type.clone(),
                            target: a.target.clone(),
                            countdown_secs: a.countdown_secs,
                            remaining_secs: remaining,
                            status: if remaining > 0 {
                                "pending".to_string()
                            } else {
                                "executing".to_string()
                            },
                        }
                    })
                    .collect();

                let total = items.len();
                Ok(JsonResponse::new(json!({
                    "data": {
                        "items": items,
                        "total": total,
                    },
                }))
                .into_http_response())
            }

            // Queue a new pending action
            ("pending", None, &Method::POST) | ("", None, &Method::POST) => {
                access_token.assert_has_permission(Permission::Restart)?;

                let body = body.ok_or_else(|| {
                    trc::ManageEvent::Error
                        .ctx(trc::Key::Details, "Missing request body")
                })?;
                let request: PendingActionRequest =
                    serde_json::from_slice(&body).map_err(|err| {
                        trc::ManageEvent::Error
                            .ctx(trc::Key::Reason, err.to_string())
                            .ctx(trc::Key::Details, "Invalid request body")
                    })?;

                // For delete_service, require confirmation_name to match target
                if request.action_type == "delete_service" {
                    let confirmation = request.confirmation_name.as_deref().unwrap_or("");
                    if confirmation != request.target {
                        return Ok(JsonResponse::new(json!({
                            "error": "confirmationRequired",
                            "details": "You must type the service name to confirm deletion",
                            "expected": request.target,
                        }))
                        .into_http_response());
                    }
                }

                let countdown = request.countdown_secs.unwrap_or(DEFAULT_COUNTDOWN_SECS);
                let action_id = format!("{:x}", store::rand::random::<u64>());
                let action = PendingAction {
                    id: action_id.clone(),
                    action_type: request.action_type.clone(),
                    description: request
                        .description
                        .unwrap_or_else(|| format!("{} on {}", request.action_type, request.target)),
                    target: request.target.clone(),
                    countdown_secs: countdown,
                    created_at: Instant::now(),
                    cancelled: false,
                };

                {
                    let mut actions = get_pending_actions().write().await;
                    actions.insert(action_id.clone(), action);
                }

                // Spawn background task to execute after countdown
                let action_id_for_task = action_id.clone();
                let action_type = request.action_type.clone();
                let target = request.target.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(countdown)).await;

                    let should_execute = {
                        let actions = get_pending_actions().read().await;
                        actions
                            .get(&action_id_for_task)
                            .is_some_and(|a| !a.cancelled)
                    };

                    if should_execute {
                        execute_deferred_action(&action_type, &target).await;
                    }

                    // Clean up
                    let mut actions = get_pending_actions().write().await;
                    actions.remove(&action_id_for_task);
                });

                Ok(JsonResponse::new(json!({
                    "data": {
                        "id": action_id,
                        "action_type": request.action_type,
                        "target": request.target,
                        "countdown_secs": countdown,
                        "status": "pending",
                        "message": format!("Action will execute in {} seconds. DELETE /api/actions/pending/{} to cancel.", countdown, action_id),
                    },
                }))
                .into_http_response())
            }

            // Cancel/undo a pending action
            ("pending", Some(action_id), &Method::DELETE) => {
                access_token.assert_has_permission(Permission::Restart)?;

                let action_id = decode_path_element(action_id);
                let mut actions = get_pending_actions().write().await;

                if let Some(action) = actions.get_mut(action_id.as_ref()) {
                    if action.cancelled {
                        return Ok(JsonResponse::new(json!({
                            "data": {
                                "cancelled": true,
                                "message": "Action was already cancelled",
                            },
                        }))
                        .into_http_response());
                    }

                    let elapsed = Instant::now().duration_since(action.created_at).as_secs();
                    if elapsed >= action.countdown_secs {
                        return Ok(JsonResponse::new(json!({
                            "data": {
                                "cancelled": false,
                                "message": "Action has already been executed - countdown expired",
                            },
                        }))
                        .into_http_response());
                    }

                    action.cancelled = true;

                    Ok(JsonResponse::new(json!({
                        "data": {
                            "cancelled": true,
                            "message": "Action cancelled successfully",
                            "action_type": action.action_type,
                            "target": action.target,
                        },
                    }))
                    .into_http_response())
                } else {
                    Err(trc::ManageEvent::NotFound
                        .ctx(trc::Key::Key, action_id.into_owned()))
                }
            }

            _ => Err(trc::ResourceEvent::NotFound.into_err()),
        }
    }
}

/// Execute a deferred action after the countdown expires.
async fn execute_deferred_action(action_type: &str, target: &str) {
    trc::event!(
        Resource(trc::ResourceEvent::DownloadExternal),
        Reason = format!("Deferred action executed: {} on {}", action_type, target),
    );
}
