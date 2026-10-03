import { ComputeBudgetProgram, Connection, Keypair, PublicKey, Transaction, TransactionInstruction } from "@solana/web3.js";

export async function compileAndSign({ apiBase, rpcUrl, authorization, calculationId, routeId, nonce }) {
  if (!Number.isSafeInteger(nonce) || nonce <= 0) {
    throw new Error("NONCE must be a positive safe integer");
  }

  const payer = Keypair.generate();
  const compileResponse = await request(`${apiBase}/v1/settlement/compile-from-quote`, {
    method: "POST",
    headers: { "content-type": "application/json", authorization },
    body: JSON.stringify({
      calculation_id: calculationId,
      route_id: routeId,
      payer: payer.publicKey.toBase58(),
      nonce: String(nonce),
    }),
  });
  const artifact = compileResponse.payload;
  if (compileResponse.status !== 200 || artifact.status !== "prepared_unsubmitted" || typeof artifact.intent_id !== "string") {
    throw new Error(`settlement compile did not return a durable prepared intent: HTTP ${compileResponse.status} ${JSON.stringify(artifact)}`);
  }
  if (!Array.isArray(artifact.accounts) || artifact.accounts.length === 0 || typeof artifact.instruction_data_base64 !== "string") {
    throw new Error("settlement compile returned an incomplete instruction artifact");
  }

  const connection = new Connection(rpcUrl, "confirmed");
  const { blockhash } = await connection.getLatestBlockhash("confirmed");
  if (!artifact.compute_budget_policy
      || artifact.compute_budget_policy.policy_version !== 1
      || !Number.isSafeInteger(artifact.compute_budget_policy.compute_unit_limit)
      || !Number.isSafeInteger(artifact.compute_budget_policy.micro_lamports_per_compute_unit)) {
    throw new Error("settlement compile returned an invalid compute-budget policy");
  }
  const transaction = new Transaction({
    feePayer: payer.publicKey,
    recentBlockhash: blockhash,
  }).add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: artifact.compute_budget_policy.compute_unit_limit }),
    ComputeBudgetProgram.setComputeUnitPrice({
      microLamports: artifact.compute_budget_policy.micro_lamports_per_compute_unit,
    }),
    new TransactionInstruction({
      programId: new PublicKey(artifact.program_id),
      data: Buffer.from(artifact.instruction_data_base64, "base64"),
      keys: artifact.accounts.map((account) => ({
        pubkey: new PublicKey(account.pubkey),
        isSigner: account.is_signer,
        isWritable: account.is_writable,
      })),
    }),
  );
  transaction.sign(payer);
  const signedTransactionBase64 = transaction.serialize({
    requireAllSignatures: true,
    verifySignatures: true,
  }).toString("base64");
  const submitBody = JSON.stringify({
    intent_id: artifact.intent_id,
    signed_transaction_base64: signedTransactionBase64,
    instruction: artifact,
  });
  return { artifact, submitBody };
}

export async function request(url, options = {}) {
  const response = await fetch(url, options);
  const payload = await response.json().catch(() => null);
  return { status: response.status, payload };
}

export async function sendCount(rpcUrl) {
  const response = await fetch(`${rpcUrl}/admin/send-count`);
  if (!response.ok) throw new Error(`RPC fixture send-count failed with HTTP ${response.status}`);
  return response.json();
}

export function required(name) {
  const value = process.env[name];
  if (!value || !value.trim()) throw new Error(`${name} is required`);
  return value;
}
