import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const repositoryRoot = resolve(new URL("..", import.meta.url).pathname);
const dashboardPath = process.env.GRAFANA_DASHBOARD_PATH ?? resolve(repositoryRoot, "ops/observability/grafana/dashboards/service-overview.json");

function fail(message) {
  console.error(`Grafana dashboard contract failed: ${message}`);
  process.exit(1);
}

let dashboard;
try {
  dashboard = JSON.parse(await readFile(dashboardPath, "utf8"));
} catch (error) {
  fail(`dashboard is not valid JSON: ${error instanceof Error ? error.message : "unknown error"}`);
}

if (dashboard.editable !== false) fail("dashboard must be provisioned read-only");
if (dashboard.title !== "Solana Tax Protocol / Service Overview") fail("dashboard title drifted");
if (!Array.isArray(dashboard.templating?.list) || dashboard.templating.list.length !== 0) {
  fail("dashboard variables must remain empty to prevent unbounded label queries");
}

const panels = Array.isArray(dashboard.panels) ? dashboard.panels : [];
const titles = new Set(panels.map((panel) => panel.title));
const requiredPanels = [
  ["Request rate", "stp_requests_total"],
  ["Error rate", "stp_errors_total"],
  ["Estimated p95 request latency", "stp_request_duration_ms_bucket"],
  ["Requests in last hour", "stp_requests_total"],
  ["Protocol pause state", "stp_settlement_protocol_paused"],
  ["Replay backlog", "stp_indexer_replay_gaps"],
  ["Database pool pressure", "stp_database_pool_waiting"],
  ["Tax rule cache hit ratio", "stp_tax_rule_cache_hits_total"],
  ["Settlement provider health", "stp_settlement_provider_health_score"],
];
for (const [title, metric] of requiredPanels) {
  if (!titles.has(title)) fail(`missing required panel: ${title}`);
  const panel = panels.find((candidate) => candidate.title === title);
  const expressions = (panel?.targets ?? []).map((target) => target.expr).filter((expr) => typeof expr === "string");
  if (!expressions.some((expression) => expression.includes(metric))) {
    fail(`panel ${title} does not use the required fixed metric ${metric}`);
  }
}

const serialized = JSON.stringify(dashboard).toLowerCase();
for (const forbidden of ["tenant_id", "merchant_id", "wallet", "signature", "correlation_id"]) {
  if (serialized.includes(forbidden)) fail(`dashboard contains a prohibited sensitive label or identifier: ${forbidden}`);
}

console.log(`Grafana dashboard contract passed (${requiredPanels.length} bounded operational panels)`);
