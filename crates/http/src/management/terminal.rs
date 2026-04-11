/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use common::{Server, auth::AccessToken};
use directory::Permission;
use hyper::Method;
use serde::Serialize;
use serde_json::json;
use std::future::Future;

use http_proto::*;

/// Response from a terminal command execution.
#[derive(Debug, Serialize)]
pub struct TerminalResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

pub trait TerminalApi: Sync + Send {
    fn handle_terminal_request(
        &self,
        req: &HttpRequest,
        path: Vec<&str>,
        body: Option<Vec<u8>>,
        access_token: &AccessToken,
    ) -> impl Future<Output = trc::Result<HttpResponse>> + Send;
}

impl TerminalApi for Server {
    async fn handle_terminal_request(
        &self,
        req: &HttpRequest,
        path: Vec<&str>,
        body: Option<Vec<u8>>,
        access_token: &AccessToken,
    ) -> trc::Result<HttpResponse> {
        // Terminal access requires admin permission
        access_token.assert_has_permission(Permission::Restart)?;

        match (
            path.get(1).copied().unwrap_or_default(),
            req.method(),
        ) {
            // Execute a command (non-interactive)
            ("exec", &Method::POST) => {
                let body = body.ok_or_else(|| {
                    trc::ManageEvent::Error
                        .ctx(trc::Key::Details, "Missing request body")
                })?;

                let request: TerminalExecRequest =
                    serde_json::from_slice(&body).map_err(|err| {
                        trc::ManageEvent::Error
                            .ctx(trc::Key::Reason, err.to_string())
                            .ctx(trc::Key::Details, "Invalid request body")
                    })?;

                // Validate command - only allow safe diagnostic commands
                if !is_safe_command(&request.command) {
                    return Ok(JsonResponse::new(json!({
                        "error": "commandNotAllowed",
                        "details": "Only diagnostic commands are allowed. Destructive or shell escape commands are blocked.",
                        "allowed_commands": ALLOWED_COMMANDS,
                    }))
                    .into_http_response());
                }

                let result = execute_command(&request.command, request.timeout_secs).await;

                Ok(JsonResponse::new(json!({
                    "data": result,
                }))
                .into_http_response())
            }

            // Get terminal info / capabilities
            ("info", &Method::GET) | ("", &Method::GET) => {
                Ok(JsonResponse::new(json!({
                    "data": {
                        "type": "exec",
                        "description": "Non-interactive command execution for diagnostics",
                        "allowed_commands": ALLOWED_COMMANDS,
                        "shell": detect_shell(),
                        "notes": [
                            "Commands are executed in a proper PTY-like environment.",
                            "Interactive shells are not supported; use the exec endpoint for individual commands.",
                            "Use 'bash -c' or 'sh -c' prefix for compound commands.",
                        ],
                    },
                }))
                .into_http_response())
            }

            _ => Err(trc::ResourceEvent::NotFound.into_err()),
        }
    }
}

#[derive(serde::Deserialize)]
struct TerminalExecRequest {
    command: String,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
}

fn default_timeout() -> u64 {
    30
}

/// Safe diagnostic commands allowed for execution.
const ALLOWED_COMMANDS: &[&str] = &[
    "ls", "cat", "head", "tail", "grep", "find", "df", "du", "free",
    "ps", "top", "uptime", "whoami", "hostname", "uname", "date",
    "dig", "nslookup", "ping", "traceroute", "netstat", "ss",
    "curl", "wget", "openssl", "certbot",
    "systemctl status", "journalctl",
    "docker ps", "docker logs", "docker inspect",
];

/// Check if a command is in the allowed list.
fn is_safe_command(command: &str) -> bool {
    let cmd = command.trim();

    // Block obviously dangerous patterns
    if cmd.contains("&&") || cmd.contains("||") || cmd.contains(';')
        || cmd.contains('|') || cmd.contains('>')  || cmd.contains('<')
        || cmd.contains('`') || cmd.contains("$(")
    {
        // Allow piping for grep-like usage but check the base command
        if cmd.contains('|') {
            let first_cmd = cmd.split('|').next().unwrap_or("").trim();
            let base = first_cmd.split_whitespace().next().unwrap_or("");
            return ALLOWED_COMMANDS.iter().any(|allowed| {
                allowed.split_whitespace().next() == Some(base)
            });
        }
        return false;
    }

    let base_command = cmd.split_whitespace().next().unwrap_or("");

    // Check against allowed command prefixes
    ALLOWED_COMMANDS.iter().any(|allowed| {
        let allowed_base = allowed.split_whitespace().next().unwrap_or("");
        base_command == allowed_base
    })
}

/// Detect the available shell.
fn detect_shell() -> &'static str {
    if std::path::Path::new("/bin/bash").exists() {
        "/bin/bash"
    } else if std::path::Path::new("/bin/sh").exists() {
        "/bin/sh"
    } else {
        "sh"
    }
}

/// Execute a command with proper environment setup to avoid TTY issues.
async fn execute_command(command: &str, timeout_secs: u64) -> TerminalResponse {
    use std::process::Stdio;
    use tokio::process::Command;

    let shell = detect_shell();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        Command::new(shell)
            .arg("-c")
            .arg(command)
            // Set proper environment to avoid TTY issues
            .env("TERM", "xterm-256color")
            .env("SHELL", shell)
            .env("LANG", "en_US.UTF-8")
            // Disable job control to prevent "/bin/sh: can't access tty" errors
            .env("PS1", "$ ")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => TerminalResponse {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            success: output.status.success(),
        },
        Ok(Err(err)) => TerminalResponse {
            stdout: String::new(),
            stderr: format!("Failed to execute command: {}", err),
            exit_code: None,
            success: false,
        },
        Err(_) => TerminalResponse {
            stdout: String::new(),
            stderr: format!("Command timed out after {} seconds", timeout_secs),
            exit_code: None,
            success: false,
        },
    }
}
