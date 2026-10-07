// @vitest-environment node

import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const scripts = resolve(process.cwd(), "../../scripts");

function run(script: string) {
  const result = spawnSync("sh", [resolve(scripts, script)], { encoding: "utf8" });
  return { status: result.status, output: `${result.stdout}${result.stderr}` };
}

describe("non-custodial wallet contract (fail-closed)", () => {
  it("passes on the real source tree without any external scanner", () => {
    const result = run("verify_non_custodial_wallet_contract.sh");
    expect(result.output).not.toMatch(/command not found/);
    expect(result.status).toBe(0);
    expect(result.output).toContain("non-custodial wallet contract passed");
  });

  it("fails on injected key material and on scanner errors (regression)", () => {
    const result = run("verify_non_custodial_wallet_contract_test.sh");
    expect(result.status, result.output).toBe(0);
    expect(result.output).toContain("regression test passed");
  });
});
