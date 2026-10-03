import { compileAndSign, request, required, sendCount } from "./settlement_drill_support.mjs";

const apiBase = required("API_BASE");
const secondaryApiBase = process.env.API_BASE_SECONDARY ?? apiBase;
const rpcUrl = required("RPC_FIXTURE_URL");
const authorization = required("AUTHORIZATION");
const calculationId = required("CALCULATION_ID");
const routeId = required("ROUTE_ID");
const scenario = process.env.DRILL_SCENARIO ?? "concurrent";
const nonce = Number(process.env.NONCE ?? "12");

if (!Number.isSafeInteger(nonce) || nonce <= 0) {
  throw new Error("NONCE must be a positive safe integer");
}
if (scenario !== "concurrent" && scenario !== "ambiguous-reconcile") {
  throw new Error(`unsupported DRILL_SCENARIO: ${scenario}`);
}

const { artifact, submitBody } = await compileAndSign({ apiBase, rpcUrl, authorization, calculationId, routeId, nonce });

if (scenario === "concurrent") {
  const [first, second] = await Promise.all([
    request(`${secondaryApiBase}/v1/settlement/submit`, {
      method: "POST",
      headers: { "content-type": "application/json", authorization },
      body: submitBody,
    }),
    request(`${apiBase}/v1/settlement/submit`, {
      method: "POST",
      headers: { "content-type": "application/json", authorization },
      body: submitBody,
    }),
  ]);
  const statuses = [first, second].map((response) => response.payload?.status ?? null);
  const statusCodes = [first, second].map((response) => response.status);
  if (statusCodes.filter((status) => status === 200).length !== 1 || statusCodes.filter((status) => status === 409).length !== 1) {
    throw new Error(`expected one 200 and one 409 submission response, received ${statusCodes.join(",")}: ${JSON.stringify([first.payload, second.payload])}`);
  }
  if (statuses.filter((status) => status === "submitted").length !== 1) {
    throw new Error(`expected exactly one submitted response, received ${statuses.join(",")}`);
  }

  const countPayload = await sendCount(rpcUrl);
  if (countPayload.send_transaction_calls !== 1) {
    throw new Error(`expected exactly one sendTransaction call, received ${countPayload.send_transaction_calls}`);
  }

  const intentResponse = await request(`${apiBase}/v1/settlement/intents/${artifact.intent_id}`, {
    headers: { authorization },
  });
  if (intentResponse.status !== 200 || intentResponse.payload.status !== "submitted" || typeof intentResponse.payload.signature !== "string") {
    throw new Error(`durable intent did not converge to submitted with a native signature: HTTP ${intentResponse.status} ${JSON.stringify(intentResponse.payload)}`);
  }

  console.log(JSON.stringify({
    scenario,
    replicas: secondaryApiBase === apiBase ? 1 : 2,
    intent_id: artifact.intent_id,
    status_codes: statusCodes,
    statuses,
    send_transaction_calls: countPayload.send_transaction_calls,
    durable_status: intentResponse.payload.status,
    signature: intentResponse.payload.signature,
  }));
} else {
  const initialCount = (await sendCount(rpcUrl)).send_transaction_calls;
  const controlResponse = await fetch(`${rpcUrl}/admin/ambiguous`, { method: "POST" });
  if (controlResponse.status !== 204) {
    throw new Error(`RPC fixture ambiguous mode failed with HTTP ${controlResponse.status}`);
  }
  const submission = await request(`${apiBase}/v1/settlement/submit`, {
    method: "POST",
    headers: { "content-type": "application/json", authorization },
    body: submitBody,
  });
  if (submission.status !== 503 || submission.payload?.error?.code !== "SETTLEMENT_SUBMISSION_UNKNOWN") {
    throw new Error(`ambiguous submission did not fail closed: HTTP ${submission.status} ${JSON.stringify(submission.payload)}`);
  }

  const unknownIntent = await request(`${apiBase}/v1/settlement/intents/${artifact.intent_id}`, {
    headers: { authorization },
  });
  if (unknownIntent.status !== 200 || unknownIntent.payload.status !== "unknown" || typeof unknownIntent.payload.signature !== "string") {
    throw new Error(`ambiguous submission did not persist unknown signature state: HTTP ${unknownIntent.status} ${JSON.stringify(unknownIntent.payload)}`);
  }

  const reconciled = await request(`${apiBase}/v1/settlement/intents/${artifact.intent_id}/reconcile`, {
    method: "POST",
    headers: { authorization },
  });
  if (reconciled.status !== 200 || reconciled.payload.status !== "confirmed" || reconciled.payload.signature !== unknownIntent.payload.signature) {
    throw new Error(`signature reconciliation did not advance the exact intent: HTTP ${reconciled.status} ${JSON.stringify(reconciled.payload)}`);
  }

  const countPayload = await sendCount(rpcUrl);
  if (countPayload.send_transaction_calls !== initialCount + 1) {
    throw new Error(`reconciliation resubmitted an ambiguous transaction: expected ${initialCount + 1}, received ${countPayload.send_transaction_calls}`);
  }
  console.log(JSON.stringify({
    scenario,
    intent_id: artifact.intent_id,
    submission_status: submission.status,
    durable_status_before_reconcile: unknownIntent.payload.status,
    reconciled_status: reconciled.payload.status,
    send_transaction_calls: countPayload.send_transaction_calls,
    signature: reconciled.payload.signature,
  }));
}
