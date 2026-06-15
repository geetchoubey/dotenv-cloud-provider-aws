//! Pure helpers for turning references into AWS identifiers and for selecting
//! fields from structured secret payloads (spec §6.3, §6.4). No AWS calls here,
//! so these are unit-testable in isolation.

use crate::error::{ErrorClass, ProviderError};
use crate::protocol::Reference;

/// Authority joined with path, e.g. `aws-sm://prod/db/password` -> `prod/db/password`.
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

/// Apply a fragment field selector to a (possibly JSON) secret string
/// (spec §6.4).
///
/// * No fragment -> the raw string is returned unchanged.
/// * Fragment + JSON object containing the key -> that field, stringified.
/// * Fragment but value is not a JSON object, or key missing -> `InvalidSecretPayload`.
pub fn select_field(raw: &str, fragment: Option<&str>) -> Result<String, ProviderError> {
    let Some(field) = fragment else {
        return Ok(raw.to_string());
    };

    let parsed: serde_json::Value = serde_json::from_str(raw).map_err(|_| {
        ProviderError::invalid_payload(format!(
            "secret is not a JSON object but fragment `#{field}` was requested"
        ))
    })?;

    let obj = parsed.as_object().ok_or_else(|| {
        ProviderError::invalid_payload(format!(
            "secret is not a JSON object but fragment `#{field}` was requested"
        ))
    })?;

    let value = obj.get(field).ok_or_else(|| {
        ProviderError::new(
            ErrorClass::InvalidSecretPayload,
            format!("field `{field}` not present in secret payload"),
        )
    })?;

    Ok(stringify(value))
}

/// Convert a JSON value to the string that becomes the env var value: strings
/// pass through unquoted; everything else is serialized as JSON.
fn stringify(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Read a boolean query/config flag like `with_decryption=true`.
pub fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn reference(
        scheme: &str,
        authority: Option<&str>,
        path: &str,
        fragment: Option<&str>,
    ) -> Reference {
        Reference {
            original: format!("{scheme}://..."),
            scheme: scheme.to_string(),
            authority: authority.map(String::from),
            path: path.to_string(),
            fragment: fragment.map(String::from),
            query: BTreeMap::new(),
        }
    }

    #[test]
    fn sm_id_from_authority_and_path() {
        let r = reference("aws-sm", Some("prod"), "/db/password", None);
        assert_eq!(secret_manager_id(&r).unwrap(), "prod/db/password");
    }

    #[test]
    fn sm_id_from_path_only() {
        let r = reference("aws-sm", None, "/foo/bar", None);
        assert_eq!(secret_manager_id(&r).unwrap(), "foo/bar");
    }

    #[test]
    fn ssm_name_triple_slash_is_absolute() {
        let r = reference("aws-ssm", None, "/prod/app/api_token", None);
        assert_eq!(ssm_parameter_name(&r).unwrap(), "/prod/app/api_token");
    }

    #[test]
    fn ssm_name_double_slash_gets_leading_slash() {
        let r = reference("aws-ssm", Some("prod"), "/app", None);
        assert_eq!(ssm_parameter_name(&r).unwrap(), "/prod/app");
    }

    #[test]
    fn no_fragment_returns_raw() {
        assert_eq!(select_field("plain-secret", None).unwrap(), "plain-secret");
    }

    #[test]
    fn fragment_selects_json_field() {
        let raw = r#"{"api_key":"abc","db_password":"xyz"}"#;
        assert_eq!(select_field(raw, Some("api_key")).unwrap(), "abc");
    }

    #[test]
    fn fragment_on_non_object_is_invalid_payload() {
        let err = select_field("not json", Some("api_key")).unwrap_err();
        assert!(matches!(err.class, ErrorClass::InvalidSecretPayload));
    }

    #[test]
    fn missing_field_is_invalid_payload() {
        let raw = r#"{"other":"value"}"#;
        let err = select_field(raw, Some("api_key")).unwrap_err();
        assert!(matches!(err.class, ErrorClass::InvalidSecretPayload));
    }

    #[test]
    fn non_string_json_field_is_serialized() {
        let raw = r#"{"port":5432,"enabled":true}"#;
        assert_eq!(select_field(raw, Some("port")).unwrap(), "5432");
        assert_eq!(select_field(raw, Some("enabled")).unwrap(), "true");
    }

    #[test]
    fn bool_parsing() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("FALSE"), Some(false));
        assert_eq!(parse_bool("maybe"), None);
    }
}
