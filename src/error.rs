//! Provider error classes (spec §7.7) and a redaction helper.
//!
//! Error messages must never contain resolved values, tokens, or credentials.

/// A classified provider error. `class` matches the protocol error strings.
#[derive(Debug)]
pub struct ProviderError {
    pub class: ErrorClass,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorClass {
    AuthenticationFailed,
    PermissionDenied,
    NotFound,
    InvalidReference,
    InvalidSecretPayload,
    Timeout,
    RateLimited,
    Network,
    ProviderUnavailable,
    Internal,
}

impl ErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorClass::AuthenticationFailed => "AuthenticationFailed",
            ErrorClass::PermissionDenied => "PermissionDenied",
            ErrorClass::NotFound => "NotFound",
            ErrorClass::InvalidReference => "InvalidReference",
            ErrorClass::InvalidSecretPayload => "InvalidSecretPayload",
            ErrorClass::Timeout => "Timeout",
            ErrorClass::RateLimited => "RateLimited",
            ErrorClass::Network => "Network",
            ErrorClass::ProviderUnavailable => "ProviderUnavailable",
            ErrorClass::Internal => "Internal",
        }
    }
}

impl ProviderError {
    pub fn new(class: ErrorClass, message: impl Into<String>) -> Self {
        ProviderError {
            class,
            message: message.into(),
        }
    }

    pub fn invalid_reference(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::InvalidReference, message)
    }

    pub fn invalid_payload(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::InvalidSecretPayload, message)
    }

    #[allow(dead_code)] // general-purpose constructor used by future call sites.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::Internal, message)
    }
}

/// Map an AWS error code string onto a provider error class.
pub fn classify_code(code: Option<&str>) -> ErrorClass {
    match code.unwrap_or("") {
        "ResourceNotFoundException" | "ParameterNotFound" | "ParameterVersionNotFound" => {
            ErrorClass::NotFound
        }
        "AccessDeniedException" | "UnauthorizedOperation" | "AccessDenied" => {
            ErrorClass::PermissionDenied
        }
        "UnrecognizedClientException"
        | "InvalidSignatureException"
        | "ExpiredTokenException"
        | "IncompleteSignature" => ErrorClass::AuthenticationFailed,
        "ThrottlingException"
        | "Throttling"
        | "TooManyRequestsException"
        | "RequestLimitExceeded"
        | "SlowDown" => ErrorClass::RateLimited,
        "InvalidParameterException" | "InvalidRequestException" | "ValidationException" => {
            ErrorClass::InvalidReference
        }
        "InternalServiceError" | "InternalFailure" | "InternalServerError" => {
            ErrorClass::ProviderUnavailable
        }
        _ => ErrorClass::Internal,
    }
}

/// Map an AWS SDK error onto a [`ProviderError`]. Inspects the SdkError variant
/// for transport-level failures and falls back to the service error code.
pub fn map_sdk_error<E, R>(
    err: aws_smithy_runtime_api::client::result::SdkError<E, R>,
) -> ProviderError
where
    E: aws_smithy_types::error::metadata::ProvideErrorMetadata,
{
    use aws_smithy_runtime_api::client::result::SdkError;

    match &err {
        SdkError::TimeoutError(_) => ProviderError::new(ErrorClass::Timeout, "request timed out"),
        SdkError::DispatchFailure(cause) => {
            if cause.is_timeout() {
                ProviderError::new(ErrorClass::Timeout, "connection timed out")
            } else {
                ProviderError::new(ErrorClass::Network, "network dispatch failure")
            }
        }
        SdkError::ServiceError(se) => {
            // `E: ProvideErrorMetadata` is in scope via the bound above, so
            // `.code()` is callable without a redundant import.
            let code = se.err().code().map(str::to_string);
            let class = classify_code(code.as_deref());
            let msg = match code {
                Some(c) => format!("AWS error: {c}"),
                None => "AWS service error".to_string(),
            };
            ProviderError::new(class, msg)
        }
        SdkError::ResponseError(_) => {
            ProviderError::new(ErrorClass::Network, "invalid response from AWS")
        }
        SdkError::ConstructionFailure(_) => {
            ProviderError::new(ErrorClass::Internal, "failed to construct AWS request")
        }
        _ => ProviderError::new(ErrorClass::Internal, "unknown AWS SDK error"),
    }
}

/// Conservatively redact a reference for inclusion in diagnostics: drop the
/// final path segment and any fragment value (mirrors the core's policy).
pub fn redact_reference(original: &str) -> String {
    if let Some((base, _frag)) = original.split_once('#') {
        return format!("{base}#[redacted]");
    }
    match original.rsplit_once('/') {
        Some((head, tail)) if !tail.is_empty() => format!("{head}/[redacted]"),
        _ => original.to_string(),
    }
}
