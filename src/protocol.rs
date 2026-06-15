//! Wire protocol (v1): newline-delimited JSON over stdin/stdout.
//!
//! Mirrors `docs/PROVIDER_PROTOCOL.md` in the dotenv-cloud core repo.
//!
//! Some request fields are deserialized for protocol fidelity even though this
//! plugin does not act on all of them.
#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: &str = "1";

/// A request from the core. Discriminated by the `type` field.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    #[serde(rename = "handshake")]
    Handshake {
        #[serde(default)]
        protocol_version: Option<String>,
        #[serde(default)]
        dotenv_cloud_version: Option<String>,
    },
    #[serde(rename = "resolve")]
    Resolve(Box<ResolveRequest>),
}

#[derive(Debug, Deserialize)]
pub struct ResolveRequest {
    pub request_id: String,
    #[serde(default)]
    pub profile: Option<String>,
    pub reference: Reference,
    #[serde(default)]
    pub provider_config: serde_json::Value,
}

/// A parsed secret reference (see core `SecretReference`).
#[derive(Debug, Clone, Deserialize)]
pub struct Reference {
    pub original: String,
    pub scheme: String,
    #[serde(default)]
    pub authority: Option<String>,
    pub path: String,
    #[serde(default)]
    pub fragment: Option<String>,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum Response {
    #[serde(rename = "handshake_result")]
    HandshakeResult {
        protocol_version: &'static str,
        plugin: PluginInfo,
    },
    #[serde(rename = "resolve_result")]
    ResolveResult {
        request_id: String,
        value: String,
        metadata: Metadata,
    },
    #[serde(rename = "error")]
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        class: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reference: Option<String>,
    },
}

#[derive(Debug, Serialize)]
pub struct PluginInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub schemes: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct Metadata {
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl Response {
    pub fn handshake() -> Response {
        Response::HandshakeResult {
            protocol_version: PROTOCOL_VERSION,
            plugin: PluginInfo {
                name: env!("CARGO_PKG_NAME"),
                version: env!("CARGO_PKG_VERSION"),
                schemes: vec!["aws-sm", "aws-ssm"],
            },
        }
    }
}
