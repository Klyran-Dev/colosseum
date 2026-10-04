import { FormEvent, useEffect, useMemo, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { createClientUuid } from "./clientIdentity";
import { AuthorizationSnapshot } from "./authorization";
import { formatInteger } from "./formatters";

const API_BASE = import.meta.env.VITE_API_BASE ?? "/api";

type PlanStatus = "proposed" | "approved" | "active" | "paused" | "cancelled" | "completed";

interface Plan {
  id: string;
  status: PlanStatus;
  proposerSubject: string;
  approverSubject: string | null;
  operatorSubject: string | null;
  startSlot: string;
  endSlot: string | null;
  maxTotalAtomic: string;
  maxClaimAtomic: string;
  accruedTotalAtomic: string;
  releasedTotalAtomic: string;
  nextNonce: string;
  settlementRouteId: string;
  destinationRouteId: string | null;
  asset: number[];
  recipient: number[];
  planDigest: number[];
  policyDigest: number[];
}

interface Props {
  accessToken: string;
  authorizationSnapshot: AuthorizationSnapshot;
  authorizationError: string | null;
}

interface DestinationReference {
  id: string;
  routeKey: string;
  version: number;
  recipientOwner: string;
  mint: string;
  tokenProgram: string;
  verificationSlot: string;
}

interface SplitPlanReference {
  id: string;
  planKey: string;
  version: number;
  mint: string;
}

interface PolicyReference {
  id: string;
  ruleSetId: string;
  ruleExternalId: string;
  ruleVersion: number;
  jurisdiction: string;
  regime: string;
  effectiveFrom: string;
  effectiveUntil: string | null;
}

interface ReferenceCatalog {
  destinations: DestinationReference[];
  splitPlans: SplitPlanReference[];
  policies: PolicyReference[];
}

function objectValue(value: unknown, message: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(message);
  return value as Record<string, unknown>;
}

function textValue(value: unknown, message: string): string {
  if (typeof value !== "string" || value.length === 0) throw new Error(message);
  return value;
}

function bytesValue(value: unknown, message: string): number[] {
  if (!Array.isArray(value) || value.length !== 32 || value.some((byte) => !Number.isInteger(byte) || byte < 0 || byte > 255)) {
    throw new Error(message);
  }
  return value as number[];
}

function atomicValue(value: unknown, message: string): string {
  const text = textValue(value, message);
  try {
    if (BigInt(text) < 0n) throw new Error();
  } catch {
    throw new Error(message);
  }
  return text;
}

function planIdValue(value: unknown): string {
  if (typeof value === "string") return value;
  if (Array.isArray(value) && value.length === 1 && typeof value[0] === "string") return value[0];
  throw new Error("The service returned an invalid payout plan ID.");
}

function parsePlan(value: unknown): Plan {
  const record = objectValue(value, "The service returned an invalid payout plan.");
  const spec = objectValue(record.spec, "The payout plan has no immutable configuration.");
  const status = textValue(record.status, "The payout plan has no status.");
  if (!["proposed", "approved", "active", "paused", "cancelled", "completed"].includes(status)) throw new Error("The payout plan has an unknown status.");
  return {
    id: planIdValue(record.id),
    status: status as PlanStatus,
    proposerSubject: textValue(record.proposer_subject, "The payout plan has no proposer."),
    approverSubject: record.approver_subject === null ? null : textValue(record.approver_subject, "The payout plan has an invalid approver."),
    operatorSubject: spec.operator_subject === null ? null : textValue(spec.operator_subject, "The payout plan has an invalid payout preparer."),
    startSlot: atomicValue(spec.start_slot, "The payout plan start slot is invalid."),
    endSlot: spec.end_slot === null ? null : atomicValue(spec.end_slot, "The payout plan end slot is invalid."),
    maxTotalAtomic: atomicValue(spec.max_total_atomic, "The payout plan total limit is invalid."),
    maxClaimAtomic: atomicValue(spec.max_claim_atomic, "The payout plan claim limit is invalid."),
    accruedTotalAtomic: atomicValue(record.accrued_total_atomic, "The accrued balance is invalid."),
    releasedTotalAtomic: atomicValue(record.released_total_atomic, "The released balance is invalid."),
    nextNonce: atomicValue(record.next_nonce, "The next payout reference is invalid."),
    settlementRouteId: textValue(spec.settlement_route_id, "The payout route binding is invalid."),
    destinationRouteId: record.destination_route_id == null ? null : textValue(record.destination_route_id, "The payout destination verification is invalid."),
    asset: bytesValue(spec.asset, "The payout asset is invalid."),
    recipient: bytesValue(spec.recipient, "The partner destination is invalid."),
    planDigest: bytesValue(spec.plan_digest, "The payout plan fingerprint is invalid."),
    policyDigest: bytesValue(spec.policy_digest, "The policy fingerprint is invalid."),
  };
}

function parsePlans(value: unknown): Plan[] {
  if (!Array.isArray(value)) throw new Error("The service returned an invalid payout plan list.");
  return value.map(parsePlan);
}

function integerValue(value: unknown, message: string): number {
  if (!Number.isInteger(value) || (value as number) <= 0) throw new Error(message);
  return value as number;
}

function parseDestinationReference(value: unknown): DestinationReference {
  const record = objectValue(value, "The service returned an invalid payout destination.");
  return {
    id: textValue(record.id, "The payout destination reference is invalid."),
    routeKey: textValue(record.route_key, "The payout destination label is invalid."),
    version: integerValue(record.version, "The payout destination version is invalid."),
    recipientOwner: textValue(record.recipient_owner, "The payout partner identity is invalid."),
    mint: textValue(record.mint, "The payout asset reference is invalid."),
    tokenProgram: textValue(record.token_program, "The payout token interface is invalid."),
    verificationSlot: atomicValue(record.verification_slot, "The payout verification position is invalid."),
  };
}

function parseSplitPlanReference(value: unknown): SplitPlanReference {
  const record = objectValue(value, "The service returned an invalid settlement split.");
  return {
    id: textValue(record.id, "The settlement split reference is invalid."),
    planKey: textValue(record.plan_key, "The settlement split name is invalid."),
    version: integerValue(record.version, "The settlement split version is invalid."),
    mint: textValue(record.mint, "The settlement split asset is invalid."),
  };
}

function parsePolicyReference(value: unknown): PolicyReference {
  const record = objectValue(value, "The service returned an invalid tax policy.");
  return {
    id: textValue(record.id, "The tax policy reference is invalid."),
    ruleSetId: textValue(record.rule_set_id, "The tax rule reference is invalid."),
    ruleExternalId: textValue(record.rule_external_id, "The tax rule name is invalid."),
    ruleVersion: integerValue(record.rule_version, "The tax rule version is invalid."),
    jurisdiction: textValue(record.jurisdiction, "The tax jurisdiction is invalid."),
    regime: textValue(record.regime, "The tax regime is invalid."),
    effectiveFrom: textValue(record.effective_from, "The tax policy effective date is invalid."),
    effectiveUntil: record.effective_until === null ? null : textValue(record.effective_until, "The tax policy end date is invalid."),
  };
}

function parseReferenceCatalog(value: unknown): ReferenceCatalog {
  const record = objectValue(value, "The service returned an invalid payout setup catalog.");
  if (!Array.isArray(record.destinations) || !Array.isArray(record.split_plans) || !Array.isArray(record.policies)) {
    throw new Error("The payout setup catalog is incomplete.");
  }
  return {
    destinations: record.destinations.map(parseDestinationReference),
    splitPlans: record.split_plans.map(parseSplitPlanReference),
    policies: record.policies.map(parsePolicyReference),
  };
}

function shortAddress(bytes: number[]): string {
  try {
    const address = new PublicKey(Uint8Array.from(bytes)).toBase58();
    return `${address.slice(0, 6)}…${address.slice(-6)}`;
  } catch {
    return "Unavailable";
  }
}

function shortPublicKey(value: string): string {
  try {
    const address = new PublicKey(value).toBase58();
    return `${address.slice(0, 6)}…${address.slice(-6)}`;
  } catch {
    return "Unavailable";
  }
}

function allowed(snapshot: AuthorizationSnapshot, capability: string): boolean {
  return snapshot.state === "server-allowed" && snapshot.capabilities?.[capability]?.kind === "allow";
}

function friendlyStatus(status: PlanStatus): string {
  return status === "proposed" ? "Needs review" : status === "approved" ? "Approved" : status === "active" ? "Receiving accruals" : status === "paused" ? "Paused" : status[0].toUpperCase() + status.slice(1);
}

function available(plan: Plan): string {
  try {
    return (BigInt(plan.accruedTotalAtomic) - BigInt(plan.releasedTotalAtomic)).toString();
  } catch {
    return "0";
  }
}

export default function RevenueStreamingPanel({ accessToken, authorizationSnapshot, authorizationError }: Props) {
  const [plans, setPlans] = useState<Plan[]>([]);
  const [references, setReferences] = useState<ReferenceCatalog | null>(null);
  const [referenceState, setReferenceState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [referenceError, setReferenceError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(authorizationError);
  const [notice, setNotice] = useState<string | null>(null);
  const [destinationRouteId, setDestinationRouteId] = useState("");
  const [splitPlanId, setSplitPlanId] = useState("");
  const [policyId, setPolicyId] = useState("");
  const [operatorSubject, setOperatorSubject] = useState("");
  const [startSlot, setStartSlot] = useState("");
  const [endSlot, setEndSlot] = useState("");
  const [maxTotal, setMaxTotal] = useState("");
  const [maxClaim, setMaxClaim] = useState("");
  const [claimAmount, setClaimAmount] = useState<Record<string, string>>({});
  const [claimKey, setClaimKey] = useState<Record<string, string>>({});
  const [completeSlot, setCompleteSlot] = useState<Record<string, string>>({});

  const canRead = allowed(authorizationSnapshot, "revenue_stream.read");
  const canPropose = allowed(authorizationSnapshot, "revenue_stream.propose");
  const canGovern = allowed(authorizationSnapshot, "revenue_stream.govern");
  const canClaim = allowed(authorizationSnapshot, "revenue_stream.claim.prepare");
  const activeCount = useMemo(() => plans.filter((plan) => plan.status === "active").length, [plans]);
  const totalAvailable = useMemo(() => plans.reduce((total, plan) => total + BigInt(available(plan)), 0n).toString(), [plans]);
  const selectedDestination = useMemo(() => references?.destinations.find((destination) => destination.id === destinationRouteId) ?? null, [references, destinationRouteId]);
  const compatibleSplitPlans = useMemo(() => references?.splitPlans.filter((splitPlan) => !selectedDestination || splitPlan.mint === selectedDestination.mint) ?? [], [references, selectedDestination]);

  useEffect(() => {
    if (!selectedDestination || !references) return;
    if (!compatibleSplitPlans.some((splitPlan) => splitPlan.id === splitPlanId)) setSplitPlanId(compatibleSplitPlans[0]?.id ?? "");
  }, [selectedDestination, references, compatibleSplitPlans, splitPlanId]);

  async function loadPlans(signal?: AbortSignal) {
    if (!accessToken.trim() || !canRead) return;
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetchWithTimeout(`${API_BASE}/v1/revenue-streaming/plans`, { headers: { authorization: `Bearer ${accessToken.trim()}` }, signal }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, "Payout plans could not be loaded."));
      setPlans(parsePlans(payload));
    } catch (cause) {
      if (!(cause instanceof DOMException && cause.name === "AbortError")) setError(cause instanceof Error ? cause.message : "Payout plans could not be loaded.");
    } finally {
      setIsLoading(false);
    }
  }

  async function loadReferences(signal?: AbortSignal) {
    if (!accessToken.trim() || !canPropose) return;
    setReferenceState("loading");
    setReferenceError(null);
    setReferences(null);
    setDestinationRouteId("");
    setSplitPlanId("");
    setPolicyId("");
    try {
      const response = await fetchWithTimeout(`${API_BASE}/v1/revenue-streaming/references`, { headers: { authorization: `Bearer ${accessToken.trim()}` }, signal }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, "Approved payout setup options could not be loaded."));
      const catalog = parseReferenceCatalog(payload);
      setReferences(catalog);
      setDestinationRouteId((current) => catalog.destinations.some((destination) => destination.id === current) ? current : catalog.destinations[0]?.id || "");
      setSplitPlanId((current) => catalog.splitPlans.some((splitPlan) => splitPlan.id === current) ? current : catalog.splitPlans[0]?.id || "");
      setPolicyId((current) => catalog.policies.some((policy) => policy.id === current) ? current : catalog.policies[0]?.id || "");
      setReferenceState("ready");
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      setReferenceState("error");
      setReferenceError(cause instanceof Error ? cause.message : "Approved payout setup options could not be loaded.");
    }
  }

  useEffect(() => {
    const controller = new AbortController();
    void loadPlans(controller.signal);
    return () => controller.abort();
  }, [accessToken, canRead]);

  useEffect(() => {
    const controller = new AbortController();
    void loadReferences(controller.signal);
    return () => controller.abort();
  }, [accessToken, canPropose]);

  async function submitPlan(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    setIsSaving(true);
    try {
      const response = await fetchWithTimeout(`${API_BASE}/v1/revenue-streaming/plans`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({
          idempotency_key: createClientUuid(),
          operator_subject: operatorSubject.trim(),
          destination_route_id: destinationRouteId,
          split_plan_id: splitPlanId,
          policy_id: policyId,
          start_slot: startSlot.trim(),
          end_slot: endSlot.trim() || null,
          max_total_atomic: maxTotal.trim(),
          max_claim_atomic: maxClaim.trim(),
        }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, "The payout plan could not be saved."));
      const plan = parsePlan(payload);
      setPlans((current) => [plan, ...current.filter((item) => item.id !== plan.id)]);
      setNotice("The payout plan is recorded and waiting for its required review.");
      setDestinationRouteId(""); setSplitPlanId(""); setPolicyId(""); setOperatorSubject(""); setStartSlot(""); setEndSlot(""); setMaxTotal(""); setMaxClaim("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The payout plan could not be saved.");
    } finally {
      setIsSaving(false);
    }
  }

  async function transition(plan: Plan, toStatus: string) {
    setError(null); setNotice(null); setIsSaving(true);
    try {
      const response = await fetchWithTimeout(`${API_BASE}/v1/revenue-streaming/plans/${plan.id}/transition`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({ to_status: toStatus, observed_slot: toStatus === "completed" ? completeSlot[plan.id]?.trim() || null : null, evidence: { source: "operator_workflow", confirmation: "The operator reviewed this payout plan state." } }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, "The payout plan could not be updated."));
      setNotice("The new payout plan state is recorded in the audit trail.");
      await loadPlans();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The payout plan could not be updated.");
    } finally {
      setIsSaving(false);
    }
  }

  async function prepareClaim(plan: Plan) {
    setError(null); setNotice(null); setIsSaving(true);
    try {
      const requestReference = claimKey[plan.id] ?? createClientUuid();
      if (!claimKey[plan.id]) setClaimKey((current) => ({ ...current, [plan.id]: requestReference }));
      const response = await fetchWithTimeout(`${API_BASE}/v1/revenue-streaming/plans/${plan.id}/claims`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({ idempotency_key: requestReference, amount_atomic: claimAmount[plan.id]?.trim() || "" }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, "The payout release could not be prepared."));
      setNotice("The payout release is reserved. A verified signer and finalized chain evidence are still required before money moves.");
      await loadPlans();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The payout release could not be prepared.");
    } finally {
      setIsSaving(false);
    }
  }

  if (!accessToken.trim() || authorizationSnapshot.state !== "server-allowed") {
    return <section className="panel empty-state" aria-live="polite"><div className="empty-icon" aria-hidden="true">✓</div><strong>Sign in to review partner payouts</strong><p>{authorizationError ?? "Your account access is checked before payout plans or balances are shown."}</p></section>;
  }
  if (!canRead) {
    return <section className="panel empty-state" aria-live="polite"><div className="empty-icon" aria-hidden="true">i</div><strong>This workspace is not available to your account</strong><p>The service did not grant access to partner payout plans. Ask an administrator for the appropriate responsibility.</p></section>;
  }

  return <div className="revenue-stream-layout">
    <section className="revenue-stream-overview" aria-label="Partner payout summary">
      <div className="metric-card"><span>Active plans</span><strong>{activeCount}</strong><small>Currently accepting finalized accruals</small></div>
      <div className="metric-card"><span>Payout plans</span><strong>{plans.length}</strong><small>Scoped to this account</small></div>
      <div className="metric-card"><span>Available to pay</span><strong>{formatInteger(totalAvailable)}</strong><small>After finalized earnings, before pending releases</small></div>
    </section>
    {error && <p className="inline-error" role="alert">{error}</p>}
    {notice && <p className="inline-success" role="status">{notice}</p>}
    <div className="revenue-stream-columns">
      <section className="panel" aria-labelledby="payout-plans-title">
        <div className="panel-heading"><div><p className="eyebrow">Verified balances</p><h2 id="payout-plans-title">Partner payout plans</h2></div><span className="status-pill">{isLoading ? "Refreshing" : "Live view"}</span></div>
        <p className="panel-copy">Each plan has one partner, one asset, a clear limit, and a recorded approval trail. Balances come from finalized settlement evidence.</p>
        {plans.length === 0 && !isLoading ? <div className="empty-state"><div className="empty-icon" aria-hidden="true">＋</div><strong>No payout plans yet</strong><p>Create a plan when a partner should receive a governed share of finalized settlements.</p></div> : <div className="revenue-plan-list">
          {plans.map((plan) => <article className="revenue-plan-card" key={plan.id}>
            <div className="revenue-plan-topline"><div><strong>Partner payout</strong><small>Destination {shortAddress(plan.recipient)} · asset {shortAddress(plan.asset)}</small></div><span className={`status-pill ${plan.status === "cancelled" ? "status-pill-danger" : plan.status === "proposed" ? "status-pill-warning" : ""}`}>{friendlyStatus(plan.status)}</span></div>
            <dl className="summary-list"><div><dt>Ready to pay</dt><dd>{formatInteger(available(plan))}</dd></div><div><dt>Total payout limit</dt><dd>{formatInteger(plan.maxTotalAtomic)}</dd></div><div><dt>Maximum single payout</dt><dd>{formatInteger(plan.maxClaimAtomic)}</dd></div><div><dt>Earned / paid</dt><dd>{formatInteger(plan.accruedTotalAtomic)} / {formatInteger(plan.releasedTotalAtomic)}</dd></div><div><dt>Payout preparer</dt><dd>{plan.operatorSubject ?? "Needs assignment"}</dd></div><div><dt>Destination check</dt><dd><span className={`status-pill ${plan.destinationRouteId ? "" : "status-pill-warning"}`}>{plan.destinationRouteId ? "Verified" : "Setup required"}</span></dd></div></dl>
            {canGovern && <div className="status-actions" aria-label={`Actions for payout plan ${plan.id}`}>
              {plan.status === "proposed" && <button type="button" disabled={isSaving} onClick={() => void transition(plan, "approved")}>Approve plan</button>}
              {plan.status === "approved" && <button type="button" disabled={isSaving} onClick={() => void transition(plan, "active")}>Start accruals</button>}
              {plan.status === "active" && <button type="button" disabled={isSaving} onClick={() => void transition(plan, "paused")}>Pause new accruals</button>}
              {plan.status === "paused" && <button type="button" disabled={isSaving} onClick={() => void transition(plan, "active")}>Resume accruals</button>}
              {(plan.status === "active" || plan.status === "paused") && <><label className="revenue-inline-field"><span>Finalized position to close the plan</span><input value={completeSlot[plan.id] ?? ""} onChange={(event) => setCompleteSlot((current) => ({ ...current, [plan.id]: event.target.value }))} inputMode="numeric" placeholder="Required to complete" /></label><button type="button" disabled={isSaving || !completeSlot[plan.id]?.trim()} onClick={() => void transition(plan, "completed")}>Complete plan</button></>}
            </div>}
            {canClaim && plan.status === "active" && <form className="revenue-claim-form" onSubmit={(event) => { event.preventDefault(); void prepareClaim(plan); }}><div><strong>Prepare a payout</strong><small>Sets aside the amount for review. It does not move money.</small></div><label><span>Payout amount</span><input required value={claimAmount[plan.id] ?? ""} onChange={(event) => setClaimAmount((current) => ({ ...current, [plan.id]: event.target.value }))} inputMode="numeric" placeholder={`Up to ${plan.maxClaimAtomic}`} /><small>Enter the exact amount used by this payment asset.</small></label><button type="submit" disabled={isSaving}>Prepare payout for review</button></form>}
            <details className="technical-details"><summary>View plan evidence</summary><p>Partner address: <code>{shortAddress(plan.recipient)}</code>. Asset: <code>{shortAddress(plan.asset)}</code>. Settlement route: <code>{plan.settlementRouteId}</code>. Payout destination: <code>{plan.destinationRouteId ?? "not bound"}</code>. Next release reference: <code>{plan.nextNonce}</code>.</p></details>
          </article>)}
        </div>}
      </section>
      {canPropose ? <section className="panel" aria-labelledby="create-payout-title">
        <div className="panel-heading"><div><p className="eyebrow">Controlled setup</p><h2 id="create-payout-title">Create a payout plan</h2></div><span className="status-pill status-pill-warning">Review required</span></div>
        <p className="panel-copy">Choose records that have already been approved and verified. The plan becomes active only after another authorized person reviews it.</p>
        {referenceState === "loading" && <p className="field-help" role="status">Loading approved payout setup options…</p>}
        {referenceError && <p className="inline-error" role="alert">{referenceError}</p>}
        {referenceState === "ready" && references && (references.destinations.length === 0 || references.splitPlans.length === 0 || references.policies.length === 0) && <div className="empty-state"><div className="empty-icon" aria-hidden="true">i</div><strong>Approved setup is not ready</strong><p>Ask an administrator to verify a payout destination, activate a settlement split, and publish a tax policy before creating this plan.</p></div>}
        {referenceState === "ready" && references && references.destinations.length > 0 && references.splitPlans.length > 0 && references.policies.length > 0 && <form className="form-grid" onSubmit={(event) => void submitPlan(event)}>
          <label><span>Verified payout destination</span><select required value={destinationRouteId} onChange={(event) => setDestinationRouteId(event.target.value)}><option value="">Choose a verified partner destination</option>{references.destinations.map((destination) => <option key={destination.id} value={destination.id}>{destination.routeKey} · partner {shortPublicKey(destination.recipientOwner)} · asset {shortPublicKey(destination.mint)}</option>)}</select><small className="field-help">Only finalized-verified destinations are offered. The receiving token account is controlled by the service record.</small></label>
          <label><span>Settlement split</span><select required value={splitPlanId} onChange={(event) => setSplitPlanId(event.target.value)}><option value="">Choose an active settlement split</option>{compatibleSplitPlans.map((splitPlan) => <option key={splitPlan.id} value={splitPlan.id}>{splitPlan.planKey} · version {splitPlan.version} · asset {shortPublicKey(splitPlan.mint)}</option>)}</select><small className="field-help">The service records the exact approved split and checks that its asset matches the destination.</small></label>
          <label><span>Published tax policy</span><select required value={policyId} onChange={(event) => setPolicyId(event.target.value)}><option value="">Choose a published tax policy</option>{references.policies.map((policy) => <option key={policy.id} value={policy.id}>{policy.jurisdiction} · {policy.regime} · {policy.ruleExternalId} v{policy.ruleVersion}</option>)}</select><small className="field-help">Only an effective, published policy can be attached to a new plan.</small></label>
          <label><span>Who prepares payouts?</span><input required value={operatorSubject} onChange={(event) => setOperatorSubject(event.target.value)} placeholder="Their sign-in identity" autoComplete="off" /><small className="field-help">They will later connect an approved wallet. This plan never accepts a wallet address here.</small></label>
          <label><span>Start after settlement</span><input required value={startSlot} onChange={(event) => setStartSlot(event.target.value)} inputMode="numeric" placeholder="Finalized settlement position" /></label>
          <label><span>Stop after settlement</span><input value={endSlot} onChange={(event) => setEndSlot(event.target.value)} inputMode="numeric" placeholder="Leave open-ended if allowed" /></label>
          <label><span>Maximum lifetime payout</span><input required value={maxTotal} onChange={(event) => setMaxTotal(event.target.value)} inputMode="numeric" placeholder="Exact payment-asset amount" /><small className="field-help">This cannot be exceeded by earnings or corrections.</small></label>
          <label><span>Maximum payout at one time</span><input required value={maxClaim} onChange={(event) => setMaxClaim(event.target.value)} inputMode="numeric" placeholder="Exact payment-asset amount" /></label>
          <details className="technical-details full-field"><summary>View selected record evidence</summary><p>Destination reference: <code>{destinationRouteId || "not selected"}</code>. Settlement split reference: <code>{splitPlanId || "not selected"}</code>. Tax policy reference: <code>{policyId || "not selected"}</code>.</p></details>
          <button type="submit" disabled={isSaving || !destinationRouteId || !splitPlanId || !policyId}>{isSaving ? "Saving securely…" : "Create plan for review"}</button>
        </form>}
      </section> : <section className="panel empty-state"><div className="empty-icon" aria-hidden="true">i</div><strong>Plan creation needs a specific responsibility</strong><p>Your account can review payout plans, but it cannot propose new ones.</p></section>}
    </div>
    <p className="revenue-disclaimer">A prepared release is not a transfer. A wallet or governed signer must approve the exact instruction, and the service must observe finalized Solana evidence before the release is shown as paid.</p>
  </div>;
}
