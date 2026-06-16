# dotenv-cloud-provider-aws

AWS provider plugin for [`dotenv-cloud`](https://github.com/geetchoubey/dotenv-cloud).

Resolves remote secret references for two URI schemes:

| Scheme | AWS API |
| --- | --- |
| `aws-secrets://` | AWS Secrets Manager (`GetSecretValue`) |
| `aws-ssm://` | AWS SSM Parameter Store (`GetParameter`) |

This is a standalone executable. The `dotenv-cloud` core launches it as a child
process and exchanges newline-delimited JSON over stdin/stdout (protocol v1).
It links the AWS SDK; the core binary does not. See the core repo's
[`docs/PROVIDER_PROTOCOL.md`](https://github.com/geetchoubey/dotenv-cloud/blob/main/docs/PROVIDER_PROTOCOL.md)
for the wire contract.

## Status

🚧 Early development. The repository is being scaffolded.

## URI forms

AWS Secrets Manager:

```dotenv
DB_PASSWORD=aws-secrets://prod/db/password
API_KEY=aws-secrets://prod/app/config#api_key          # fragment selects a JSON field
DB_PASSWORD=aws-secrets://prod/db/password?version_id=abc
DB_PASSWORD=aws-secrets://prod/db/password?version_stage=AWSCURRENT
```

AWS SSM Parameter Store (parameter names are absolute paths, hence three slashes):

```dotenv
API_TOKEN=aws-ssm:///prod/app/api_token
API_TOKEN=aws-ssm:///prod/app/api_token?with_decryption=true
```

## Authentication

Uses the AWS default credential chain (environment credentials, shared config
profiles, SSO, web identity, IAM roles, instance/task roles). Credentials are
never managed or persisted by `dotenv-cloud` or this plugin. Optional `region`
and `profile` may be supplied via the core's `[providers.aws]` config.

## Installation

Once released, install via the core CLI:

```sh
dotenv-cloud providers install aws
```

Or place the built executable and a `manifest.toml` under
`.dotenv-cloud/providers/aws/` (project-local) or the user provider directory.

## License

Licensed under either of MIT or Apache-2.0 at your option.
