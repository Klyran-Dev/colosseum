# Jurisdiction-resolution bounded context

The jurisdiction context resolves normalized, externally evidenced signals into
one jurisdiction for a tax calculation. It is deliberately separate from the
tax arithmetic engine and from provider adapters.

```text
provider adapters / merchant evidence
  -> authenticated normalized candidates
  -> versioned ResolutionPolicy
  -> deterministic JurisdictionResolution
  -> normalized tax facts + calculation attestation
```

## Contract

`ResolutionPolicy` declares an immutable ordered list of typed evidence sources
and a conflict mode. `JurisdictionCandidate` contains a canonical jurisdiction
code, source, and digest of the evidence envelope. The resolver never trusts a
caller-provided source rank, confidence score, array position, or provider
preference.

`HighestPrecedence` selects the highest configured source present and rejects
different jurisdiction codes at that rank. `RequireAgreement` rejects any
different codes, even if they came from lower-precedence sources. Both modes
fail when there are no valid candidates.

The result contains the selected jurisdiction/source, policy digest, evidence
digest, candidate count, and a bounded trace. The policy and evidence digests
are part of calculation evidence; changing provider data, policy order, or
conflict mode creates a different result.

## Trust boundary

The pure crate does not verify a driver's license, geocode an address, call a
tax authority, or decide whether a jurisdiction's law applies. An application
adapter must authenticate and validate those inputs, enforce tenant scope and
effective windows, and produce a digest over the evidence envelope. Legal/tax
owners approve each jurisdiction-specific policy and evidence contract.

## Release boundary

The current implementation proves the deterministic domain contract and golden
vectors. It does not claim jurisdiction coverage, legal compliance, filing,
reporting, or production provider availability. Before enabling a jurisdiction
in production, the release must add reviewed policy provenance, provider
contracts, privacy controls, replay persistence, attestation binding, and
jurisdiction-specific legal/tax approval.

