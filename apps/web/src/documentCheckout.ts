export type DocumentAttestationVerification = {
  status: "document_attested_unsettled";
  calculation_id: string;
  payload_digest: number[];
  issuer_public_key: number[];
  expires_at_unix: number;
};

const UUIDV7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

function record(value: unknown, message: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(message);
  return value as Record<string, unknown>;
}

function digest(value: unknown, field: string): number[] {
  if (!Array.isArray(value) || value.length !== 32
      || value.some((byte) => typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 255)) {
    throw new Error(`The API returned an invalid ${field}.`);
  }
  return [...value] as number[];
}

export function parseDocumentAttestationJson(value: string): Record<string, unknown> {
  if (new TextEncoder().encode(value).length > 256 * 1024) {
    throw new Error("The signed document attestation is larger than the 256 KiB browser safety bound.");
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    throw new Error("The signed document attestation must be valid JSON.");
  }
  return record(parsed, "The signed document attestation must be a JSON object.");
}

export function parseDocumentAttestationVerification(value: unknown): DocumentAttestationVerification {
  const response = record(value, "The API returned an invalid document attestation response.");
  if (response.status !== "document_attested_unsettled"
      || typeof response.calculation_id !== "string"
      || !UUIDV7.test(response.calculation_id)
      || typeof response.expires_at_unix !== "number"
      || !Number.isSafeInteger(response.expires_at_unix)) {
    throw new Error("The API did not return an active document attestation.");
  }
  return {
    status: "document_attested_unsettled",
    calculation_id: response.calculation_id.toLowerCase(),
    payload_digest: digest(response.payload_digest, "document attestation payload digest"),
    issuer_public_key: digest(response.issuer_public_key, "issuer public key"),
    expires_at_unix: response.expires_at_unix,
  };
}

export function parseDocumentScopeLineIds(value: string): string[] {
  const lineIds = value.split(",").map((lineId) => lineId.trim()).filter(Boolean);
  if (lineIds.length === 0 || lineIds.length > 256) {
    throw new Error("A line subset must contain between one and 256 complete line IDs.");
  }
  const unique = new Set(lineIds);
  if (unique.size !== lineIds.length || lineIds.some((lineId) => lineId.length > 64)) {
    throw new Error("Line subset IDs must be unique and no longer than 64 characters.");
  }
  return [...unique].sort();
}
