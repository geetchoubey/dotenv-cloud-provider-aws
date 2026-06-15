//! AWS client construction and resolution for Secrets Manager (spec §8.1) and
//! SSM Parameter Store (spec §8.2).

use std::time::Duration;

use aws_config::{BehaviorVersion, Region};
use aws_smithy_types::timeout::TimeoutConfig;
use base64::Engine;

use dotenv_cloud_protocol::{ErrorClass, Reference};
use dotenv_cloud_provider_sdk::{parse_bool, select_field, ProviderError, ResolvedSecret};

use crate::config::AwsConfig;
use crate::error::map_sdk_error;
use crate::payload::{secret_manager_id, ssm_parameter_name};

/// Long-lived AWS clients, built once from the first request's config.
pub struct AwsClients {
    sm: aws_sdk_secretsmanager::Client,
    ssm: aws_sdk_ssm::Client,
    cfg: AwsConfig,
}

impl AwsClients {
    /// Load the AWS default credential/region chain, applying optional region,
    /// profile, and timeout overrides.
    pub async fn load(cfg: AwsConfig) -> AwsClients {
        let mut loader = aws_config::defaults(BehaviorVersion::latest());
        if let Some(region) = &cfg.region {
            loader = loader.region(Region::new(region.clone()));
        }
        if let Some(profile) = &cfg.profile {
            loader = loader.profile_name(profile.clone());
        }
        if let Some(ms) = cfg.timeout_ms {
            let timeout = TimeoutConfig::builder()
                .operation_timeout(Duration::from_millis(ms))
                .build();
            loader = loader.timeout_config(timeout);
        }
        let shared = loader.load().await;
        AwsClients {
            sm: aws_sdk_secretsmanager::Client::new(&shared),
            ssm: aws_sdk_ssm::Client::new(&shared),
            cfg,
        }
    }

    /// Resolve an `aws-sm://` reference via `GetSecretValue`.
    pub async fn resolve_secrets_manager(
        &self,
        reference: &Reference,
    ) -> Result<ResolvedSecret, ProviderError> {
        let id = secret_manager_id(reference)?;
        let mut req = self.sm.get_secret_value().secret_id(id);
        if let Some(v) = reference.query.get("version_id") {
            req = req.version_id(v);
        }
        if let Some(v) = reference.query.get("version_stage") {
            req = req.version_stage(v);
        }

        let resp = req.send().await.map_err(map_sdk_error)?;
        let version = resp.version_id().map(String::from);

        if let Some(s) = resp.secret_string() {
            let value = select_field(s, reference.fragment.as_deref())?;
            return Ok(ResolvedSecret { value, version });
        }

        if let Some(blob) = resp.secret_binary() {
            // Binary secrets are only emitted when explicitly base64-encoded.
            let wants_base64 = reference
                .query
                .get("binary")
                .map(|v| v.eq_ignore_ascii_case("base64"))
                .unwrap_or(false);
            if !wants_base64 {
                return Err(ProviderError::invalid_payload(
                    "secret is binary; add `?binary=base64` to encode it",
                ));
            }
            let encoded = base64::engine::general_purpose::STANDARD.encode(blob.as_ref());
            return Ok(ResolvedSecret {
                value: encoded,
                version,
            });
        }

        Err(ProviderError::invalid_payload("secret has no value"))
    }

    /// Resolve an `aws-ssm://` reference via `GetParameter`.
    pub async fn resolve_ssm(
        &self,
        reference: &Reference,
    ) -> Result<ResolvedSecret, ProviderError> {
        let name = ssm_parameter_name(reference)?;

        // Precedence for with_decryption: URI query > provider config > default true.
        let with_decryption = reference
            .query
            .get("with_decryption")
            .and_then(|v| parse_bool(v))
            .or(self.cfg.ssm_with_decryption)
            .unwrap_or(true);

        let resp = self
            .ssm
            .get_parameter()
            .name(name)
            .with_decryption(with_decryption)
            .send()
            .await
            .map_err(map_sdk_error)?;

        let param = resp
            .parameter()
            .ok_or_else(|| ProviderError::new(ErrorClass::NotFound, "parameter not found"))?;
        let raw = param
            .value()
            .ok_or_else(|| ProviderError::invalid_payload("parameter has no value"))?;
        let version = Some(param.version().to_string());

        let value = select_field(raw, reference.fragment.as_deref())?;
        Ok(ResolvedSecret { value, version })
    }

    /// Dispatch by scheme.
    pub async fn resolve(&self, reference: &Reference) -> Result<ResolvedSecret, ProviderError> {
        match reference.scheme.as_str() {
            "aws-sm" => self.resolve_secrets_manager(reference).await,
            "aws-ssm" => self.resolve_ssm(reference).await,
            other => Err(ProviderError::invalid_reference(format!(
                "unsupported scheme `{other}`"
            ))),
        }
    }
}
