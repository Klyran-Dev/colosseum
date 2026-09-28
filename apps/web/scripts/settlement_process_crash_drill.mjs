import { execFileSync } from "node:child_process";
import { compileAndSign, request, required, sendCount } from "./settlement_drill_support.mjs";

const apiBase = required("API_BASE");
const rpcUrl = required("RPC_FIXTURE_URL");
const authorization = required("AUTHORIZATION");
const calculationId = required("CALCULATION_ID");
const routeId = required("ROUTE_ID");
const composeProject = required("COMPOSE_PROJECT");
const policyDigest = JSON.parse(required("POLICY_DIGEST_JSON"));
const nonce = Number(process.env.NONCE ?? "14");

const { artifact, submitBody } = await compileAndSign({
  apiBase,
  rpcUrl,
  authorization,
  calculationId,
  routeId,
  policyDigest,
  nonce,
});

const holdResponse = await fetch(`${rpcUrl}/admin/hold`, { method: "POST" });
if (holdResponse.status !== 204) {
  throw new Error(`RPC fixture hold control failed with HTTP ${holdResponse.status}`);
}
const initialCount = (await sendCount(rpcUrl)).send_transaction_calls;
const submissionPromise = request(`${apiBase}/v1/settlement/submit`, {
  method: "POST",
  headers: { "content-type": "application/json", authorization },
  body: submitBody,
}).catch((error) => ({ status: 0, payload: { error: String(error) } }));

let submittingIntent;
await waitUntil(async () => {
  const count = await sendCount(rpcUrl);
  const intent = await request(`${apiBase}/v1/settlement/intents/${artifact.intent_id}`, {
    headers: { authorization },
  });
  if (count.send_transaction_calls === initialCount + 1 && intent.status === 200 && intent.payload.status === "submitting" && typeof intent.payload.signature === "string") {
    submittingIntent = intent.payload;
    return true;
  }
  return false;
}, "durable submitting claim before process interruption");

execFileSync("docker", ["compose", "-p", composeProject, "kill", "-s", "SIGKILL", "api"], { stdio: "inherit" });
const releaseResponse = await fetch(`${rpcUrl}/admin/release`, { method: "POST" });
if (releaseResponse.status !== 204) {
  throw new Error(`RPC fixture release control failed with HTTP ${releaseResponse.status}`);
}
await submissionPromise;

execFileSync("docker", ["compose", "-p", composeProject, "up", "-d", "api"], { stdio: "inherit" });
await waitUntil(async () => {
  try {
    const response = await fetch(`${apiBase}/readyz`);
    return response.ok;
  } catch {
    // The first socket after a container restart may be reset while the
    // application listener is being replaced. Retry within waitUntil's bound.
    return false;
  }
}, "API recovery after SIGKILL");

let recovered;
await waitUntil(async () => {
  try {
    const candidate = await request(`${apiBase}/v1/settlement/intents/${artifact.intent_id}`, {
      headers: { authorization },
    });
    if (candidate.status === 200 && candidate.payload.status === "submitting" && candidate.payload.signature === submittingIntent.signature && !("signed_transaction_base64" in candidate.payload)) {
      recovered = candidate;
      return true;
    }
  } catch {
    // A restarted listener can accept readiness before the first application
    // socket is stable. Retry within the bounded recovery window.
  }
  return false;
}, "durable intent readback after API recovery");

let reconciled;
await waitUntil(async () => {
  try {
    const candidate = await request(`${apiBase}/v1/settlement/intents/${artifact.intent_id}/reconcile`, {
      method: "POST",
      headers: { authorization },
    });
    if (candidate.status === 200 && candidate.payload.status === "confirmed" && candidate.payload.signature === submittingIntent.signature) {
      reconciled = candidate;
      return true;
    }
  } catch {
    // Keep retrying only within the bounded post-crash reconciliation window.
  }
  return false;
}, "exact-signature reconciliation after API recovery");

const finalCount = await sendCount(rpcUrl);
if (finalCount.send_transaction_calls !== initialCount + 1) {
  throw new Error(`process recovery resubmitted the transaction: expected ${initialCount + 1}, received ${finalCount.send_transaction_calls}`);
}

console.log(JSON.stringify({
  scenario: "process-crash-reconcile",
  intent_id: artifact.intent_id,
  status_before_crash: submittingIntent.status,
  recovered_status: recovered.payload.status,
  reconciled_status: reconciled.payload.status,
  send_transaction_calls: finalCount.send_transaction_calls,
  signature: reconciled.payload.signature,
}));

async function waitUntil(predicate, description) {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    if (await predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`timed out waiting for ${description}`);
}
