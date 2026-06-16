//! AWS-specific helpers: turning a [`Reference`] into a Secrets Manager secret
//! id or an SSM parameter name (spec §8.1, §8.2). JSON `#fragment` selection and
//! bool parsing live in `dotenv-cloud-provider-sdk`.

use dotenv_cloud_protocol::Reference;
use dotenv_cloud_provider_sdk::ProviderError;

/// Authority joined with path, e.g. `aws-secrets://prod/db/password` -> `prod/db/password`.
fn joined(reference: &Reference) -> String {
    match &reference.authority {
        Some(a) => format!("{a}{}", reference.path),
        None => reference.path.clone(),
    }
}

/// Secrets Manager secret id (no leading slash).
pub fn secret_manager_id(reference: &Reference) -> Result<String, ProviderError> {
    let id = joined(reference);
    let id = id.trim_start_matches('/').to_string();
    if id.is_empty() {
        return Err(ProviderError::invalid_reference(
            "empty Secrets Manager secret id",
        ));
    }
    Ok(id)
}

/// SSM parameter name (absolute, leading slash).
pub fn ssm_parameter_name(reference: &Reference) -> Result<String, ProviderError> {
    let name = joined(reference);
    if name.is_empty() || name == "/" {
        return Err(ProviderError::invalid_reference("empty SSM parameter name"));
    }
    Ok(if name.starts_with('/') {
        name
    } else {
        format!("/{name}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn reference(authority: Option<&str>, path: &str) -> Reference {
        Reference {
            original: "aws-secrets://...".into(),
            scheme: "aws-secrets".into(),
            authority: authority.map(String::from),
            path: path.to_string(),
            fragment: None,
            query: BTreeMap::new(),
        }
    }

    #[test]
    fn sm_id_from_authority_and_path() {
        assert_eq!(
            secret_manager_id(&reference(Some("prod"), "/db/password")).unwrap(),
            "prod/db/password"
        );
    }

    #[test]
    fn sm_id_from_path_only() {
        assert_eq!(
            secret_manager_id(&reference(None, "/foo/bar")).unwrap(),
            "foo/bar"
        );
    }

    #[test]
    fn ssm_name_triple_slash_is_absolute() {
        assert_eq!(
            ssm_parameter_name(&reference(None, "/prod/app/api_token")).unwrap(),
            "/prod/app/api_token"
        );
    }

    #[test]
    fn ssm_name_double_slash_gets_leading_slash() {
        assert_eq!(
            ssm_parameter_name(&reference(Some("prod"), "/app")).unwrap(),
            "/prod/app"
        );
    }
}
