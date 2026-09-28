# ADR-0172: Exact decimal wire encoding for Solana `u64` values

## Status

Accepted — implementation in progress.

## Context

Solana nonces, slots, and other protocol counters are unsigned 64-bit values.
Rust preserves their full range, but JSON numbers are not a safe interchange
type for browser JavaScript: values above `2^53 - 1` cannot be represented
exactly by a JavaScript `number`. The adjustment wallet workflow previously
rejected unsafe numeric values in the browser, which prevented silent
corruption but made the API incapable of representing valid high-range
protocol values.

The same boundary applies to refund claim nonces and any future wallet-bound
chain artifact. Financial atomic amounts already use canonical decimal
strings; protocol counters must follow the same rule at browser/API edges.

## Decision

Encode browser-facing Solana `u64` values as canonical base-10 JSON strings.
The serializer always emits the string form. Rust compatibility
deserialization may accept an existing JSON number when reading checked-in
historical vectors, but browser parsers and newly generated artifacts accept
only the canonical string form.

The canonical form is the Rust `u64` decimal representation: no sign, no
leading zeroes except `0`, and a value in `0..=18446744073709551615`. Domain
code remains `u64`; conversion happens only at the serialization boundary.
Browser code keeps the value as a string and validates it without converting
to a lossy `number`. Signed transaction bytes remain the authority for the
encoded instruction.

This slice covers compiled adjustment `adjustment_nonce`, finalized
adjustment registry `registry_slot`, and the shared compiled refund instruction
`claim_nonce`, so the adjacent movement contract cannot reintroduce the same
precision defect.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Keep JSON numbers and reject values above the browser safe-integer limit | Rejected | Fails valid protocol values and makes the API range depend on a client runtime. |
| Send hexadecimal or base64 counters | Rejected | Less interoperable and needlessly different from canonical decimal financial values. |
| Use JavaScript `BigInt` everywhere | Rejected | Does not solve JSON serialization and complicates UI/wallet boundaries. |
| Accept numbers and strings and serialize using the input shape | Rejected | Creates two wire contracts and unstable replay/request bytes. |
| **Serialize canonical decimal strings and preserve Rust `u64` internally** | **Selected** | Exact across languages, deterministic, and consistent with financial-field transport. |

## Invariants and failure policy

- Rust domain values remain bounded `u64`; overflow or non-canonical text fails
  before application logic.
- API responses always serialize covered counters as strings.
- Browser parsers reject numbers, signs, whitespace, leading zeroes, empty
  values, and values above `u64::MAX`.
- The browser never derives a nonce, slot, signature, or instruction digest
  from a lossy numeric conversion.
- Cross-language vectors continue to test compiler semantics independently of
  the browser wire representation.

## Verification

1. Rust serialization tests prove `0`, `2^53`, and `u64::MAX` emit canonical
   strings and historical numeric fixtures still deserialize.
2. Browser tests prove canonical high-range strings parse and malformed or
   numeric counters fail closed.
3. API, web build, workspace tests, integration, and public-document checks
   pass with the updated contract.

## Consequences

Browser clients must treat covered counters as strings. This removes the
JavaScript safe-integer ceiling from wallet-bound protocol evidence and makes
the same rule available to the refund movement slice.
