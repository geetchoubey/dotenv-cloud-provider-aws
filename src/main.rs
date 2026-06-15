//! dotenv-cloud AWS provider plugin.
//!
//! Resolves `aws-sm://` and `aws-ssm://` references via the AWS SDK. The
//! `dotenv-cloud-provider-sdk` crate owns the stdin/stdout protocol runtime;
//! this binary only implements [`Provider`].

mod aws;
mod config;
mod error;
mod payload;

use dotenv_cloud_provider_sdk::protocol::{PluginInfo, Reference};
use dotenv_cloud_provider_sdk::{serve, Provider, ProviderError, ResolvedSecret};
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
            schemes: vec!["aws-sm".to_string(), "aws-ssm".to_string()],
        }
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
