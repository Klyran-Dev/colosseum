// @vitest-environment node

import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterAll, describe, expect, it } from "vitest";

const script = resolve(process.cwd(), "../../scripts/verify_web_audit_contract.mjs");
const root = mkdtempSync(join(tmpdir(), "audit-contract-"));
afterAll(() => rmSync(root, { recursive: true, force: true }));

function advisory(id: string, module: string, severity: string, ghsa: string) {
  return { id, module_name: module, severity, github_advisory_id: ghsa, title: `${module} issue`, vulnerable_versions: "*", patched_versions: "none" };
}
function run(name: string, advisories: unknown[], exceptions: unknown[], extra: string[] = []) {
  const auditFile = join(root, `${name}-audit.json`);
  const exceptionFile = join(root, `${name}-exceptions.json`);
  writeFileSync(auditFile, JSON.stringify({ advisories: Object.fromEntries((advisories as Array<{ id: string }>).map((entry) => [entry.id, entry])) }));
  writeFileSync(exceptionFile, JSON.stringify({ exceptions }));
  const result = spawnSync("node", [script, "--audit-json", auditFile, "--exceptions", exceptionFile, "--today", "2026-10-07", ...extra], { encoding: "utf8" });
  return { status: result.status, output: `${result.stdout}${result.stderr}` };
}
const valid = { ghsa: "GHSA-aaaa-bbbb-cccc", package: "bigint-buffer", severity: "high", reason: "dev-only transitive; no patched release", owner: "security", expires: "2027-01-05", reachableInShippedBundle: false, approvedBy: "j.doe (security lead)" };

describe("web dependency audit contract (AGENTS.md: fail on high/critical unless excepted)", () => {
  it("passes when there are no high or critical findings", () => {
    const result = run("none", [advisory("1", "uuid", "moderate", "GHSA-1")], []);
    expect(result.status).toBe(0);
    expect(result.output).toContain("audit contract passed");
  });

  it("passes for a high finding covered by an unexpired, complete exception", () => {
    expect(run("ok", [advisory("2", "bigint-buffer", "high", valid.ghsa)], [valid]).status).toBe(0);
  });

  it("fails for a high or critical finding with no exception", () => {
    const high = run("high", [advisory("3", "evil", "high", "GHSA-zzzz")], [valid]);
    expect(high.status).toBe(1);
    expect(high.output).toContain("no approved exception");
    expect(run("critical", [advisory("4", "worse", "critical", "GHSA-yyyy")], []).status).toBe(1);
  });

  it("fails when the exception has expired", () => {
    const result = run("expired", [advisory("5", "bigint-buffer", "high", valid.ghsa)], [{ ...valid, expires: "2026-10-01" }]);
    expect(result.status).toBe(1);
    expect(result.output).toContain("expired");
  });

  it("fails when an exception is stale (the advisory no longer applies) so it gets removed", () => {
    const result = run("stale", [], [valid]);
    expect(result.status).toBe(1);
    expect(result.output).toContain("stale exception");
  });

  it("rejects incomplete exceptions and ones claiming exposure in the shipped bundle", () => {
    expect(run("incomplete", [advisory("6", "bigint-buffer", "high", valid.ghsa)], [{ ...valid, owner: "" }]).status).toBe(1);
    expect(run("shipped", [advisory("7", "bigint-buffer", "high", valid.ghsa)], [{ ...valid, reachableInShippedBundle: true }]).status).toBe(1);
    expect(run("far", [advisory("8", "bigint-buffer", "high", valid.ghsa)], [{ ...valid, expires: "2028-12-31" }]).output).toContain("longer than 180 days");
  });

  it("treats an unapproved proposal as failing, and reports it as a warning only with --allow-pending", () => {
    const proposal = { ...valid, approvedBy: "" };
    const strict = run("pending", [advisory("10", "bigint-buffer", "high", valid.ghsa)], [proposal]);
    expect(strict.status).toBe(1);
    expect(strict.output).toContain("proposed but not approved");
    const lenient = run("pending2", [advisory("11", "bigint-buffer", "high", valid.ghsa)], [proposal], ["--allow-pending"]);
    expect(lenient.status).toBe(0);
    expect(lenient.output).toContain("warning");
  });

  it("fails closed when the audit output cannot be read", () => {
    const bad = join(root, "bad.json");
    writeFileSync(bad, "not json");
    const result = spawnSync("node", [script, "--audit-json", bad, "--exceptions", join(root, "ok-exceptions.json"), "--today", "2026-10-07"], { encoding: "utf8" });
    expect(result.status).toBe(1);
    expect(`${result.stdout}${result.stderr}`).toContain("could not read the audit result");
  });

  it("fails if a package with an exception is found in the shipped bundle", () => {
    const dist = join(root, "dist", "assets");
    spawnSync("mkdir", ["-p", dist]);
    writeFileSync(join(dist, "index.js"), "/* bigint-buffer */ const x = 1;");
    const result = run("bundle", [advisory("9", "bigint-buffer", "high", valid.ghsa)], [valid], ["--dist", join(root, "dist")]);
    expect(result.status).toBe(1);
    expect(result.output).toContain("shipped bundle");
  });
});
