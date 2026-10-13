import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { requestJson } from "./data";
import { AdminCapability, AdminSession } from "./model";

interface EvidenceAuditPanelProps {
  session: AdminSession | null;
  capability?: AdminCapability;
}

interface EvidenceRecord {
  id: string;
  calculationId: string;
  actor: string;
  requestId: string;
  purpose: string;
  outcome: string;
  occurredAt: string;
}

interface ReplayVerification { status: "replay_verified"; calculationId: string; engineVersion: string; factsDigest: number[]; ruleDigest: number[] }

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function parseEvidence(value: unknown): EvidenceRecord[] {
  if (!Array.isArray(value) || value.length > 100) throw new Error("The evidence service returned an invalid bounded audit projection.");
  return value.map((raw) => {
    const entry = objectValue(raw);
    const id = stringValue(entry.id);
    const calculationId = stringValue(entry.calculation_id);
    const actor = stringValue(entry.actor_subject);
    const requestId = stringValue(entry.request_id);
    const purpose = stringValue(entry.purpose);
    const outcome = stringValue(entry.outcome);
    const occurredAt = stringValue(entry.occurred_at);
    if (!id || !calculationId || !actor || !requestId || !purpose || !outcome || !occurredAt) throw new Error("The evidence service returned an incomplete audit record.");
    return { id, calculationId, actor, requestId, purpose, outcome, occurredAt };
  });
}

function formatTimestamp(value: string): string {
  const date = new Date(value);
  return Number.isFinite(date.getTime()) ? new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(date) : "Unknown time";
}

function parseReplay(value: unknown): ReplayVerification {
  const source = objectValue(value);
  const calculationId = stringValue(source.calculation_id);
  const engineVersion = stringValue(source.engine_version);
  const validDigest = (candidate: unknown): candidate is number[] => Array.isArray(candidate) && candidate.length === 32 && candidate.every((byte) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255);
  if (source.status !== "replay_verified" || !calculationId || !engineVersion || !validDigest(source.facts_digest) || !validDigest(source.rule_digest)) throw new Error("The replay service returned incomplete verification evidence.");
  return { status: "replay_verified", calculationId, engineVersion, factsDigest: [...source.facts_digest], ruleDigest: [...source.rule_digest] };
}

export default function EvidenceAuditPanel({ session, capability }: EvidenceAuditPanelProps) {
  const enabled = Boolean(session && capability?.decision === "allow");
  const [calculationId, setCalculationId] = useState("");
  const query = useQuery({ queryKey: ["admin", "evidence-access", session?.email ?? "anonymous"], enabled, retry: false, queryFn: async () => parseEvidence(await requestJson("/api/v1/audit/evidence-access?limit=50", session)) });
  const replayMutation = useMutation({ mutationFn: async () => { if (!/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu.test(calculationId.trim())) throw new Error("A calculation UUIDv7 is required for replay."); return parseReplay(await requestJson(`/api/v1/tax/quotes/${encodeURIComponent(calculationId.trim())}/replay`, session)); } });
  if (!enabled) return null;
  return <article className="overflow-hidden rounded-2xl border border-admin-line bg-admin-surface shadow-soft"><div className="flex flex-wrap items-start justify-between gap-4 border-b border-admin-line p-5"><div><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Protected projection</p><h2 className="mt-1 text-xl font-black text-admin-ink">Evidence access ledger</h2><p className="mt-2 max-w-2xl text-sm leading-6 text-admin-muted">Redacted access records only. Decrypted facts and request bodies never cross this UI boundary.</p></div><button type="button" onClick={() => void query.refetch()} disabled={query.isFetching} className="rounded-xl border border-admin-line px-4 py-2.5 text-sm font-bold text-admin-ink transition hover:bg-admin-surface-muted disabled:cursor-not-allowed disabled:opacity-50">{query.isFetching ? "Refreshing…" : "Refresh projection"}</button></div><div className="border-b border-admin-line bg-admin-surface-muted p-5"><div className="flex flex-wrap items-end gap-3"><label className="min-w-[20rem] flex-1" htmlFor="evidence-replay-calculation"><span className="mb-2 block text-sm font-bold text-admin-ink">Replay calculation UUIDv7</span><input id="evidence-replay-calculation" value={calculationId} onChange={(event) => setCalculationId(event.target.value)} className="w-full rounded-xl border border-admin-line bg-admin-bg px-4 py-3 font-mono text-sm" /></label><button type="button" onClick={() => replayMutation.mutate()} disabled={!calculationId.trim() || replayMutation.isPending} className="rounded-xl bg-admin-primary px-4 py-3 text-sm font-bold text-white disabled:cursor-not-allowed disabled:opacity-50">{replayMutation.isPending ? "Verifying…" : "Verify replay"}</button></div>{replayMutation.error && <p className="mt-3 rounded-xl border border-admin-red/30 bg-admin-red-soft p-3 text-sm text-admin-red" role="alert">{replayMutation.error instanceof Error ? replayMutation.error.message : "Replay verification failed."}</p>}{replayMutation.data && <div className="mt-4 rounded-xl border border-admin-green/30 bg-admin-green-soft p-4 text-sm text-admin-green" role="status"><strong className="block">Replay verified by the server</strong><dl className="mt-3 grid gap-3 text-xs sm:grid-cols-3"><div><dt>Calculation</dt><dd className="mt-1 break-all font-mono">{replayMutation.data.calculationId}</dd></div><div><dt>Engine</dt><dd className="mt-1 font-mono">{replayMutation.data.engineVersion}</dd></div><div><dt>Digests</dt><dd className="mt-1 font-mono">facts {replayMutation.data.factsDigest.length} bytes · rules {replayMutation.data.ruleDigest.length} bytes</dd></div></dl></div>}</div>{query.isPending && <p className="p-6 text-sm text-admin-muted" aria-live="polite">Loading finalized evidence-access records…</p>}{query.error && <div className="m-5 rounded-xl border border-admin-amber/30 bg-admin-amber-soft p-4 text-sm text-admin-amber-ink" role="alert"><strong className="block">Evidence projection unavailable</strong><p className="mt-1 leading-5">The server did not return a verified audit projection. Retry when the storage boundary is healthy.</p></div>}{query.data && <div className="overflow-x-auto"><table className="w-full min-w-[56rem] text-left text-sm"><caption className="sr-only">Redacted evidence access ledger</caption><thead className="border-b border-admin-line text-xs uppercase tracking-[0.1em] text-admin-muted"><tr><th className="px-5 py-3 font-black">Occurred</th><th className="px-5 py-3 font-black">Purpose</th><th className="px-5 py-3 font-black">Calculation</th><th className="px-5 py-3 font-black">Actor</th><th className="px-5 py-3 font-black">Outcome</th><th className="px-5 py-3 font-black">Request</th></tr></thead><tbody className="divide-y divide-admin-line">{query.data.map((record) => <tr key={record.id}><td className="whitespace-nowrap px-5 py-4 text-xs text-admin-muted">{formatTimestamp(record.occurredAt)}</td><td className="px-5 py-4 font-semibold text-admin-ink">{record.purpose}</td><td className="px-5 py-4 font-mono text-xs text-admin-muted">{record.calculationId}</td><td className="px-5 py-4 font-mono text-xs text-admin-muted">{record.actor}</td><td className="px-5 py-4"><span className={`rounded-full px-2.5 py-1 text-xs font-bold ${record.outcome === "accepted" || record.outcome === "verified" ? "bg-admin-green-soft text-admin-green" : "bg-admin-amber-soft text-admin-amber-ink"}`}>{record.outcome}</span></td><td className="px-5 py-4 font-mono text-xs text-admin-muted">{record.requestId}</td></tr>)}</tbody></table>{query.data.length === 0 && <p className="p-8 text-center text-sm text-admin-muted">No evidence-access records are available for this tenant.</p>}</div>}</article>;
}
