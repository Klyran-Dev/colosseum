#!/usr/bin/env node
// Dependency audit contract for apps/web (ADR-0440). AGENTS.md: CI fails on
// critical/high findings unless a time-bounded exception is approved.
//   node scripts/verify_web_audit_contract.mjs [--exceptions file] [--audit-json file]
//        [--dist dir] [--today YYYY-MM-DD] [--allow-pending]
// Fails closed: an unreadable audit result, a high/critical finding without an
// exception, an expired / incomplete / over-long exception, a stale exception (the
// advisory no longer applies), or an excepted package found in the shipped bundle.
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = resolve(new URL("..", import.meta.url).pathname);
const args = process.argv.slice(2);
const option = (name, fallback) => (args.includes(name) ? args[args.indexOf(name) + 1] : fallback);
const exceptionsPath = option("--exceptions", join(repoRoot, "docs/security/dependency-exceptions.json"));
const auditJsonPath = option("--audit-json", null);
const distDirectory = option("--dist", existsSync(join(repoRoot, "apps/web/dist")) ? join(repoRoot, "apps/web/dist") : null);
const today = new Date(`${option("--today", new Date().toISOString().slice(0, 10))}T00:00:00Z`);
const MAX_EXCEPTION_DAYS = 180;
const allowPending = args.includes("--allow-pending");
const problems = [];
const fail = (message) => problems.push(message);
const warnings = [];

function readAudit() {
  let text;
  if (auditJsonPath) text = readFileSync(auditJsonPath, "utf8");
  else {
    const result = spawnSync("pnpm", ["--dir", join(repoRoot, "apps/web"), "audit", "--json"], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
    text = result.stdout; // pnpm audit exits non-zero when it finds vulnerabilities; the JSON is still valid.
  }
  try {
    const parsed = JSON.parse(text);
    if (typeof parsed !== "object" || parsed === null || (!parsed.advisories && !parsed.metadata)) throw new Error("unexpected shape");
    return Object.values(parsed.advisories ?? {});
  } catch (error) {
    fail(`could not read the audit result (${error.message}); failing closed`);
    return [];
  }
}

let exceptions = [];
try {
  exceptions = JSON.parse(readFileSync(exceptionsPath, "utf8")).exceptions ?? [];
} catch (error) {
  fail(`could not read the exceptions file ${exceptionsPath}: ${error.message}`);
}

const advisories = readAudit();
const serious = advisories.filter((advisory) => ["high", "critical"].includes(advisory.severity));
const byGhsa = new Map(exceptions.map((entry) => [entry.ghsa, entry]));

for (const entry of exceptions) {
  const label = `exception ${entry.ghsa ?? "(no id)"} for ${entry.package ?? "(no package)"}`;
  for (const field of ["ghsa", "package", "severity", "reason", "owner", "expires"]) {
    if (typeof entry[field] !== "string" || entry[field].trim() === "") fail(`${label} is incomplete: missing ${field}`);
  }
  // An exception only counts once a named human has approved it; a proposal is reported.
  if (typeof entry.approvedBy !== "string" || entry.approvedBy.trim() === "") {
    const message = `${label} is proposed but not approved: set approvedBy to the accountable owner`;
    if (allowPending) warnings.push(message); else fail(message);
  }
  if (entry.reachableInShippedBundle !== false) fail(`${label} must state reachableInShippedBundle: false (an exception cannot cover shipped runtime code)`);
  const expires = new Date(`${entry.expires}T00:00:00Z`);
  if (Number.isNaN(expires.getTime())) fail(`${label} has an invalid expiry date`);
  else {
    if (expires < today) fail(`${label} expired on ${entry.expires}; fix the dependency or renew with a new review`);
    if ((expires - today) / 86_400_000 > MAX_EXCEPTION_DAYS) fail(`${label} expires ${entry.expires}, longer than ${MAX_EXCEPTION_DAYS} days from today; exceptions must be time-bounded`);
  }
}

for (const advisory of serious) {
  if (!byGhsa.has(advisory.github_advisory_id)) {
    fail(`${advisory.severity} finding ${advisory.github_advisory_id} in ${advisory.module_name} has no approved exception`);
  }
}
const reported = new Set(advisories.map((advisory) => advisory.github_advisory_id));
for (const entry of exceptions) {
  if (!reported.has(entry.ghsa)) fail(`stale exception ${entry.ghsa} for ${entry.package}: the advisory is no longer reported; remove it`);
}

function bundleFiles(directory) {
  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    return statSync(path).isDirectory() ? bundleFiles(path) : /\.(js|css|html)$/.test(name) ? [path] : [];
  });
}
if (distDirectory && existsSync(distDirectory)) {
  const files = bundleFiles(distDirectory).map((path) => ({ path, text: readFileSync(path, "utf8") }));
  for (const entry of exceptions) {
    const hit = files.find((file) => file.text.includes(entry.package));
    if (hit) fail(`excepted package ${entry.package} appears in the shipped bundle (${hit.path.replace(distDirectory, "dist")}); the exception does not apply`);
  }
}

warnings.forEach((warning) => console.warn(`web audit contract warning: ${warning}`));
if (problems.length) {
  problems.forEach((problem) => console.error(`web audit contract failed: ${problem}`));
  process.exit(1);
}
console.log(`web audit contract passed (${advisories.length} advisories, ${serious.length} high/critical, ${exceptions.filter((entry) => typeof entry.approvedBy === "string" && entry.approvedBy.trim() !== "").length} approved and ${exceptions.length} recorded exceptions)`);
