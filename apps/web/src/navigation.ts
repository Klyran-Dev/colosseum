export type ProductWorkspace =
  | "quote"
  | "preview"
  | "document-quote"
  | "checkout"
  | "document-checkout"
  | "governance"
  | "access-control"
  | "replay-ops"
  | "vault-release"
  | "vault-reconciliation"
  | "adjustments"
  | "refund-claims"
  | "refund-cancellations"
  | "refund-executions";

const WORKSPACE_PATHS: Readonly<Record<ProductWorkspace, string>> = {
  quote: "/operations/tax-quotes",
  preview: "/operations/tax-simulator",
  "document-quote": "/operations/invoice-tax",
  checkout: "/operations/payment-review",
  "document-checkout": "/operations/document-payment",
  governance: "/rules/tax-rules",
  "access-control": "/rules/access-approvals",
  "vault-release": "/money/treasury",
  "vault-reconciliation": "/money/balance-checks",
  adjustments: "/money/adjustments",
  "refund-claims": "/money/refund-requests",
  "refund-cancellations": "/money/refund-cancellations",
  "refund-executions": "/money/refund-payments",
  "replay-ops": "/records/exceptions-recovery",
};

const WORKSPACE_BY_PATH = new Map<string, ProductWorkspace>(
  Object.entries(WORKSPACE_PATHS).map(([workspace, path]) => [path, workspace as ProductWorkspace]),
);

export function pathForWorkspace(workspace: ProductWorkspace): string {
  return WORKSPACE_PATHS[workspace];
}

export function workspaceFromPath(pathname: string): ProductWorkspace {
  const normalizedPath = pathname.length > 1 ? pathname.replace(/\/+$/, "") : pathname;
  return WORKSPACE_BY_PATH.get(normalizedPath) ?? "quote";
}

export function isCanonicalWorkspacePath(pathname: string): boolean {
  return WORKSPACE_BY_PATH.has(pathname);
}

/**
 * OIDC providers may redirect to the SPA root or another non-workspace path.
 * Keep the callback query intact until the authentication effect consumes it.
 */
export function isAuthenticationCallback(search: string): boolean {
  const params = new URLSearchParams(search);
  return params.has("code") || params.has("error");
}

export function allWorkspacePaths(): readonly string[] {
  return Object.values(WORKSPACE_PATHS);
}
