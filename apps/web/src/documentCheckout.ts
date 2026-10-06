export type DocumentAttestationStatus = {
  status: "verified";
  calculation_id: string;
  payload_digest: number[];
  issuer_public_key: number[];
  expires_at_unix: number;
  checked_at_unix: number;
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

export function parseDocumentAttestationStatus(value: unknown): DocumentAttestationStatus {
  const response = record(value, "The API returned an invalid document evidence status.");
  if (response.status !== "verified") throw new Error("The service did not return a verified invoice record.");
  if (typeof response.calculation_id !== "string" || !UUIDV7.test(response.calculation_id)) throw new Error("The API returned an invalid calculation_id.");
  if (typeof response.expires_at_unix !== "number" || !Number.isSafeInteger(response.expires_at_unix)) throw new Error("The API returned an invalid expires_at_unix.");
  if (typeof response.checked_at_unix !== "number" || !Number.isSafeInteger(response.checked_at_unix)) throw new Error("The API returned an invalid checked_at_unix.");
  return {
    status: "verified",
    calculation_id: response.calculation_id.toLowerCase(),
    payload_digest: digest(response.payload_digest, "document evidence payload digest"),
    issuer_public_key: digest(response.issuer_public_key, "issuer public key"),
    expires_at_unix: response.expires_at_unix,
    checked_at_unix: response.checked_at_unix,
  };
}

export function buildDocumentAttestationStatusUrl(apiBase: string, calculationId: string, routeId: string): string {
  if (!UUIDV7.test(calculationId) || !UUIDV7.test(routeId)) {
    throw new Error("Invoice and payment route references must be UUIDv7.");
  }
  const base = apiBase.replace(/\/$/, "");
  return `${base}/v1/tax/document-quotes/${encodeURIComponent(calculationId)}/attestation/status?route_id=${encodeURIComponent(routeId)}`;
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
