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

/// A node in the service topology graph.
#[derive(Debug, Serialize, Clone)]
pub struct TopologyNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    pub status: String,
    pub ports: Vec<PortInfo>,
    pub exposed: bool,
}

/// Port information with direction.
#[derive(Debug, Serialize, Clone)]
pub struct PortInfo {
    pub port: u16,
    pub protocol: String,
    pub direction: String,
    pub tls: bool,
}

/// An edge/connection in the topology graph.
#[derive(Debug, Serialize, Clone)]
pub struct TopologyEdge {
    pub from: String,
    pub to: String,
    pub port: u16,
    pub protocol: String,
    pub label: String,
}

/// Full topology response.
#[derive(Debug, Serialize)]
pub struct TopologyResponse {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
}

pub trait TopologyApi: Sync + Send {
    fn handle_topology_request(
        &self,
        req: &HttpRequest,
        path: Vec<&str>,
        access_token: &AccessToken,
    ) -> impl Future<Output = trc::Result<HttpResponse>> + Send;
}

impl TopologyApi for Server {
    async fn handle_topology_request(
        &self,
        req: &HttpRequest,
        path: Vec<&str>,
        access_token: &AccessToken,
    ) -> trc::Result<HttpResponse> {
        match (
            path.get(1).copied().unwrap_or_default(),
            req.method(),
        ) {
            // Get full service topology
            ("services", &Method::GET) | ("", &Method::GET) => {
                access_token.assert_has_permission(Permission::LogsView)?;

                let topology = build_topology(self).await?;

                Ok(JsonResponse::new(json!({
                    "data": topology,
                }))
                .into_http_response())
            }

            _ => Err(trc::ResourceEvent::NotFound.into_err()),
        }
    }
}

async fn build_topology(server: &Server) -> trc::Result<TopologyResponse> {
    let server_name = &server.core.network.server_name;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Internet gateway node
    nodes.push(TopologyNode {
        id: "internet".to_string(),
        node_type: "internet".to_string(),
        label: "Internet".to_string(),
        status: "active".to_string(),
        ports: Vec::new(),
        exposed: true,
    });

    // Mail server node with all active services
    let mut mail_ports = Vec::new();
    let services = server
        .core
        .storage
        .config
        .get_services()
        .await
        .unwrap_or_default();

    for (protocol, port, is_tls) in &services {
        let direction = match protocol.as_str() {
            "smtp" if *port == 25 => "inbound",
            "smtp" => "inbound/outbound",
            "imap" | "pop3" | "http" => "inbound",
            _ => "inbound",
        };

        mail_ports.push(PortInfo {
            port: *port,
            protocol: protocol.clone(),
            direction: direction.to_string(),
            tls: *is_tls,
        });
    }

    nodes.push(TopologyNode {
        id: "mail-server".to_string(),
        node_type: "service".to_string(),
        label: format!("Mail Server ({})", server_name),
        status: "running".to_string(),
        ports: mail_ports.clone(),
        exposed: true,
    });

    // Add edges from internet to exposed ports
    for port_info in &mail_ports {
        edges.push(TopologyEdge {
            from: "internet".to_string(),
            to: "mail-server".to_string(),
            port: port_info.port,
            protocol: port_info.protocol.clone(),
            label: format!(
                "{} :{}{}",
                port_info.protocol.to_uppercase(),
                port_info.port,
                if port_info.tls { " (TLS)" } else { "" }
            ),
        });
    }

    // Database/store node
    nodes.push(TopologyNode {
        id: "datastore".to_string(),
        node_type: "database".to_string(),
        label: "Data Store".to_string(),
        status: "running".to_string(),
        ports: Vec::new(),
        exposed: false,
    });

    edges.push(TopologyEdge {
        from: "mail-server".to_string(),
        to: "datastore".to_string(),
        port: 0,
        protocol: "internal".to_string(),
        label: "Data Access".to_string(),
    });

    // Blob store node
    nodes.push(TopologyNode {
        id: "blobstore".to_string(),
        node_type: "storage".to_string(),
        label: "Blob Store".to_string(),
        status: "running".to_string(),
        ports: Vec::new(),
        exposed: false,
    });

    edges.push(TopologyEdge {
        from: "mail-server".to_string(),
        to: "blobstore".to_string(),
        port: 0,
        protocol: "internal".to_string(),
        label: "Blob Storage".to_string(),
    });

    Ok(TopologyResponse { nodes, edges })
}
