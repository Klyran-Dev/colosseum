// @vitest-environment happy-dom

import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ApplicabilityVerificationCard from "./ApplicabilityVerificationCard";
import { APPLICABILITY_VERIFICATION_STATUSES } from "./applicabilityVerification";

const intentId = "0198f18a-7b4b-7d44-9a0f-4e4f5f2c1a11";

function payload(status: string) {
  return {
    settlement_intent_id: intentId,
    status,
    signature: status === "verified" ? "5".repeat(88) : null,
    event_index: 0,
    event_slot: "42",
    provider_slot: status === "verified" ? "43" : null,
    receipt: null,
    binding: null,
    merchant: null,
    mint: null,
    context_id_hex: null,
    attestation_digest_hex: null,
    event_evidence_digest_hex: null,
    verification_commitment: null,
    attempts: status === "pending" || status === "not_applicable" ? null : 2,
    last_error_code: status === "retrying" || status === "dead_lettered" ? "provider_transport_failed" : null,
    verified_at: status === "verified" ? "2026-10-06T00:01:00.000Z" : null,
    claimed_at: "2026-10-06T00:00:00.000Z",
    dead_lettered_at: status === "dead_lettered" ? "2026-10-06T00:02:00.000Z" : null,
  };
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("applicability verification card lifecycle", () => {
  for (const status of APPLICABILITY_VERIFICATION_STATUSES) {
    it(`renders the ${status} state with its own semantic container and the not-finality caveat`, async () => {
      vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(payload(status)), { status: 200 })));
      const { container } = render(
        <ApplicabilityVerificationCard apiBase="/api" accessToken="operator-token" serviceOnline intentId={intentId} streamKey="local/settlement" locale="en-US" />,
      );

      await waitFor(() => expect(container.querySelector(`.applicability-evidence.state-${status}`)).toBeTruthy());
      expect(container.querySelector("button.secondary-button")).toBeTruthy();
      if (status === "verified") expect(container.querySelector(".inline-success")).toBeTruthy();
      else expect(container.querySelector(".field-help")).toBeTruthy();
    });
  }
});
