export type TenantStatus = "active" | "suspended";
export type TenantReadinessState = "not_started" | "in_progress" | "ready" | "blocked";
export type TenantSetupStepStatus = "complete" | "pending" | "blocked";

export interface TenantSetupStep {
  id: "workspace_profile" | "settlement_route" | "tax_policy";
  label: string;
  status: TenantSetupStepStatus;
  actionPath: string;
}

export interface TenantContext {
  schema: "auth-tenant-context/v1";
  tenantId: string;
  displayName: string;
  status: TenantStatus;
  readiness: {
    state: TenantReadinessState;
    completedSteps: number;
    totalSteps: number;
    steps: TenantSetupStep[];
  };
}

export const UNNAMED_TENANT_DISPLAY_NAME = "Untitled workspace";

export function tenantDisplayLabel(displayName: string): string {
  return displayName === UNNAMED_TENANT_DISPLAY_NAME ? "Name your workspace" : displayName;
}

export function tenantProfileInputValue(displayName: string): string {
  return displayName === UNNAMED_TENANT_DISPLAY_NAME ? "" : displayName;
}

export function setupProgressLabel(status: TenantSetupStepStatus): string {
  if (status === "complete") return "Ready";
  if (status === "blocked") return "Blocked";
  return "Next step";
}

export function tenantSetupSummary(context: TenantContext): { completed: number; total: number; state: TenantReadinessState; label: string } {
  const { completedSteps, totalSteps, state } = context.readiness;
  return {
    completed: completedSteps,
    total: totalSteps,
    state,
    label: state === "ready" ? "Workspace ready" : state === "blocked" ? "Setup is blocked" : `${completedSteps} of ${totalSteps} setup steps complete`,
  };
}
