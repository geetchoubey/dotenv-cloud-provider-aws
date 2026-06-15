//! Mapping of AWS SDK errors onto the shared provider error classes.
//!
//! Error messages must never contain resolved values, tokens, or credentials.

use dotenv_cloud_protocol::ErrorClass;
use dotenv_cloud_provider_sdk::ProviderError;

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
        SdkError::TimeoutError(_) => ProviderError::timeout("request timed out"),
        SdkError::DispatchFailure(cause) => {
            if cause.is_timeout() {
                ProviderError::timeout("connection timed out")
            } else {
                ProviderError::network("network dispatch failure")
            }
        }
        SdkError::ServiceError(se) => {
            // `E: ProvideErrorMetadata` is in scope via the bound, so `.code()`
            // is callable without a redundant import.
            let code = se.err().code().map(str::to_string);
            let class = classify_code(code.as_deref());
            let msg = match code {
                Some(c) => format!("AWS error: {c}"),
                None => "AWS service error".to_string(),
            };
            ProviderError::new(class, msg)
        }
        SdkError::ResponseError(_) => ProviderError::network("invalid response from AWS"),
        SdkError::ConstructionFailure(_) => {
            ProviderError::internal("failed to construct AWS request")
        }
        _ => ProviderError::internal("unknown AWS SDK error"),
    }
}
