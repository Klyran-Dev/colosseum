import { useCallback, useEffect, useState, type FormEvent } from "react";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { createClientUuidV7 } from "./clientIdentity";
import type { AuthorizationSnapshot } from "./authorization";

interface RoleManagementPanelProps {
  apiBase: string;
  accessToken: string;
  authorizationSnapshot: AuthorizationSnapshot;
}

interface UserDirectoryEntry {
  id: string;
  email: string;
  status: "active" | "disabled";
  createdAtUnix: number;
  lastVerifiedAtUnix: number;
  roles: readonly string[];
}

interface InvitationDirectoryEntry {
  id: string;
  email: string;
  role: "merchant_operator" | "moderator";
  status: "pending" | "accepted" | "revoked" | "expired";
  createdAtUnix: number;
  expiresAtUnix: number;
  acceptedAtUnix: number | null;
  revokedAtUnix: number | null;
}

interface RoleCatalogEntry {
  id: string;
  label: string;
  description: string;
  assignable: boolean;
}

const DIRECTORY_SCHEMA = "auth-user-directory/v1";
const INVITATION_DIRECTORY_SCHEMA = "auth-invitation-directory/v1";
const MUTATION_SCHEMA = "auth-role-mutation/v1";
const INVITATION_SCHEMA = "auth-invitation/v1";
const ROLE_CATALOG_SCHEMA = "auth-role-catalog/v1";
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;

function recordValue(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("The identity service returned an invalid response.");
  return value as Record<string, unknown>;
}

function stringValue(value: unknown, message: string, maxLength = 320): string {
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength) throw new Error(message);
  return value;
}

function integerValue(value: unknown, message: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new Error(message);
  return value;
}

function parseDirectory(payload: unknown): UserDirectoryEntry[] {
  const record = recordValue(payload);
  if (record.schema !== DIRECTORY_SCHEMA || !Array.isArray(record.users) || record.users.length > 100) throw new Error("The identity service returned an invalid user directory.");
  return record.users.map((raw) => {
    const entry = recordValue(raw);
    const id = stringValue(entry.id, "The user directory contains an invalid user ID.", 64);
    const status = stringValue(entry.status, "The user directory contains an invalid status.", 16);
    if (!UUID_PATTERN.test(id) || (status !== "active" && status !== "disabled")) throw new Error("The user directory contains an invalid user.");
    if (!Array.isArray(entry.roles) || entry.roles.length > 32 || entry.roles.some((role) => typeof role !== "string" || role.length === 0 || role.length > 128)) throw new Error("The user directory contains invalid roles.");
    return {
      id,
      email: stringValue(entry.email, "The user directory contains an invalid email."),
      status,
      createdAtUnix: integerValue(entry.created_at_unix, "The user directory contains an invalid creation time."),
      lastVerifiedAtUnix: integerValue(entry.last_verified_at_unix, "The user directory contains an invalid verification time."),
      roles: entry.roles as string[],
    };
  });
}

function isModerator(user: UserDirectoryEntry): boolean {
  return user.roles.includes("moderator");
}

function roleLabel(role: string): string {
  const labels: Record<string, string> = {
    administrator: "Administrator",
    moderator: "Moderator",
    merchant_operator: "Merchant operator",
  };
  return labels[role] ?? role.replace(/[_-]+/gu, " ").replace(/\b\p{L}/gu, (letter) => letter.toUpperCase());
}

function statusLabel(status: string): string {
  const labels: Record<string, string> = {
    active: "Active",
    disabled: "Disabled",
    pending: "Pending",
    accepted: "Accepted",
    revoked: "Revoked",
    expired: "Expired",
  };
  return labels[status] ?? status.replace(/[_-]+/gu, " ").replace(/\b\p{L}/gu, (letter) => letter.toUpperCase());
}

function parseRoleCatalog(payload: unknown): RoleCatalogEntry[] {
  const record = recordValue(payload);
  if (record.schema !== ROLE_CATALOG_SCHEMA || !Array.isArray(record.roles) || record.roles.length > 32) throw new Error("The identity service returned an invalid responsibility catalog.");
  return record.roles.map((raw) => {
    const entry = recordValue(raw);
    const id = stringValue(entry.id, "The responsibility catalog contains an invalid role.", 128);
    const label = stringValue(entry.label, "The responsibility catalog contains an invalid label.", 128);
    const description = stringValue(entry.description, "The responsibility catalog contains an invalid description.", 512);
    if (!/^[a-z0-9_-]+$/u.test(id) || typeof entry.assignable !== "boolean") throw new Error("The responsibility catalog contains an invalid role.");
    return { id, label, description, assignable: entry.assignable };
  });
}

function parseInvitations(payload: unknown): InvitationDirectoryEntry[] {
  const record = recordValue(payload);
  if (record.schema !== INVITATION_DIRECTORY_SCHEMA || !Array.isArray(record.invitations) || record.invitations.length > 100) throw new Error("The identity service returned an invalid invitation directory.");
  return record.invitations.map((raw) => {
    const entry = recordValue(raw);
    const id = stringValue(entry.id, "The invitation directory contains an invalid invitation ID.", 64);
    const email = stringValue(entry.email, "The invitation directory contains an invalid email.");
    const role = stringValue(entry.role, "The invitation directory contains an invalid role.", 32);
    const status = stringValue(entry.status, "The invitation directory contains an invalid status.", 16);
    if (!UUID_PATTERN.test(id) || (role !== "merchant_operator" && role !== "moderator") || !["pending", "accepted", "revoked", "expired"].includes(status)) throw new Error("The invitation directory contains an invalid invitation.");
    const optionalInteger = (value: unknown, message: string): number | null => value === null ? null : integerValue(value, message);
    return {
      id,
      email,
      role: role as InvitationDirectoryEntry["role"],
      status: status as InvitationDirectoryEntry["status"],
      createdAtUnix: integerValue(entry.created_at_unix, "The invitation directory contains an invalid creation time."),
      expiresAtUnix: integerValue(entry.expires_at_unix, "The invitation directory contains an invalid expiry time."),
      acceptedAtUnix: optionalInteger(entry.accepted_at_unix, "The invitation directory contains an invalid acceptance time."),
      revokedAtUnix: optionalInteger(entry.revoked_at_unix, "The invitation directory contains an invalid revocation time."),
    };
  });
}

function invitationDate(unix: number): string {
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(new Date(unix * 1000));
}

export default function RoleManagementPanel({ apiBase, accessToken, authorizationSnapshot }: RoleManagementPanelProps) {
  const [users, setUsers] = useState<UserDirectoryEntry[]>([]);
  const [invitations, setInvitations] = useState<InvitationDirectoryEntry[]>([]);
  const [roleCatalog, setRoleCatalog] = useState<RoleCatalogEntry[]>([]);
  const [state, setState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [invitationState, setInvitationState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [roleCatalogState, setRoleCatalogState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [error, setError] = useState<string | null>(null);
  const [busyUser, setBusyUser] = useState<string | null>(null);
  const [busyInvitation, setBusyInvitation] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [inviteEmail, setInviteEmail] = useState("");
  const [inviteRole, setInviteRole] = useState<"merchant_operator" | "moderator">("merchant_operator");
  const [selectedRoles, setSelectedRoles] = useState<Record<string, string>>({});
  const [inviteState, setInviteState] = useState<"idle" | "saving">("idle");
  const canManage = authorizationSnapshot.state === "server-allowed"
    && authorizationSnapshot.capabilities?.["identity.role.manage"]?.kind === "allow";
  const summaryReady = state === "ready" && invitationState === "ready";
  const summaryUnavailable = state === "error" || invitationState === "error";
  const summaryValue = (value: number): string | number => summaryReady ? value : "—";
  const summaryCopy = summaryUnavailable
    ? "The server projection is unavailable."
    : summaryReady
      ? "Current tenant projection"
      : "Loading server records…";
  const activePeople = users.filter((user) => user.status === "active").length;
  const moderators = users.filter((user) => user.status === "active" && isModerator(user)).length;
  const pendingInvitations = invitations.filter((invitation) => invitation.status === "pending").length;
  const assignableDomainRoles = roleCatalog.filter((role) => role.assignable && role.id !== "moderator");

  const loadUsers = useCallback(async () => {
    if (!accessToken || !canManage) return;
    setState("loading");
    setError(null);
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/admin/users`, {
        headers: { accept: "application/json", authorization: `Bearer ${accessToken}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, "The user directory could not be loaded.");
      setUsers(parseDirectory(payload));
      setState("ready");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The user directory could not be loaded.");
      setState("error");
    }
  }, [accessToken, apiBase, canManage]);

  const loadInvitations = useCallback(async () => {
    if (!accessToken || !canManage) return;
    setInvitationState("loading");
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/admin/invitations`, {
        headers: { accept: "application/json", authorization: `Bearer ${accessToken}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, "The invitation directory could not be loaded.");
      setInvitations(parseInvitations(payload));
      setInvitationState("ready");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The invitation directory could not be loaded.");
      setInvitationState("error");
    }
  }, [accessToken, apiBase, canManage]);

  const loadRoleCatalog = useCallback(async () => {
    if (!accessToken || !canManage) return;
    setRoleCatalogState("loading");
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/admin/role-catalog`, {
        headers: { accept: "application/json", authorization: `Bearer ${accessToken}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, "The responsibility catalog could not be loaded.");
      setRoleCatalog(parseRoleCatalog(payload));
      setRoleCatalogState("ready");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The responsibility catalog could not be loaded.");
      setRoleCatalogState("error");
    }
  }, [accessToken, apiBase, canManage]);

  useEffect(() => { void loadUsers(); void loadInvitations(); void loadRoleCatalog(); }, [loadInvitations, loadRoleCatalog, loadUsers]);

  async function revokeInvitation(invitation: InvitationDirectoryEntry) {
    if (!canManage || busyInvitation || invitation.status !== "pending") return;
    setBusyInvitation(invitation.id);
    setError(null);
    setNotice(null);
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/admin/invitations/${encodeURIComponent(invitation.id)}`, {
        method: "DELETE",
        headers: {
          accept: "application/json",
          authorization: `Bearer ${accessToken}`,
          "idempotency-key": createClientUuidV7(),
        },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, "The invitation could not be revoked.");
      const record = recordValue(payload);
      if (record.schema !== INVITATION_SCHEMA || record.status !== "revoked") throw new Error("The identity service returned an invalid invitation status.");
      setNotice(`Invitation revoked for ${invitation.email}.`);
      await loadInvitations();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The invitation could not be revoked.");
    } finally {
      setBusyInvitation(null);
    }
  }

  async function mutateRole(user: UserDirectoryEntry, role: string) {
    if (!canManage || busyUser) return;
    const assigning = !user.roles.includes(role);
    const label = roleCatalog.find((entry) => entry.id === role)?.label ?? roleLabel(role);
    setBusyUser(user.id);
    setError(null);
    setNotice(null);
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/admin/users/${encodeURIComponent(user.id)}/roles/${encodeURIComponent(role)}`, {
        method: assigning ? "POST" : "DELETE",
        headers: {
          accept: "application/json",
          authorization: `Bearer ${accessToken}`,
          "idempotency-key": createClientUuidV7(),
        },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, "The role change could not be saved.");
      const record = recordValue(payload);
      if (record.schema !== MUTATION_SCHEMA || typeof record.mutation_id !== "string") throw new Error("The identity service returned an invalid role change.");
      setNotice(assigning ? `${label} responsibility assigned to ${user.email}.` : `${label} responsibility revoked from ${user.email}.`);
      await loadUsers();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The role change could not be saved.");
    } finally {
      setBusyUser(null);
    }
  }

  async function createInvitation(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canManage || inviteState === "saving") return;
    setInviteState("saving");
    setError(null);
    setNotice(null);
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/admin/invitations`, {
        method: "POST",
        headers: {
          accept: "application/json",
          "content-type": "application/json",
          authorization: `Bearer ${accessToken}`,
          "idempotency-key": createClientUuidV7(),
        },
        body: JSON.stringify({ email: inviteEmail, role: inviteRole }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, "The invitation could not be created.");
      const record = recordValue(payload);
      if (record.schema !== INVITATION_SCHEMA || typeof record.invitation_id !== "string") throw new Error("The identity service returned an invalid invitation.");
      setNotice(`Invitation created for ${inviteEmail.trim().toLowerCase()}. They can use the invitation link to join this tenant.`);
      setInviteEmail("");
      await loadInvitations();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The invitation could not be created.");
    } finally {
      setInviteState("idle");
    }
  }

  if (!accessToken) return <section className="role-management panel" aria-label="Role administration"><p className="eyebrow">Workspace roles</p><h2>Sign in to manage roles</h2><p className="panel-copy">Administrator access is required to view people in this tenant or change responsibilities.</p></section>;
  if (!canManage) return <section className="role-management panel" aria-label="Role administration"><p className="eyebrow">Workspace roles</p><h2>Administrator approval required</h2><p className="panel-copy">Your account can view access rules, but the service has not granted role-management authority.</p></section>;

  return <section className="role-management panel" aria-label="Role administration">
    <div className="panel-heading"><div><p className="eyebrow">Administrator controls</p><h2>People and responsibilities</h2></div><span className="status-pill">Tenant scoped</span></div>
    <p className="panel-copy">Invite people into this tenant, then assign the bounded responsibilities published by the service catalog. Changes are recorded and appear in the person’s next sign-in token.</p>
    <section className="authorization-posture role-management-summary" aria-label="Tenant identity summary">
      <div><span>Active people</span><strong>{summaryValue(activePeople)}</strong><small>{summaryCopy}</small></div>
      <div><span>Moderators</span><strong>{summaryValue(moderators)}</strong><small>{summaryReady ? "Bounded review responsibility" : summaryCopy}</small></div>
      <div><span>Pending invitations</span><strong>{summaryValue(pendingInvitations)}</strong><small>{summaryReady ? "Awaiting recipient verification" : summaryCopy}</small></div>
    </section>
    {error && <div className="inline-warning" role="alert"><strong>Could not load administrator data.</strong> {error} <button className="text-button" type="button" onClick={() => { void loadUsers(); void loadInvitations(); void loadRoleCatalog(); }}>Try again</button></div>}
    {notice && <div className="inline-success" role="status">{notice}</div>}
    <form className="role-invite-form" onSubmit={(event) => void createInvitation(event)}>
      <div><label htmlFor="tenant-invite-email">Invite by email</label><input id="tenant-invite-email" type="email" value={inviteEmail} onChange={(event) => setInviteEmail(event.target.value)} placeholder="person@example.com" required maxLength={320} /></div>
      <div><label htmlFor="tenant-invite-role">Initial responsibility</label><select id="tenant-invite-role" value={inviteRole} onChange={(event) => setInviteRole(event.target.value as "merchant_operator" | "moderator")}><option value="merchant_operator">Merchant operator</option><option value="moderator">Moderator</option></select></div>
      <button className="primary-button" type="submit" disabled={inviteState === "saving"}>{inviteState === "saving" ? "Creating invitation…" : "Create invitation"}</button>
    </form>
    {roleCatalogState === "ready" && <section className="authorization-posture" aria-label="Available responsibilities"><div><span>Available responsibilities</span><strong>{assignableDomainRoles.length}</strong><small>Explicit server-owned grants; approval remains separate.</small></div><div className="role-chip-list">{assignableDomainRoles.slice(0, 6).map((role) => <span className="decision-chip decision-allow" title={role.description} key={role.id}>{role.label}</span>)}</div></section>}
    {state === "loading" && <p className="panel-copy" role="status" aria-live="polite">Loading people in this tenant…</p>}
    {state === "ready" && users.length === 0 && <div className="empty-state"><strong>No people found</strong><p>There are no active accounts in this tenant yet.</p></div>}
    {state === "ready" && users.length > 0 && <div className="role-directory-wrap"><table className="role-directory"><caption className="sr-only">People and assigned responsibilities</caption><thead><tr><th scope="col">Person</th><th scope="col">Status</th><th scope="col">Responsibilities</th><th scope="col"><span className="sr-only">Action</span></th></tr></thead><tbody>{users.map((user) => { const selectedRole = selectedRoles[user.id] ?? assignableDomainRoles[0]?.id ?? "moderator"; const selectedCatalogRole = roleCatalog.find((role) => role.id === selectedRole); const selectedLabel = selectedCatalogRole?.label ?? roleLabel(selectedRole); const isAssigned = user.roles.includes(selectedRole); return <tr key={user.id}><th scope="row"><span>{user.email}</span><small>{user.id}</small></th><td><span className={`status-pill ${user.status === "active" ? "status-pill-success" : "status-pill-danger"}`}>{statusLabel(user.status)}</span></td><td><div className="role-chip-list">{user.roles.map((role) => <span className="decision-chip decision-allow" key={role}>{roleCatalog.find((entry) => entry.id === role)?.label ?? roleLabel(role)}</span>)}</div></td><td><div className="role-action-stack"><button className="secondary-button role-action" type="button" disabled={user.status !== "active" || busyUser !== null} onClick={() => void mutateRole(user, "moderator")}>{busyUser === user.id ? "Saving…" : isModerator(user) ? "Revoke moderator" : "Assign moderator"}</button>{assignableDomainRoles.length > 0 && <><label className="sr-only" htmlFor={`responsibility-${user.id}`}>Responsibility for {user.email}</label><select id={`responsibility-${user.id}`} value={selectedRole} onChange={(event) => setSelectedRoles((current) => ({ ...current, [user.id]: event.target.value }))} disabled={user.status !== "active" || busyUser !== null}>{assignableDomainRoles.map((role) => <option value={role.id} key={role.id}>{role.label}</option>)}</select><button className="secondary-button role-action" type="button" aria-label={`${isAssigned ? "Revoke" : "Assign"} ${selectedLabel.toLowerCase()} to ${user.email}`} disabled={user.status !== "active" || busyUser !== null} onClick={() => void mutateRole(user, selectedRole)}>{busyUser === user.id ? "Saving…" : `${isAssigned ? "Revoke" : "Assign"} ${selectedLabel.toLowerCase()}`}</button></>}</div></td></tr>; })}</tbody></table></div>}
    <section className="role-invitations" aria-labelledby="tenant-invitations-title">
      <div className="panel-heading"><div><p className="eyebrow">Tenant onboarding</p><h3 id="tenant-invitations-title">Invitation history</h3></div><span className="status-pill">Server record</span></div>
      <p className="panel-copy">Pending invitations can be stopped before they are consumed. Accepted, expired, and revoked invitations remain visible as terminal history.</p>
      {invitationState === "loading" && <p className="panel-copy" role="status" aria-live="polite">Loading invitation history…</p>}
      {invitationState === "ready" && invitations.length === 0 && <div className="empty-state"><strong>No invitations yet</strong><p>Create an invitation above to onboard a person into this tenant.</p></div>}
      {invitationState === "ready" && invitations.length > 0 && <div className="role-directory-wrap"><table className="role-directory"><caption className="sr-only">Tenant invitation history</caption><thead><tr><th scope="col">Recipient</th><th scope="col">Responsibility</th><th scope="col">Status</th><th scope="col">Expiry</th><th scope="col"><span className="sr-only">Action</span></th></tr></thead><tbody>{invitations.map((invitation) => <tr key={invitation.id}><th scope="row"><span>{invitation.email}</span><small>{invitation.id}</small></th><td>{roleLabel(invitation.role)}</td><td><span className={`status-pill ${invitation.status === "pending" ? "status-pill-warning" : invitation.status === "accepted" ? "status-pill-success" : "status-pill-danger"}`}>{statusLabel(invitation.status)}</span></td><td><time dateTime={new Date(invitation.expiresAtUnix * 1000).toISOString()}>{invitationDate(invitation.expiresAtUnix)}</time></td><td>{invitation.status === "pending" ? <button className="secondary-button role-action" type="button" aria-label={`Revoke invitation for ${invitation.email}`} disabled={busyInvitation !== null} onClick={() => void revokeInvitation(invitation)}>{busyInvitation === invitation.id ? "Revoking…" : "Revoke invitation"}</button> : <span className="field-help">Terminal</span>}</td></tr>)}</tbody></table></div>}
    </section>
  </section>;
}

