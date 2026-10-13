import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { createClientUuidV7 } from "../clientIdentity";
import { requestJson } from "./data";
import { AdminCapability, AdminSession } from "./model";
import { BROWSER_AUTH_CONFIG, BROWSER_AUTH_SERVICE_BASE_URL } from "../authConfig";

interface AccessDirectoryPanelProps {
  session: AdminSession | null;
  capability?: AdminCapability;
  managementCapability?: AdminCapability;
}

interface UserEntry {
  id: string;
  email: string;
  status: string;
  roles: string[];
  createdAt: number;
  lastVerifiedAt: number;
}

interface InvitationEntry {
  id: string;
  email: string;
  role: string;
  status: string;
  createdAt: number;
  expiresAt: number;
}

interface RoleEntry {
  id: string;
  label: string;
  description: string;
  assignable: boolean;
}

interface AccessDirectory {
  users: UserEntry[];
  invitations: InvitationEntry[];
  roles: RoleEntry[];
}

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function safeUnix(value: unknown): number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : 0;
}

function parseUsers(value: unknown): UserEntry[] {
  const source = objectValue(value);
  if (source.schema !== "auth-user-directory/v1" || !Array.isArray(source.users)) throw new Error("The identity service returned an invalid user directory.");
  return source.users.map((raw) => {
    const entry = objectValue(raw);
    const id = stringValue(entry.id);
    const email = stringValue(entry.email);
    if (!id || !email || (entry.status !== "active" && entry.status !== "disabled") || !Array.isArray(entry.roles) || entry.roles.some((role) => typeof role !== "string")) throw new Error("The identity service returned an invalid user record.");
    return { id, email, status: String(entry.status), roles: entry.roles.filter((role): role is string => typeof role === "string"), createdAt: safeUnix(entry.created_at_unix), lastVerifiedAt: safeUnix(entry.last_verified_at_unix) };
  });
}

function parseInvitations(value: unknown): InvitationEntry[] {
  const source = objectValue(value);
  if (source.schema !== "auth-invitation-directory/v1" || !Array.isArray(source.invitations)) throw new Error("The identity service returned an invalid invitation directory.");
  return source.invitations.map((raw) => {
    const entry = objectValue(raw);
    const id = stringValue(entry.id);
    const email = stringValue(entry.email);
    const role = stringValue(entry.role);
    const status = stringValue(entry.status);
    if (!id || !email || !role || !status) throw new Error("The identity service returned an invalid invitation record.");
    return { id, email, role, status, createdAt: safeUnix(entry.created_at_unix), expiresAt: safeUnix(entry.expires_at_unix) };
  });
}

function parseRoles(value: unknown): RoleEntry[] {
  const source = objectValue(value);
  if (source.schema !== "auth-role-catalog/v1" || !Array.isArray(source.roles)) throw new Error("The identity service returned an invalid responsibility catalog.");
  return source.roles.map((raw) => {
    const entry = objectValue(raw);
    const id = stringValue(entry.id);
    const label = stringValue(entry.label);
    const description = stringValue(entry.description);
    if (!id || !label || !description || typeof entry.assignable !== "boolean") throw new Error("The identity service returned an invalid responsibility record.");
    return { id, label, description, assignable: entry.assignable };
  });
}

function authEndpoint(path: string): string {
  const base = BROWSER_AUTH_SERVICE_BASE_URL || BROWSER_AUTH_CONFIG.emailOtp?.baseUrl || "";
  return `${base}/v1/admin/${path}`;
}

function formatUnix(value: number): string {
  return value ? new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(new Date(value * 1000)) : "—";
}

export default function AccessDirectoryPanel({ session, capability, managementCapability }: AccessDirectoryPanelProps) {
  const queryClient = useQueryClient();
  const enabled = Boolean(session && capability?.decision === "allow");
  const canManage = Boolean(session && managementCapability?.decision === "allow");
  const [inviteEmail, setInviteEmail] = useState("");
  const [inviteRole, setInviteRole] = useState("");
  const query = useQuery({
    queryKey: ["admin", "access-directory", session?.email ?? "anonymous"],
    enabled,
    retry: false,
    queryFn: async (): Promise<AccessDirectory> => {
      const [users, invitations, roles] = await Promise.all([
        requestJson(authEndpoint("users"), session),
        requestJson(authEndpoint("invitations"), session),
        requestJson(authEndpoint("role-catalog"), session),
      ]);
      return { users: parseUsers(users), invitations: parseInvitations(invitations), roles: parseRoles(roles) };
    },
  });
  const roleMutation = useMutation({ mutationFn: async ({ userId, role, assigning }: { userId: string; role: string; assigning: boolean }) => requestJson(authEndpoint(`users/${encodeURIComponent(userId)}/roles/${encodeURIComponent(role)}`), session, { method: assigning ? "POST" : "DELETE", headers: { "idempotency-key": createClientUuidV7() } }), onSuccess: () => { void queryClient.invalidateQueries({ queryKey: ["admin", "access-directory"] }); } });
  const revokeInvitationMutation = useMutation({ mutationFn: async (invitationId: string) => requestJson(authEndpoint(`invitations/${encodeURIComponent(invitationId)}`), session, { method: "DELETE", headers: { "idempotency-key": createClientUuidV7() } }), onSuccess: () => { void queryClient.invalidateQueries({ queryKey: ["admin", "access-directory"] }); } });
  const invitationMutation = useMutation({ mutationFn: async () => { const email = inviteEmail.trim().toLowerCase(); if (!/^[^@\s]+@[^@\s]+\.[^@\s]+$/u.test(email) || !inviteRole) throw new Error("A valid email and assignable responsibility are required."); return requestJson(authEndpoint("invitations"), session, { method: "POST", headers: { "content-type": "application/json", "idempotency-key": createClientUuidV7() }, body: JSON.stringify({ email, role: inviteRole }) }); }, onSuccess: () => { setInviteEmail(""); void queryClient.invalidateQueries({ queryKey: ["admin", "access-directory"] }); } });
  if (!enabled) return null;
  if (query.isPending) return <section className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft" aria-live="polite"><p className="text-sm font-semibold text-admin-muted">Loading server-owned people and responsibility records…</p></section>;
  if (query.error) return <section className="rounded-2xl border border-admin-amber/30 bg-admin-amber-soft p-6 text-sm text-admin-amber-ink" role="alert"><strong className="block">Access directory unavailable</strong><p className="mt-1 leading-5">The identity service did not return a verified directory. No local role assumptions are shown.</p></section>;
  const data = query.data;
  return <section className="space-y-6" aria-label="Server-owned access directory"><div className="grid gap-4 sm:grid-cols-3"><article className="rounded-2xl border border-admin-line bg-admin-surface p-5 shadow-soft"><span className="text-xs font-bold uppercase tracking-[0.12em] text-admin-muted">Active people</span><strong className="mt-3 block text-3xl font-black text-admin-ink">{data.users.filter((user) => user.status === "active").length}</strong><small className="mt-1 block text-xs text-admin-muted">Tenant-scoped directory</small></article><article className="rounded-2xl border border-admin-line bg-admin-surface p-5 shadow-soft"><span className="text-xs font-bold uppercase tracking-[0.12em] text-admin-muted">Pending invitations</span><strong className="mt-3 block text-3xl font-black text-admin-ink">{data.invitations.filter((invitation) => invitation.status === "pending").length}</strong><small className="mt-1 block text-xs text-admin-muted">Server lifecycle state</small></article><article className="rounded-2xl border border-admin-line bg-admin-surface p-5 shadow-soft"><span className="text-xs font-bold uppercase tracking-[0.12em] text-admin-muted">Assignable responsibilities</span><strong className="mt-3 block text-3xl font-black text-admin-ink">{data.roles.filter((role) => role.assignable).length}</strong><small className="mt-1 block text-xs text-admin-muted">Published role catalog</small></article></div>{!canManage && <p className="rounded-xl border border-admin-amber/30 bg-admin-amber-soft p-4 text-sm text-admin-amber-ink" role="status">Directory access is granted, but role and invitation mutations require a separate server decision: <code>identity.role.manage</code>.</p>}<form className="rounded-2xl border border-admin-line bg-admin-surface p-5 shadow-soft" onSubmit={(event) => { event.preventDefault(); if (canManage) invitationMutation.mutate(); }}><div className="flex flex-wrap items-end gap-3"><label className="min-w-[16rem] flex-1" htmlFor="access-invite-email"><span className="mb-2 block text-sm font-bold text-admin-ink">Invite email</span><input id="access-invite-email" type="email" value={inviteEmail} onChange={(event) => setInviteEmail(event.target.value)} className="w-full rounded-xl border border-admin-line bg-admin-bg px-4 py-3 text-sm" required /></label><label className="min-w-[14rem] flex-1" htmlFor="access-invite-role"><span className="mb-2 block text-sm font-bold text-admin-ink">Responsibility</span><select id="access-invite-role" value={inviteRole} onChange={(event) => setInviteRole(event.target.value)} className="w-full rounded-xl border border-admin-line bg-admin-bg px-4 py-3 text-sm" required><option value="">Select server role</option>{data.roles.filter((role) => role.assignable).map((role) => <option key={role.id} value={role.id}>{role.label}</option>)}</select></label><button type="submit" disabled={!canManage || invitationMutation.isPending} className="rounded-xl bg-admin-primary px-4 py-3 text-sm font-bold text-white disabled:cursor-not-allowed disabled:opacity-50">{invitationMutation.isPending ? "Creating…" : "Create invitation"}</button></div>{invitationMutation.error && <p className="mt-3 rounded-xl border border-admin-red/30 bg-admin-red-soft p-3 text-sm text-admin-red" role="alert">{invitationMutation.error instanceof Error ? invitationMutation.error.message : "The invitation service rejected the request."}</p>}</form><div className="grid gap-6 xl:grid-cols-2"><article className="overflow-hidden rounded-2xl border border-admin-line bg-admin-surface shadow-soft"><div className="border-b border-admin-line p-5"><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Identity directory</p><h2 className="mt-1 text-xl font-black text-admin-ink">People</h2></div><div className="overflow-x-auto"><table className="w-full min-w-[48rem] text-left text-sm"><caption className="sr-only">Tenant people and responsibilities</caption><thead className="border-b border-admin-line text-xs uppercase tracking-[0.1em] text-admin-muted"><tr><th className="px-5 py-3 font-black">Identity</th><th className="px-5 py-3 font-black">Responsibilities</th><th className="px-5 py-3 font-black">Last verified</th><th className="px-5 py-3 font-black">Mutation</th></tr></thead><tbody className="divide-y divide-admin-line">{data.users.map((user) => <tr key={user.id}><td className="px-5 py-4"><strong className="block text-admin-ink">{user.email}</strong><span className="text-xs text-admin-muted">{user.status}</span></td><td className="px-5 py-4"><div className="flex flex-wrap gap-1.5">{user.roles.map((role) => <span key={role} className="rounded-full bg-admin-primary-soft px-2 py-1 text-xs font-bold text-admin-primary">{role}</span>)}</div></td><td className="px-5 py-4 text-xs text-admin-muted">{formatUnix(user.lastVerifiedAt)}</td><td className="px-5 py-4"><div className="flex flex-wrap gap-2">{data.roles.filter((role) => role.assignable).map((role) => { const assigned = user.roles.includes(role.id); return <button key={role.id} type="button" disabled={!canManage || roleMutation.isPending} onClick={() => roleMutation.mutate({ userId: user.id, role: role.id, assigning: !assigned })} className="rounded-lg border border-admin-line px-2.5 py-1.5 text-xs font-bold text-admin-ink disabled:cursor-not-allowed disabled:opacity-50">{assigned ? `Revoke ${role.label}` : `Assign ${role.label}`}</button>; })}</div></td></tr>)}</tbody></table>{data.users.length === 0 && <p className="p-6 text-sm text-admin-muted">No verified people are available.</p>}</div></article><article className="overflow-hidden rounded-2xl border border-admin-line bg-admin-surface shadow-soft"><div className="border-b border-admin-line p-5"><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Invitation lifecycle</p><h2 className="mt-1 text-xl font-black text-admin-ink">Invitations</h2></div><div className="overflow-x-auto"><table className="w-full min-w-[42rem] text-left text-sm"><caption className="sr-only">Tenant invitations and lifecycle status</caption><thead className="border-b border-admin-line text-xs uppercase tracking-[0.1em] text-admin-muted"><tr><th className="px-5 py-3 font-black">Email</th><th className="px-5 py-3 font-black">Role</th><th className="px-5 py-3 font-black">Status</th><th className="px-5 py-3 font-black">Expires</th><th className="px-5 py-3 font-black">Mutation</th></tr></thead><tbody className="divide-y divide-admin-line">{data.invitations.map((invitation) => <tr key={invitation.id}><td className="px-5 py-4 text-admin-ink">{invitation.email}</td><td className="px-5 py-4 text-xs text-admin-muted">{invitation.role}</td><td className="px-5 py-4"><span className="rounded-full bg-admin-amber-soft px-2.5 py-1 text-xs font-bold text-admin-amber-ink">{invitation.status}</span></td><td className="px-5 py-4 text-xs text-admin-muted">{formatUnix(invitation.expiresAt)}</td><td className="px-5 py-4"><button type="button" disabled={!canManage || invitation.status !== "pending" || revokeInvitationMutation.isPending} onClick={() => revokeInvitationMutation.mutate(invitation.id)} className="rounded-lg border border-admin-line px-2.5 py-1.5 text-xs font-bold text-admin-ink disabled:cursor-not-allowed disabled:opacity-50">Revoke</button></td></tr>)}</tbody></table>{data.invitations.length === 0 && <p className="p-6 text-sm text-admin-muted">No invitation records are available.</p>}</div></article></div>{(roleMutation.error || revokeInvitationMutation.error) && <p className="rounded-xl border border-admin-red/30 bg-admin-red-soft p-4 text-sm text-admin-red" role="alert">{(roleMutation.error ?? revokeInvitationMutation.error) instanceof Error ? (roleMutation.error ?? revokeInvitationMutation.error)?.message : "The identity service rejected the mutation."}</p>}<article className="rounded-2xl border border-admin-line bg-admin-surface p-5 shadow-soft"><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Server catalog</p><h2 className="mt-1 text-xl font-black text-admin-ink">Responsibility definitions</h2><div className="mt-5 grid gap-3 md:grid-cols-2">{data.roles.map((role) => <div key={role.id} className="rounded-xl border border-admin-line bg-admin-bg p-4"><div className="flex items-center justify-between gap-3"><strong className="text-sm text-admin-ink">{role.label}</strong><span className="text-xs font-bold text-admin-muted">{role.assignable ? "Assignable" : "System-owned"}</span></div><p className="mt-2 text-xs leading-5 text-admin-muted">{role.description}</p></div>)}</div></article></section>;
}
