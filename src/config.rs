//! Provider configuration extracted from the `provider_config` object the core
//! passes in each resolve request (spec §8.1, §8.2).

/// AWS provider configuration. All fields are optional; the AWS default
/// credential/region chain fills in anything not specified here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AwsConfig {
    pub region: Option<String>,
    pub profile: Option<String>,
    pub timeout_ms: Option<u64>,
    /// Default `with_decryption` for SSM (defaults to true when unset).
    pub ssm_with_decryption: Option<bool>,
}

impl AwsConfig {
    /// Parse from the JSON `provider_config` table.
    pub fn from_json(value: &serde_json::Value) -> AwsConfig {
        let s = |k: &str| value.get(k).and_then(|v| v.as_str()).map(String::from);
        let u = |k: &str| value.get(k).and_then(|v| v.as_u64());

        let ssm_with_decryption = value
            .get("ssm")
            .and_then(|s| s.get("with_decryption"))
            .and_then(|v| v.as_bool());

        AwsConfig {
            region: s("region"),
            profile: s("profile"),
            timeout_ms: u("timeout_ms"),
            ssm_with_decryption,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_config() {
        let v = serde_json::json!({
            "region": "us-east-1",
            "profile": "dev",
            "timeout_ms": 2000,
            "ssm": { "with_decryption": false }
        });
        let c = AwsConfig::from_json(&v);
        assert_eq!(c.region.as_deref(), Some("us-east-1"));
        assert_eq!(c.profile.as_deref(), Some("dev"));
        assert_eq!(c.timeout_ms, Some(2000));
        assert_eq!(c.ssm_with_decryption, Some(false));
    }

    #[test]
    fn empty_config_is_all_none() {
        assert_eq!(
            AwsConfig::from_json(&serde_json::json!({})),
            AwsConfig::default()
        );
    }
}
