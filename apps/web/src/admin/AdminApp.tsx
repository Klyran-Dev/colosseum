import { lazy, ReactNode, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { createColumnHelper, flexRender, tableFeatures, useTable } from "@tanstack/react-table";
import { Link, Navigate, Outlet, RouterProvider, createBrowserHistory, createMemoryHistory, createRootRoute, createRoute, createRouter, useNavigate, useRouterState } from "@tanstack/react-router";
import { fetchAdminSnapshot } from "./data";
import { AdminAction, AdminCapability, AdminMetric, AdminQueryError, AdminSection, AdminSession, AdminSnapshot, NAV_ITEMS, SECTION_COPY } from "./model";
import WorkflowPanel from "./WorkflowPanel";
import AccessDirectoryPanel from "./AccessDirectoryPanel";
import EvidenceAuditPanel from "./EvidenceAuditPanel";
const TaxQuotesPage = lazy(() => import("./TaxQuotesPage"));
const SettingsPage = lazy(() => import("./SettingsPage"));
const RuleGovernancePage = lazy(() => import("./RuleGovernancePage"));
const SettlementOperationsPage = lazy(() => import("./SettlementOperationsPage"));
const TreasuryOperationsPage = lazy(() => import("./TreasuryOperationsPage"));
import { SessionContext, useAdminSession } from "./session";
import AdminAuthPage from "./AdminAuthPage";

interface AdminAppProps { initialPath?: string }

import type { SessionContextValue } from "./session";

function Icon({ name }: { name: "grid" | "quote" | "rules" | "settlement" | "treasury" | "access" | "evidence" | "search" | "bell" | "chevron" | "arrow" | "shield" }) {
  const paths: Record<string, ReactNode> = {
    grid: <><rect x="4" y="4" width="6" height="6" rx="1" /><rect x="14" y="4" width="6" height="6" rx="1" /><rect x="4" y="14" width="6" height="6" rx="1" /><rect x="14" y="14" width="6" height="6" rx="1" /></>,
    quote: <><path d="M6 4.5h12v15H6z" /><path d="M9 8h6M9 12h6M9 16h4" /></>,
    rules: <><path d="M5 6h14M5 12h14M5 18h14" /><circle cx="9" cy="6" r="2" /><circle cx="15" cy="12" r="2" /><circle cx="11" cy="18" r="2" /></>,
    settlement: <><path d="M4 7h16v12H4z" /><path d="M7 7V5h10v2M8 12h8" /></>,
    treasury: <><path d="M4 8h16v12H4z" /><path d="M7 8V5h10v3M8 14h8" /></>,
    access: <><path d="M12 3.5 19 6v5.5c0 4.5-2.8 7.5-7 9-4.2-1.5-7-4.5-7-9V6z" /><path d="m9 12 2 2 4-4" /></>,
    evidence: <><path d="M6 3.5h9l3 3v14H6z" /><path d="M15 3.5v4h3M9 12h6M9 15h4" /></>,
    search: <><circle cx="10.8" cy="10.8" r="6.3" /><path d="m16 16 4.3 4.3" /></>,
    bell: <><path d="M6 17h12l-1.4-2.1V10a4.6 4.6 0 0 0-9.2 0v4.9z" /><path d="M10 20h4" /></>,
    chevron: <path d="m8 10 4 4 4-4" />,
    arrow: <><path d="M5 12h14" /><path d="m13 6 6 6-6 6" /></>,
    shield: <><path d="M12 3.5 19 6v5.5c0 4.5-2.8 7.5-7 9-4.2-1.5-7-4.5-7-9V6z" /><path d="m9 12 2 2 4-4" /></>,
  };
  return <svg aria-hidden="true" className="size-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">{paths[name]}</svg>;
}

function AppLogo() {
  return <div className="flex items-center gap-3"><span className="grid size-10 place-items-center rounded-xl bg-admin-primary text-lg font-black text-white shadow-soft">S</span><span className="leading-tight"><strong className="block text-sm font-extrabold tracking-tight text-admin-ink">Tax operations</strong><small className="block text-xs text-admin-muted">Control plane</small></span></div>;
}

function statusClass(status: AdminAction["status"]): string {
  return status === "verified" ? "bg-admin-green-soft text-admin-green" : status === "pending" ? "bg-admin-amber-soft text-admin-amber-ink" : status === "blocked" ? "bg-admin-red-soft text-admin-red" : "bg-admin-surface-muted text-admin-muted";
}

function adminError(error: unknown): AdminQueryError | null {
  if (!error || typeof error !== "object") return null;
  const value = error as Partial<AdminQueryError>;
  return value.code === "unavailable" || value.code === "unauthorized" || value.code === "invalid" ? { code: value.code, message: typeof value.message === "string" ? value.message : "The admin service returned an unknown error." } : null;
}

function sessionIsUsable(session: AdminSession | null): session is AdminSession {
  return Boolean(session?.accessToken && Number.isFinite(session.expiresAtUnix) && session.expiresAtUnix > Math.floor(Date.now() / 1000));
}

function AdminLayout() {
  const { session, signOut } = useAdminSession();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const navigate = useNavigate();
  const [mobileNavigationOpen, setMobileNavigationOpen] = useState(false);
  const navigationLinks = NAV_ITEMS.map((item) => { const active = item.to === "/admin" ? pathname === "/admin" : pathname.startsWith(item.to); const icon = item.section === "overview" ? "grid" : item.section === "quotes" ? "quote" : item.section === "rules" ? "rules" : item.section === "settlements" ? "settlement" : item.section === "treasury" ? "treasury" : item.section === "access" ? "access" : "evidence"; return <Link key={item.to} to={item.to} onClick={() => setMobileNavigationOpen(false)} className={`group flex items-center gap-3 rounded-xl px-3 py-3 text-sm font-semibold transition ${active ? "bg-admin-primary-soft text-admin-primary" : "text-admin-muted hover:bg-admin-surface-muted hover:text-admin-ink"}`} activeProps={{ "aria-current": "page" }}><Icon name={icon} /><span>{item.label}</span>{item.section === "access" && <span className="ml-auto rounded-full bg-admin-amber-soft px-2 py-0.5 text-[0.65rem] font-bold text-admin-amber-ink">Policy</span>}</Link>; });
  return <div className="min-h-screen bg-admin-bg text-admin-ink">
    <aside className="fixed inset-y-0 left-0 z-30 hidden w-64 flex-col border-r border-admin-line bg-admin-surface px-5 py-6 lg:flex" aria-label="Admin navigation">
      <div className="mb-10"><Link to="/admin" aria-label="Tax operations home"><AppLogo /></Link></div>
      <p className="mb-3 px-3 text-[0.68rem] font-black uppercase tracking-[0.16em] text-admin-muted">Workspace</p>
      <nav className="space-y-1" aria-label="Workspace navigation">{navigationLinks}</nav>
      <div className="mt-auto rounded-2xl bg-admin-primary-soft p-4"><div className="mb-3 flex size-9 items-center justify-center rounded-xl bg-admin-primary text-white"><Icon name="shield" /></div><strong className="block text-sm text-admin-ink">Governed workspace</strong><p className="mt-1 text-xs leading-5 text-admin-muted">Roles, attributes, and approvals stay server-owned.</p><span className="mt-3 inline-flex items-center gap-2 text-xs font-bold text-admin-primary"><span className="size-2 rounded-full bg-admin-green" /> RBAC · ABAC · PBAC</span></div>
    </aside>
    <div className="lg:pl-64">
      <header className="sticky top-0 z-20 border-b border-admin-line bg-admin-surface/95 px-5 py-4 backdrop-blur lg:px-9"><div className="flex items-center justify-between gap-4"><div className="flex items-center gap-3"><button type="button" className="grid size-10 place-items-center rounded-xl border border-admin-line text-admin-muted lg:hidden" aria-expanded={mobileNavigationOpen} aria-controls="mobile-admin-navigation" aria-label="Open admin navigation" onClick={() => setMobileNavigationOpen((open) => !open)}><span aria-hidden="true">☰</span></button><div className="lg:hidden"><Link to="/admin" aria-label="Tax operations home"><AppLogo /></Link></div><div className="hidden min-w-0 lg:block"><p className="text-xs font-bold uppercase tracking-[0.14em] text-admin-muted">Tenant control plane</p><p className="truncate text-sm font-semibold text-admin-ink">Solana payment, tax, and evidence operations</p></div></div><div className="flex items-center gap-2 sm:gap-3"><Link to="/admin/evidence" className="grid size-10 place-items-center rounded-xl border border-admin-line text-admin-muted transition hover:bg-admin-surface-muted hover:text-admin-ink" aria-label="Search evidence"><Icon name="search" /></Link><Link to="/admin/evidence" className="relative grid size-10 place-items-center rounded-xl border border-admin-line text-admin-muted transition hover:bg-admin-surface-muted hover:text-admin-ink" aria-label="Open notifications and evidence"><Icon name="bell" /><span className="absolute right-2 top-2 size-1.5 rounded-full bg-admin-primary" /></Link>{session ? <div className="flex items-center gap-3 border-l border-admin-line pl-3"><span className="hidden text-right sm:block"><strong className="block text-sm text-admin-ink">{session.email}</strong><small className="block text-xs text-admin-muted">Verified session</small></span><button type="button" onClick={signOut} className="grid size-10 place-items-center rounded-full bg-admin-primary-soft text-sm font-black text-admin-primary" aria-label="Sign out">{session.email.slice(0, 1).toUpperCase()}</button></div> : <Link to="/auth/signin" className="rounded-xl bg-admin-primary px-4 py-2.5 text-sm font-bold text-white shadow-soft transition hover:bg-admin-primary-strong">Sign in</Link>}</div></div>{mobileNavigationOpen && <nav id="mobile-admin-navigation" className="mt-4 space-y-1 border-t border-admin-line pt-4 lg:hidden" aria-label="Mobile admin navigation">{navigationLinks}</nav>}</header>
      <main className="min-h-[calc(100vh-4.75rem)] px-5 py-7 lg:px-9 lg:py-9"><Outlet /></main>
    </div>
  </div>;
}

function AdminRouteBoundary() {
  const { session, signOut } = useAdminSession();
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const [, refreshExpiry] = useState(0);
  const usable = sessionIsUsable(session);
  useEffect(() => {
    if (!usable) {
      if (session) signOut();
      void navigate({ to: "/auth/signin", search: { next: pathname }, replace: true });
      return;
    }
    const timeout = window.setTimeout(() => refreshExpiry((value) => value + 1), Math.max(1_000, (session.expiresAtUnix - Math.floor(Date.now() / 1000)) * 1_000));
    return () => window.clearTimeout(timeout);
  }, [navigate, refreshExpiry, session, signOut, usable]);
  if (usable) return <AdminLayout />;
  return <main className="grid min-h-screen place-items-center bg-admin-bg px-5 py-10"><section className="w-full max-w-md rounded-3xl border border-admin-line bg-admin-surface p-8 text-center shadow-panel"><AppLogo /><h1 className="mt-10 text-3xl font-black tracking-tight text-admin-ink">Sign-in required</h1><p className="mt-3 text-sm leading-6 text-admin-muted">The admin dashboard is protected. Verify your operator identity before tenant data, capabilities, or workflows are loaded.</p><Link to="/auth/signin" className="mt-7 inline-flex rounded-xl bg-admin-primary px-5 py-3 text-sm font-bold text-white shadow-soft">Continue to sign in</Link></section></main>;
}

function PageHeader({ eyebrow, title, description, action }: { eyebrow: string; title: string; description: string; action?: ReactNode }) {
  return <div className="mb-7 flex flex-col justify-between gap-5 sm:flex-row sm:items-end"><div><p className="mb-2 text-xs font-black uppercase tracking-[0.16em] text-admin-primary">{eyebrow}</p><h1 className="text-3xl font-black tracking-[-0.04em] text-admin-ink sm:text-4xl">{title}</h1><p className="mt-2 max-w-2xl text-sm leading-6 text-admin-muted">{description}</p></div>{action}</div>;
}

function StatusBanner({ error }: { error: unknown }) {
  const parsed = adminError(error);
  if (!parsed) return null;
  return <div className="mb-6 flex items-start gap-3 rounded-2xl border border-admin-amber/30 bg-admin-amber-soft p-4 text-sm text-admin-amber-ink" role="alert"><span className="mt-0.5 size-2 shrink-0 rounded-full bg-admin-amber" /><div><strong className="block">Verified dashboard data is unavailable</strong><p className="mt-1 leading-5">{parsed.message} Protected actions remain unavailable until the service returns a fresh decision.</p>{parsed.correlationId && <code className="mt-2 block text-xs">Correlation {parsed.correlationId}</code>}</div></div>;
}

function SnapshotStaleBanner() {
  return <div className="mb-6 flex items-start gap-3 rounded-2xl border border-admin-amber/30 bg-admin-amber-soft p-4 text-sm text-admin-amber-ink" role="alert"><span className="mt-0.5 size-2 shrink-0 rounded-full bg-admin-amber" /><div><strong className="block">Authorization snapshot expired</strong><p className="mt-1 leading-5">Protected controls are paused while the service obtains a fresh RBAC, ABAC, and PBAC decision.</p></div></div>;
}

function WorkspaceLoading() {
  return <div className="grid min-h-48 place-items-center rounded-2xl border border-admin-line bg-admin-surface p-8 text-center" role="status"><div><strong className="block text-sm text-admin-ink">Loading workspace</strong><p className="mt-1 text-sm leading-6 text-admin-muted">Fetching the route-specific admin surface…</p></div></div>;
}

function MetricCard({ metric }: { metric: AdminMetric }) {
  const iconName = metric.tone === "cyan" ? "settlement" : metric.tone === "amber" ? "rules" : metric.tone === "green" ? "shield" : "quote";
  return <article className="rounded-2xl border border-admin-line bg-admin-surface p-5 shadow-soft"><div className="flex items-start justify-between gap-4"><div><p className="text-xs font-bold uppercase tracking-[0.12em] text-admin-muted">{metric.label}</p><strong className="mt-3 block text-2xl font-black tracking-tight text-admin-ink">{metric.value || "—"}</strong></div><span className="grid size-10 place-items-center rounded-xl bg-admin-primary-soft text-admin-primary"><Icon name={iconName} /></span></div><div className="mt-4 flex items-center justify-between gap-3 text-xs"><span className="text-admin-muted">{metric.detail || "Awaiting verified service data"}</span>{metric.trend && <span className="rounded-full bg-admin-green-soft px-2.5 py-1 font-bold text-admin-green">{metric.trend}</span>}</div></article>;
}

const actionTableFeatures = tableFeatures({});
const actionColumnHelper = createColumnHelper<typeof actionTableFeatures, AdminAction>();
const actionColumns = actionColumnHelper.columns([
  actionColumnHelper.accessor("action", { header: "Action", cell: (info) => <span className="font-semibold text-admin-ink">{String(info.getValue()) || "Unspecified action"}</span> }),
  actionColumnHelper.accessor("subject", { header: "Subject", cell: (info) => <span className="text-admin-muted">{String(info.getValue()) || "—"}</span> }),
  actionColumnHelper.accessor("actor", { header: "Actor", cell: (info) => <span className="text-admin-muted">{String(info.getValue()) || "—"}</span> }),
  actionColumnHelper.accessor("status", { header: "Status", cell: (info) => <span className={`inline-flex rounded-full px-2.5 py-1 text-xs font-bold capitalize ${statusClass(info.getValue())}`}>{String(info.getValue())}</span> }),
]);

function EChartSurface({ series, loading, error }: { series: AdminSnapshot["policySeries"]; loading: boolean; error: unknown }) {
  const chartRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!chartRef.current || chartRef.current.clientWidth === 0 || chartRef.current.clientHeight === 0) return;
    let cancelled = false;
    let chart: import("echarts/core").ECharts | undefined;
    let observer: ResizeObserver | undefined;
    Promise.all([import("echarts/core"), import("echarts/charts"), import("echarts/components"), import("echarts/renderers")]).then(([core, charts, components, renderers]) => {
      if (cancelled || !chartRef.current) return;
      core.use([charts.LineChart, components.GridComponent, components.TooltipComponent, renderers.CanvasRenderer]);
      chart = core.init(chartRef.current);
      const root = getComputedStyle(document.documentElement);
      const primary = root.getPropertyValue("--admin-primary").trim();
      const grid = root.getPropertyValue("--admin-line").trim();
      const muted = root.getPropertyValue("--admin-muted").trim();
      chart.setOption({ animation: false, grid: { left: 12, right: 12, top: 14, bottom: 26, containLabel: true }, tooltip: { trigger: "axis" }, xAxis: { type: "category", data: series.map((point) => point.label), axisLine: { lineStyle: { color: grid } }, axisLabel: { color: muted } }, yAxis: { type: "value", splitLine: { lineStyle: { color: grid, type: "dashed" } }, axisLabel: { color: muted } }, series: [{ type: "line", smooth: true, showSymbol: false, data: series.map((point) => point.value), lineStyle: { color: primary, width: 3 }, areaStyle: { color: `${primary}22` } }] });
      observer = new ResizeObserver(() => chart?.resize());
      observer.observe(chartRef.current);
    });
    return () => { cancelled = true; observer?.disconnect(); chart?.dispose(); };
  }, [series]);
  const empty = !loading && (!series.length || Boolean(error));
  return <div className="relative h-72" data-testid="admin-chart"><div ref={chartRef} className="h-full w-full" aria-label="Verified policy activity chart" role="img" />{empty && <div className="absolute inset-0 grid place-items-center rounded-xl bg-admin-surface/90 p-6 text-center"><div><strong className="block text-sm text-admin-ink">No verified series available</strong><p className="mt-1 max-w-xs text-xs leading-5 text-admin-muted">The chart will render after the service returns a versioned activity series.</p></div></div>}{loading && <div className="absolute inset-0 grid place-items-center rounded-xl bg-admin-surface/80 text-sm font-semibold text-admin-muted">Loading verified activity…</div>}</div>;
}

function ActionTable({ actions }: { actions: AdminAction[] }) {
  const table = useTable({ features: actionTableFeatures, data: actions, columns: actionColumns });
  return <div className="overflow-x-auto" data-testid="admin-table"><table className="w-full min-w-[42rem] text-left text-sm"><caption className="sr-only">Recent verified actions</caption><thead className="border-b border-admin-line text-xs uppercase tracking-[0.1em] text-admin-muted"><tr>{table.getHeaderGroups()[0]?.headers.map((header) => <th key={header.id} className="px-4 py-3 font-black">{header.isPlaceholder ? null : <table.FlexRender header={header} />}</th>)}</tr></thead><tbody className="divide-y divide-admin-line">{table.getRowModel().rows.map((row) => <tr key={row.id} className="hover:bg-admin-surface-muted">{row.getAllCells().map((cell) => <td key={cell.id} className="px-4 py-4"><table.FlexRender cell={cell} /></td>)}</tr>)}</tbody></table>{actions.length === 0 && <div className="px-4 py-10 text-center text-sm text-admin-muted">No verified activity is available for this session.</div>}</div>;
}

function OverviewPage() {
  const { session } = useAdminSession();
  const snapshotQuery = useQuery({ queryKey: ["admin", "overview", session?.email ?? "anonymous", session?.expiresAtUnix ?? 0], queryFn: () => fetchAdminSnapshot(session), enabled: sessionIsUsable(session), retry: false, staleTime: 30_000, refetchInterval: 30_000, refetchOnWindowFocus: true });
  const snapshot = snapshotQuery.data;
  const metrics = snapshot?.metrics ?? [
    { id: "quotes", label: "Tax quotes", value: "", detail: "Verified quote volume", tone: "violet" as const },
    { id: "rules", label: "Published policies", value: "", detail: "Current immutable versions", tone: "amber" as const },
    { id: "settlements", label: "Settlements", value: "", detail: "Finalized chain evidence", tone: "cyan" as const },
    { id: "evidence", label: "Evidence events", value: "", detail: "Append-only audit records", tone: "green" as const },
  ];
  return <><PageHeader eyebrow="Overview / control plane" title="Tax operations" description="A governed overview of authorization, evidence, and workflow state for the authenticated tenant." action={<span className="inline-flex items-center gap-2 rounded-xl border border-admin-line bg-admin-surface px-4 py-2.5 text-sm font-bold text-admin-muted shadow-soft"><span>Current policy snapshot</span><Icon name="chevron" /></span>} /><StatusBanner error={snapshotQuery.error} /><section className="mb-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-4" aria-label="Verified authorization metrics">{metrics.map((metric) => <MetricCard key={metric.id} metric={metric} />)}</section><section className="mb-6 grid gap-6 xl:grid-cols-[minmax(0,1.45fr)_minmax(18rem,.8fr)]"><article className="rounded-2xl border border-admin-line bg-admin-surface p-5 shadow-soft"><div className="flex items-start justify-between gap-4"><div><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Evidence projection</p><h2 className="mt-1 text-xl font-black tracking-tight text-admin-ink">Evidence access activity</h2></div><span className="rounded-full bg-admin-primary-soft px-3 py-1 text-xs font-bold text-admin-primary">Server-owned</span></div><EChartSurface series={snapshot?.policySeries ?? []} loading={snapshotQuery.isPending} error={snapshotQuery.error} /></article><article className="rounded-2xl border border-admin-line bg-admin-surface p-5 shadow-soft"><div className="flex items-start justify-between gap-4"><div><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Decision boundary</p><h2 className="mt-1 text-xl font-black tracking-tight text-admin-ink">Access posture</h2></div><span className="grid size-9 place-items-center rounded-xl bg-admin-primary-soft text-admin-primary"><Icon name="shield" /></span></div><div className="mt-6 space-y-4">{(["RBAC", "ABAC", "PBAC"] as const).map((control) => <div key={control} className="flex items-center justify-between gap-3 border-b border-admin-line pb-4 last:border-0 last:pb-0"><div><strong className="block text-sm text-admin-ink">{control}</strong><span className="text-xs text-admin-muted">{control === "RBAC" ? "Assigned responsibilities" : control === "ABAC" ? "Tenant and resource context" : "Policy and approval evidence"}</span></div><span className="text-xs font-black text-admin-primary">Server</span></div>)}</div></article></section><article className="rounded-2xl border border-admin-line bg-admin-surface p-5 shadow-soft"><div className="mb-4 flex items-end justify-between gap-4"><div><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Audit trail</p><h2 className="mt-1 text-xl font-black tracking-tight text-admin-ink">Recent evidence access</h2></div><Link to="/admin/evidence" className="inline-flex items-center gap-2 text-sm font-bold text-admin-primary">View evidence <Icon name="arrow" /></Link></div><ActionTable actions={snapshot?.recentActions ?? []} /></article></>;
}

function SectionPage({ section }: { section: Exclude<AdminSection, "overview"> }) {
  const { session } = useAdminSession();
  const copy = SECTION_COPY[section];
  const [nowUnix, setNowUnix] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => { const timer = window.setInterval(() => setNowUnix(Math.floor(Date.now() / 1000)), 1_000); return () => window.clearInterval(timer); }, []);
  const snapshotQuery = useQuery({ queryKey: ["admin", "overview", session?.email ?? "anonymous", session?.expiresAtUnix ?? 0], queryFn: () => fetchAdminSnapshot(session), enabled: sessionIsUsable(session), retry: false, staleTime: 30_000, refetchInterval: 30_000, refetchOnWindowFocus: true });
  const capability = snapshotQuery.data?.capabilities.find((candidate) => candidate.id === copy.capability);
  const snapshotFresh = Boolean(snapshotQuery.data && (!snapshotQuery.data.expiresAtUnix || snapshotQuery.data.expiresAtUnix > nowUnix));
  const effectiveCapability = snapshotFresh ? capability : undefined;
  const denied = effectiveCapability?.decision === "deny";
  if (section === "quotes") return <Suspense fallback={<WorkspaceLoading />}><TaxQuotesPage session={session} capability={effectiveCapability} /></Suspense>;
  if (section === "rules") return <Suspense fallback={<WorkspaceLoading />}><RuleGovernancePage session={session} capability={effectiveCapability} /></Suspense>;
  if (section === "settlements") return <Suspense fallback={<WorkspaceLoading />}><SettlementOperationsPage session={session} capability={effectiveCapability} /></Suspense>;
  if (section === "treasury") return <Suspense fallback={<WorkspaceLoading />}><TreasuryOperationsPage session={session} capability={effectiveCapability} /></Suspense>;
  if (section === "settings") return <Suspense fallback={<WorkspaceLoading />}><SettingsPage session={session} snapshot={snapshotQuery.data} capability={effectiveCapability} /></Suspense>;
  const managementCapability = snapshotQuery.data?.capabilities.find((candidate) => candidate.id === "identity.role.manage");
  return <><PageHeader eyebrow={`Admin / ${section}`} title={copy.title} description={copy.description} action={<Link to="/admin" className="inline-flex items-center gap-2 text-sm font-bold text-admin-primary"><span>Back to overview</span><Icon name="arrow" /></Link>} /><StatusBanner error={snapshotQuery.error} />{snapshotQuery.data && !snapshotFresh && <SnapshotStaleBanner />}<section className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_20rem]"><div className="space-y-6"><article className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft"><div className="flex items-start justify-between gap-4"><div><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Capability boundary</p><h2 className="mt-1 text-xl font-black text-admin-ink">{effectiveCapability?.label ?? copy.title}</h2></div><span className={`rounded-full px-3 py-1 text-xs font-bold ${denied ? "bg-admin-red-soft text-admin-red" : effectiveCapability?.decision === "allow" ? "bg-admin-green-soft text-admin-green" : "bg-admin-amber-soft text-admin-amber-ink"}`}>{denied ? "Access denied" : effectiveCapability?.decision === "allow" ? "Access confirmed" : "Decision required"}</span></div><div className="mt-8 rounded-2xl border border-dashed border-admin-line bg-admin-surface-muted p-8 text-center"><strong className="block text-admin-ink">{denied ? "This workflow is unavailable for the current responsibility." : !snapshotFresh ? "Fresh authorization is required before this workflow can continue." : "Verified workflow data is not available yet."}</strong><p className="mx-auto mt-2 max-w-lg text-sm leading-6 text-admin-muted">{denied ? "Ask an administrator to review the server-owned RBAC, ABAC, and PBAC decision. The browser cannot grant this capability." : !snapshotFresh ? "The previous capability decision has expired. Protected actions remain disabled until the service returns a fresh snapshot." : "Connect the admin service to populate this workspace. No client-side fallback or fabricated records are used."}</p>{!session && <Link to="/auth/signin" className="mt-5 inline-flex rounded-xl bg-admin-primary px-4 py-2.5 text-sm font-bold text-white">Sign in to continue</Link>}</div></article>{section !== "access" && <WorkflowPanel section={section} session={session} capability={effectiveCapability} />}{section === "access" && <AccessDirectoryPanel session={session} capability={effectiveCapability} managementCapability={managementCapability} />}{section === "evidence" && <EvidenceAuditPanel session={session} capability={effectiveCapability} />}</div><aside className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft"><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Control model</p><h2 className="mt-1 text-xl font-black text-admin-ink">RBAC · ABAC · PBAC</h2><p className="mt-3 text-sm leading-6 text-admin-muted">{copy.description}</p><ul className="mt-6 space-y-4 text-sm"><li className="flex gap-3"><span className="grid size-7 shrink-0 place-items-center rounded-lg bg-admin-primary-soft text-xs font-black text-admin-primary">R</span><span><strong className="block text-admin-ink">Role responsibility</strong><small className="text-admin-muted">Assigned by the service.</small></span></li><li className="flex gap-3"><span className="grid size-7 shrink-0 place-items-center rounded-lg bg-admin-primary-soft text-xs font-black text-admin-primary">A</span><span><strong className="block text-admin-ink">Resource attributes</strong><small className="text-admin-muted">Tenant and jurisdiction context.</small></span></li><li className="flex gap-3"><span className="grid size-7 shrink-0 place-items-center rounded-lg bg-admin-primary-soft text-xs font-black text-admin-primary">P</span><span><strong className="block text-admin-ink">Policy approval</strong><small className="text-admin-muted">Versioned and auditable.</small></span></li></ul></aside></section></>;
}

const rootRoute = createRootRoute({ component: () => <Outlet /> });
const homeRoute = createRoute({ getParentRoute: () => rootRoute, path: "/", component: () => <Navigate to="/admin" replace /> });
const adminRoute = createRoute({ getParentRoute: () => rootRoute, path: "/admin", component: AdminRouteBoundary });
const overviewRoute = createRoute({ getParentRoute: () => adminRoute, path: "/", component: OverviewPage });
const quotesRoute = createRoute({ getParentRoute: () => adminRoute, path: "/tax/quotes", component: () => <SectionPage section="quotes" /> });
const rulesRoute = createRoute({ getParentRoute: () => adminRoute, path: "/tax/rules", component: () => <SectionPage section="rules" /> });
const settlementsRoute = createRoute({ getParentRoute: () => adminRoute, path: "/settlements", component: () => <SectionPage section="settlements" /> });
const treasuryRoute = createRoute({ getParentRoute: () => adminRoute, path: "/treasury", component: () => <SectionPage section="treasury" /> });
const accessRoute = createRoute({ getParentRoute: () => adminRoute, path: "/access", component: () => <SectionPage section="access" /> });
const evidenceRoute = createRoute({ getParentRoute: () => adminRoute, path: "/evidence", component: () => <SectionPage section="evidence" /> });
const settingsRoute = createRoute({ getParentRoute: () => adminRoute, path: "/settings", component: () => <SectionPage section="settings" /> });
const signInRoute = createRoute({ getParentRoute: () => rootRoute, path: "/auth/signin", component: () => <AdminAuthPage intent="signin" /> });
const signUpRoute = createRoute({ getParentRoute: () => rootRoute, path: "/auth/signup", component: () => <AdminAuthPage intent="signup" /> });
const routeTree = rootRoute.addChildren([homeRoute, signInRoute, signUpRoute, adminRoute.addChildren([overviewRoute, quotesRoute, rulesRoute, settlementsRoute, treasuryRoute, accessRoute, evidenceRoute, settingsRoute])]);

export default function AdminApp({ initialPath }: AdminAppProps) {
  const [session, setSessionState] = useState<AdminSession | null>(null);
  const [queryClient] = useState(() => new QueryClient({ defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false } } }));
  const router = useMemo(() => createRouter({ routeTree, history: initialPath ? createMemoryHistory({ initialEntries: [initialPath] }) : createBrowserHistory() }), [initialPath]);
  const context = useMemo<SessionContextValue>(() => ({ session, setSession: setSessionState, signOut: () => setSessionState(null) }), [session]);
  return <QueryClientProvider client={queryClient}><SessionContext.Provider value={context}><RouterProvider router={router} /></SessionContext.Provider></QueryClientProvider>;
}
