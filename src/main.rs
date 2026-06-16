//! dotenv-cloud AWS provider plugin.
//!
//! Resolves `aws-secrets://` and `aws-ssm://` references via the AWS SDK. The
//! `dotenv-cloud-provider-sdk` crate owns the stdin/stdout protocol runtime;
//! this binary only implements [`Provider`].

mod aws;
mod config;
mod error;
mod payload;

use dotenv_cloud_provider_sdk::protocol::{PluginInfo, Reference};
use dotenv_cloud_provider_sdk::{
    serve, ConfigField, FieldKind, Provider, ProviderError, ResolvedSecret,
};
use tokio::sync::OnceCell;

use aws::AwsClients;
use config::AwsConfig;

/// The AWS provider. Clients are built lazily on the first resolve so a
/// handshake-only invocation never touches the credential chain.
struct AwsProvider {
    clients: OnceCell<AwsClients>,
}

impl AwsProvider {
    fn new() -> Self {
        AwsProvider {
            clients: OnceCell::new(),
        }
    }
}

impl Provider for AwsProvider {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            schemes: vec!["aws-secrets".to_string(), "aws-ssm".to_string()],
        }
    }

    fn config_schema(&self) -> Vec<ConfigField> {
        vec![
            ConfigField {
                key: "region".to_string(),
                label: "AWS region".to_string(),
                kind: FieldKind::String,
                default: None,
                required: false,
                secret: false,
            },
            ConfigField {
                key: "profile".to_string(),
                label: "AWS named credentials profile".to_string(),
                kind: FieldKind::String,
                default: None,
                required: false,
                secret: false,
            },
            ConfigField {
                key: "timeout_ms".to_string(),
                label: "Per-request timeout in milliseconds".to_string(),
                kind: FieldKind::Integer,
                default: None,
                required: false,
                secret: false,
            },
            ConfigField {
                key: "ssm.with_decryption".to_string(),
                label: "Decrypt SSM SecureString parameters".to_string(),
                kind: FieldKind::Bool,
                default: Some("true".to_string()),
                required: false,
                secret: false,
            },
        ]
    }

    async fn resolve(
        &self,
        reference: &Reference,
        provider_config: &serde_json::Value,
    ) -> Result<ResolvedSecret, ProviderError> {
        let clients = self
            .clients
            .get_or_init(|| AwsClients::load(AwsConfig::from_json(provider_config)))
            .await;
        clients.resolve(reference).await
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    serve(AwsProvider::new()).await
}
