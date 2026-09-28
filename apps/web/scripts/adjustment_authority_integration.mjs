import crypto from "node:crypto";
import { Keypair } from "@solana/web3.js";

const apiBase = process.env.API_BASE;
const proposerAuthorization = process.env.PROPOSER_AUTHORIZATION;
const approverAuthorization = process.env.APPROVER_AUTHORIZATION;
if (!apiBase || !proposerAuthorization || !approverAuthorization) {
  throw new Error("API_BASE, PROPOSER_AUTHORIZATION, and APPROVER_AUTHORIZATION are required");
}

const seeds = {
  proposer: Buffer.from("01".repeat(32), "hex"),
  approver: Buffer.from("02".repeat(32), "hex"),
};

function signEd25519(seed, message) {
  const pkcs8Prefix = Buffer.from("302e020100300506032b657004220420", "hex");
  const privateKey = crypto.createPrivateKey({
    key: Buffer.concat([pkcs8Prefix, seed]),
    format: "der",
    type: "pkcs8",
  });
  return crypto.sign(null, Buffer.from(message, "utf8"), privateKey);
}

async function jsonRequest(path, authorization, body) {
  const response = await fetch(`${apiBase}${path}`, {
    method: "POST",
    headers: { "content-type": "application/json", authorization },
    body: JSON.stringify(body),
  });
  const payload = await response.json();
  if (!response.ok) {
    throw new Error(`${path} returned HTTP ${response.status}: ${JSON.stringify(payload)}`);
  }
  return payload;
}

async function bind(role, authorization) {
  const challenge = await jsonRequest(
    "/v1/settlement/adjustment-authority/challenges",
    authorization,
    { role },
  );
  const keypair = Keypair.fromSeed(seeds[role]);
  const signature = signEd25519(seeds[role], challenge.message).toString("base64url");
  const binding = await jsonRequest(
    "/v1/settlement/adjustment-authority/bindings",
    authorization,
    {
      role,
      challenge_id: challenge.challenge_id,
      public_key: keypair.publicKey.toBase58(),
      signature,
    },
  );
  if (binding.public_key !== keypair.publicKey.toBase58() || binding.role !== role) {
    throw new Error(`wallet binding response was not server-owned for ${role}`);
  }
  return binding;
}

const proposer = await bind("proposer", proposerAuthorization);
const approver = await bind("approver", approverAuthorization);
console.log(JSON.stringify({
  status: "wallet_proofs_bound",
  proposer_public_key: proposer.public_key,
  approver_public_key: approver.public_key,
}));
