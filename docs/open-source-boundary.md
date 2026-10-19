# Open-core boundary and release gate

## Free and open-source scope

The public `lambda-safe` repository is the independent reference compiler. Its
hard-language semantics, parser, static gates, solver abstraction, supported
solver adapters, canonical artifact construction, certificate verification,
CLI, HTTP service, schemas, examples, tests and public API documentation belong
to the free core. The UI repository is a separate open-source client.

## Deliberately excluded from this repository

Do not commit private or unpublished enterprise implementation, licensed
customer integrations, cloud-hosted tenancy/billing systems, enterprise-only
policy packs, private credentials, hosted-service secrets, internal endpoints,
private packages, or code copied from closed repositories without explicit
authorization to publish it.

Optional future hosted or enterprise offerings must be separate deployment and
distribution targets. They must call the same public, documented semantic
contract and may not alter hard-certification criteria or the meaning of a
lambda certificate.

## Review controls

Every contribution that changes repository contents SHOULD be reviewed for:

- secret/credential or environment-file inclusion;
- private source, internal API names and unpublished dependency URLs;
- license and provenance compatibility;
- hard-law and fail-closed behavior changes;
- generated artifacts, local logs, test data or customer content;
- API/OpenAPI/schema/documentation consistency.

Run a secret scanner and dependency audit in CI. Automated scans are useful
signals, not proof that no confidential material has leaked. Maintainers remain
responsible for reviewing additions and repository history.

## Release gate

A public release must pass formatting, clippy, workspace tests, release build,
the CLI/API smoke tests, container build and Compose configuration validation.
The release record must state which checks actually ran; do not equate local
source inspection with executed verification. The compiler does not claim
formal proof of its own implementation or runtime enforcement outside its API.
