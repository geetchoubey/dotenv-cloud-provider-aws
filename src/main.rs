//! dotenv-cloud AWS provider plugin.
//!
//! Reads newline-delimited JSON requests on stdin and writes responses on
//! stdout (protocol v1). Resolves `aws-sm://` and `aws-ssm://` references via
//! the AWS SDK. Never prints secret values to stderr or logs.

mod aws;
mod config;
mod error;
mod payload;
mod protocol;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use aws::AwsClients;
use config::AwsConfig;
use error::redact_reference;
use protocol::{Request, Response};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    // AWS clients are built lazily on the first resolve so a handshake-only
    // invocation (validate/doctor) never touches the credential chain.
    let mut clients: Option<AwsClients> = None;

    loop {
        let line = match lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break, // stdin closed: exit cleanly
            Err(e) => {
                eprintln!("error reading stdin: {e}");
                break;
            }
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let response = handle_line(line, &mut clients).await;
        if let Err(e) = write_response(&mut stdout, &response).await {
            eprintln!("error writing stdout: {e}");
            break;
        }
    }
}

async fn handle_line(line: &str, clients: &mut Option<AwsClients>) -> Response {
    let request: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            return Response::Error {
                request_id: None,
                class: error::ErrorClass::Internal.as_str().to_string(),
                message: format!("invalid request JSON: {e}"),
                reference: None,
            };
        }
    };

    match request {
        Request::Handshake { .. } => Response::handshake(),
        Request::Resolve(req) => {
            let request_id = req.request_id.clone();
            let redacted = redact_reference(&req.reference.original);

            // Build clients on first use from this request's provider config.
            if clients.is_none() {
                let cfg = AwsConfig::from_json(&req.provider_config);
                *clients = Some(AwsClients::load(cfg).await);
            }
            let aws = clients.as_ref().expect("clients initialized above");

            match aws.resolve(&req.reference).await {
                Ok(resolved) => Response::ResolveResult {
                    request_id,
                    value: resolved.value,
                    metadata: protocol::Metadata {
                        provider: req.reference.scheme.clone(),
                        version: resolved.version,
                    },
                },
                Err(e) => Response::Error {
                    request_id: Some(request_id),
                    class: e.class.as_str().to_string(),
                    message: e.message,
                    reference: Some(redacted),
                },
            }
        }
    }
}

async fn write_response(
    stdout: &mut tokio::io::Stdout,
    response: &Response,
) -> std::io::Result<()> {
    let mut buf = serde_json::to_vec(response).expect("response serializes");
    buf.push(b'\n');
    stdout.write_all(&buf).await?;
    stdout.flush().await
}
