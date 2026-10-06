import { FormEvent, useEffect, useMemo, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { createClientUuid } from "./clientIdentity";
import { AuthorizationSnapshot } from "./authorization";
import { DEFAULT_LOCALE, formatInteger } from "./formatters";
import { revenueStreamingMessagesFor, RevenueStreamingMessages } from "./revenueStreamingMessages";
import { operatorErrorMessage } from "./operatorError";

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
  locale?: string;
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

function shortAddress(bytes: number[], unavailable: string): string {
  try {
    const address = new PublicKey(Uint8Array.from(bytes)).toBase58();
    return `${address.slice(0, 6)}…${address.slice(-6)}`;
  } catch {
    return unavailable;
  }
}

function shortPublicKey(value: string, unavailable: string): string {
  try {
    const address = new PublicKey(value).toBase58();
    return `${address.slice(0, 6)}…${address.slice(-6)}`;
  } catch {
    return unavailable;
  }
}

function allowed(snapshot: AuthorizationSnapshot, capability: string): boolean {
  return snapshot.state === "server-allowed" && snapshot.capabilities?.[capability]?.kind === "allow";
}

function friendlyStatus(status: PlanStatus, messages: RevenueStreamingMessages): string {
  return messages.status[status];
}

function available(plan: Plan): string {
  try {
    return (BigInt(plan.accruedTotalAtomic) - BigInt(plan.releasedTotalAtomic)).toString();
  } catch {
    return "0";
  }
}

export default function RevenueStreamingPanel({ accessToken, authorizationSnapshot, authorizationError, locale = DEFAULT_LOCALE }: Props) {
  const messages = revenueStreamingMessagesFor(locale);
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.plansLoad, response.status));
      setPlans(parsePlans(payload));
    } catch (cause) {
      if (!(cause instanceof DOMException && cause.name === "AbortError")) setError(operatorErrorMessage(cause, messages.errors.plansLoad));
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.referencesLoad, response.status));
      const catalog = parseReferenceCatalog(payload);
      setReferences(catalog);
      setDestinationRouteId((current) => catalog.destinations.some((destination) => destination.id === current) ? current : catalog.destinations[0]?.id || "");
      setSplitPlanId((current) => catalog.splitPlans.some((splitPlan) => splitPlan.id === current) ? current : catalog.splitPlans[0]?.id || "");
      setPolicyId((current) => catalog.policies.some((policy) => policy.id === current) ? current : catalog.policies[0]?.id || "");
      setReferenceState("ready");
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      setReferenceState("error");
      setReferenceError(operatorErrorMessage(cause, messages.errors.referencesLoad));
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.save, response.status));
      const plan = parsePlan(payload);
      setPlans((current) => [plan, ...current.filter((item) => item.id !== plan.id)]);
      setNotice(messages.notices.created);
      setDestinationRouteId(""); setSplitPlanId(""); setPolicyId(""); setOperatorSubject(""); setStartSlot(""); setEndSlot(""); setMaxTotal(""); setMaxClaim("");
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.save));
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.update, response.status));
      setNotice(messages.notices.updated);
      await loadPlans();
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.update));
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.prepare, response.status));
      setNotice(messages.notices.prepared);
      await loadPlans();
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.prepare));
    } finally {
      setIsSaving(false);
    }
  }

  if (!accessToken.trim() || authorizationSnapshot.state !== "server-allowed") {
    return <section className="panel empty-state" aria-live="polite"><div className="empty-icon" aria-hidden="true">✓</div><strong>{messages.access.signIn}</strong><p>{authorizationError ?? messages.access.signInCopy}</p></section>;
  }
  if (!canRead) {
    return <section className="panel empty-state" aria-live="polite"><div className="empty-icon" aria-hidden="true">i</div><strong>{messages.access.unavailable}</strong><p>{messages.access.unavailableCopy}</p></section>;
  }

  return <div className="revenue-stream-layout">
    <section className="revenue-stream-overview" aria-label={messages.shell.ariaLabel}>
      <div className="metric-card"><span>{messages.overview.activePlans}</span><strong>{activeCount}</strong><small>{messages.overview.activeHelp}</small></div>
      <div className="metric-card"><span>{messages.overview.payoutPlans}</span><strong>{plans.length}</strong><small>{messages.overview.payoutHelp}</small></div>
      <div className="metric-card"><span>{messages.overview.availableToPay}</span><strong>{formatInteger(totalAvailable, locale)}</strong><small>{messages.overview.availableHelp}</small></div>
    </section>
    {error && <p className="inline-error" role="alert">{error}</p>}
    {notice && <p className="inline-success" role="status">{notice}</p>}
    <div className="revenue-stream-columns">
      <section className="panel" aria-labelledby="payout-plans-title">
        <div className="panel-heading"><div><p className="eyebrow">{messages.shell.eyebrow}</p><h2 id="payout-plans-title">{messages.shell.title}</h2></div><span className="status-pill">{isLoading ? messages.shell.badgeRefreshing : messages.shell.badgeLive}</span></div>
        <p className="panel-copy">{messages.shell.copy}</p>
        {plans.length === 0 && !isLoading ? <div className="empty-state"><div className="empty-icon" aria-hidden="true">＋</div><strong>{messages.plan.noPlans}</strong><p>{messages.plan.noPlansCopy}</p></div> : <div className="revenue-plan-list">
          {plans.map((plan) => <article className="revenue-plan-card" key={plan.id}>
            <div className="revenue-plan-topline"><div><strong>{messages.plan.partnerPayout}</strong><small>{messages.plan.destination} {shortAddress(plan.recipient, messages.details.unavailable)} · {messages.plan.asset} {shortAddress(plan.asset, messages.details.unavailable)}</small></div><span className={`status-pill ${plan.status === "cancelled" ? "status-pill-danger" : plan.status === "proposed" ? "status-pill-warning" : ""}`}>{friendlyStatus(plan.status, messages)}</span></div>
            <dl className="summary-list"><div><dt>{messages.plan.readyToPay}</dt><dd>{formatInteger(available(plan), locale)}</dd></div><div><dt>{messages.plan.totalLimit}</dt><dd>{formatInteger(plan.maxTotalAtomic, locale)}</dd></div><div><dt>{messages.plan.maxSingle}</dt><dd>{formatInteger(plan.maxClaimAtomic, locale)}</dd></div><div><dt>{messages.plan.earnedPaid}</dt><dd>{formatInteger(plan.accruedTotalAtomic, locale)} / {formatInteger(plan.releasedTotalAtomic, locale)}</dd></div><div><dt>{messages.plan.preparer}</dt><dd>{plan.operatorSubject ?? messages.plan.needsAssignment}</dd></div><div><dt>{messages.plan.destinationCheck}</dt><dd><span className={`status-pill ${plan.destinationRouteId ? "" : "status-pill-warning"}`}>{plan.destinationRouteId ? messages.plan.verified : messages.plan.setupRequired}</span></dd></div></dl>
            {canGovern && <div className="status-actions" aria-label={messages.plan.actions(plan.id)}>
              {plan.status === "proposed" && <button type="button" disabled={isSaving} onClick={() => void transition(plan, messages.enumValues.approved)}>{messages.actions.approve}</button>}
              {plan.status === "approved" && <button type="button" disabled={isSaving} onClick={() => void transition(plan, messages.enumValues.active)}>{messages.actions.start}</button>}
              {plan.status === "active" && <button type="button" disabled={isSaving} onClick={() => void transition(plan, messages.enumValues.paused)}>{messages.actions.pause}</button>}
              {plan.status === "paused" && <button type="button" disabled={isSaving} onClick={() => void transition(plan, messages.enumValues.active)}>{messages.actions.resume}</button>}
              {(plan.status === "active" || plan.status === "paused") && <><label className="revenue-inline-field"><span>{messages.actions.completeSlot}</span><input value={completeSlot[plan.id] ?? ""} onChange={(event) => setCompleteSlot((current) => ({ ...current, [plan.id]: event.target.value }))} inputMode="numeric" placeholder={messages.actions.completePlaceholder} /></label><button type="button" disabled={isSaving || !completeSlot[plan.id]?.trim()} onClick={() => void transition(plan, messages.enumValues.completed)}>{messages.actions.complete}</button></>}
            </div>}
            {canClaim && plan.status === "active" && <form className="revenue-claim-form" onSubmit={(event) => { event.preventDefault(); void prepareClaim(plan); }}><div><strong>{messages.actions.preparePayout}</strong><small>{messages.actions.prepareCopy}</small></div><label><span>{messages.actions.amount}</span><input required value={claimAmount[plan.id] ?? ""} onChange={(event) => setClaimAmount((current) => ({ ...current, [plan.id]: event.target.value }))} inputMode="numeric" placeholder={messages.actions.upTo(plan.maxClaimAtomic)} /><small>{messages.actions.amountHelp}</small></label><button type="submit" disabled={isSaving}>{messages.actions.prepare}</button></form>}
            <details className="technical-details"><summary>{messages.details.summary}</summary><p>{messages.details.partnerAddress}: <code>{shortAddress(plan.recipient, messages.details.unavailable)}</code>. {messages.details.asset}: <code>{shortAddress(plan.asset, messages.details.unavailable)}</code>. {messages.details.route}: <code>{plan.settlementRouteId}</code>. {messages.details.destination}: <code>{plan.destinationRouteId ?? messages.details.unbound}</code>. {messages.details.nextReference}: <code>{plan.nextNonce}</code>.</p></details>
          </article>)}
        </div>}
      </section>
      {canPropose ? <section className="panel" aria-labelledby="create-payout-title">
        <div className="panel-heading"><div><p className="eyebrow">{messages.setup.eyebrow}</p><h2 id="create-payout-title">{messages.setup.title}</h2></div><span className="status-pill status-pill-warning">{messages.setup.badge}</span></div>
        <p className="panel-copy">{messages.setup.copy}</p>
        {referenceState === "loading" && <p className="field-help" role="status">{messages.setup.loading}</p>}
        {referenceError && <p className="inline-error" role="alert">{referenceError}</p>}
        {referenceState === "ready" && references && (references.destinations.length === 0 || references.splitPlans.length === 0 || references.policies.length === 0) && <div className="empty-state"><div className="empty-icon" aria-hidden="true">i</div><strong>{messages.setup.incomplete}</strong><p>{messages.setup.incompleteCopy}</p></div>}
        {referenceState === "ready" && references && references.destinations.length > 0 && references.splitPlans.length > 0 && references.policies.length > 0 && <form className="form-grid" onSubmit={(event) => void submitPlan(event)}>
          <label><span>{messages.setup.destination}</span><select required value={destinationRouteId} onChange={(event) => setDestinationRouteId(event.target.value)}><option value="">{messages.setup.destinationPlaceholder}</option>{references.destinations.map((destination) => <option key={destination.id} value={destination.id}>{destination.routeKey} · {messages.plan.destination} {shortPublicKey(destination.recipientOwner, messages.details.unavailable)} · {messages.plan.asset} {shortPublicKey(destination.mint, messages.details.unavailable)}</option>)}</select><small className="field-help">{messages.setup.destinationHelp}</small></label>
          <label><span>{messages.setup.split}</span><select required value={splitPlanId} onChange={(event) => setSplitPlanId(event.target.value)}><option value="">{messages.setup.splitPlaceholder}</option>{compatibleSplitPlans.map((splitPlan) => <option key={splitPlan.id} value={splitPlan.id}>{splitPlan.planKey} · {splitPlan.version} · {messages.plan.asset} {shortPublicKey(splitPlan.mint, messages.details.unavailable)}</option>)}</select><small className="field-help">{messages.setup.splitHelp}</small></label>
          <label><span>{messages.setup.policy}</span><select required value={policyId} onChange={(event) => setPolicyId(event.target.value)}><option value="">{messages.setup.policyPlaceholder}</option>{references.policies.map((policy) => <option key={policy.id} value={policy.id}>{policy.jurisdiction} · {policy.regime} · {policy.ruleExternalId} v{policy.ruleVersion}</option>)}</select><small className="field-help">{messages.setup.policyHelp}</small></label>
          <label><span>{messages.setup.operator}</span><input required value={operatorSubject} onChange={(event) => setOperatorSubject(event.target.value)} placeholder={messages.setup.operatorPlaceholder} autoComplete="off" /><small className="field-help">{messages.setup.operatorHelp}</small></label>
          <label><span>{messages.setup.start}</span><input required value={startSlot} onChange={(event) => setStartSlot(event.target.value)} inputMode="numeric" placeholder={messages.setup.startPlaceholder} /></label>
          <label><span>{messages.setup.end}</span><input value={endSlot} onChange={(event) => setEndSlot(event.target.value)} inputMode="numeric" placeholder={messages.setup.endPlaceholder} /></label>
          <label><span>{messages.setup.maxTotal}</span><input required value={maxTotal} onChange={(event) => setMaxTotal(event.target.value)} inputMode="numeric" placeholder={messages.setup.maxTotalPlaceholder} /><small className="field-help">{messages.setup.maxTotalHelp}</small></label>
          <label><span>{messages.setup.maxClaim}</span><input required value={maxClaim} onChange={(event) => setMaxClaim(event.target.value)} inputMode="numeric" placeholder={messages.setup.maxClaimPlaceholder} /></label>
          <details className="technical-details full-field"><summary>{messages.setup.evidence}</summary><p>{messages.details.destinationReference}: <code>{destinationRouteId || messages.setup.notSelected}</code>. {messages.details.splitReference}: <code>{splitPlanId || messages.setup.notSelected}</code>. {messages.details.policyReference}: <code>{policyId || messages.setup.notSelected}</code>.</p></details>
          <button type="submit" disabled={isSaving || !destinationRouteId || !splitPlanId || !policyId}>{isSaving ? messages.actions.saving : messages.setup.create}</button>
        </form>}
      </section> : <section className="panel empty-state"><div className="empty-icon" aria-hidden="true">i</div><strong>{messages.setup.createDenied}</strong><p>{messages.setup.createDeniedCopy}</p></section>}
    </div>
    <p className="revenue-disclaimer">{messages.boundary.disclaimer}</p>
  </div>;
}
