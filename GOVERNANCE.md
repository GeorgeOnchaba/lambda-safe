# Governance

The project uses maintainer review with public technical decisions.

## Roles

- **Contributors** propose code, specifications, tests or documentation.
- **Maintainers** merge changes, cut releases and steward compatibility.
- **Soundness reviewers** independently review changes to the hard boundary;
  this role may be held by a maintainer or invited domain expert.

## Decisions

Routine changes use pull-request consensus. Semantic changes require an
Architecture Decision Record, conformance evidence and two approvals, including
one soundness review. If consensus cannot be reached, the project lead makes a
documented decision after recording alternatives and objections.

## Releases

Releases are immutable, signed tags. The release record includes source,
checksums, dependency lockfile, test results, SBOM and provenance. Compromised
releases are withdrawn through a security advisory; tags are not silently
moved.

## Independence of the open core

The open implementation contains the full semantics required to parse, decide,
emit and verify a v0.1 hard certificate. Commercial services may add operations
and integrations but may not privately change the meaning of the public
language, theory id, registry digest or certificate while using the same
version identifiers.
