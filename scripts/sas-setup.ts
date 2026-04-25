/**
 * One-time setup script: Create Entros credential + schema on Solana devnet
 * for the Solana Attestation Service (SAS) integration.
 *
 * Run: cd executor-node/scripts && npm install && npm run setup
 *
 * Prerequisites:
 *   - Relayer keypair at ../relayer-keypair.json (or RELAYER_KEYPAIR_PATH env var)
 *   - Devnet SOL in the relayer account (use `solana airdrop 2`)
 *
 * Output: Credential PDA and Schema PDA to add to executor .env
 */

import { readFileSync } from "fs";
import { resolve } from "path";
import {
  deriveCredentialPda,
  deriveSchemaPda,
  getCreateCredentialInstruction,
  getCreateSchemaInstruction,
  SOLANA_ATTESTATION_SERVICE_PROGRAM_ADDRESS,
} from "sas-lib";
import {
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createSignerFromKeyPair,
  createKeyPairFromBytes,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
  getSignatureFromTransaction,
  type KeyPairSigner,
} from "@solana/kit";

const DEVNET_RPC = "https://api.devnet.solana.com";
const DEVNET_WS = "wss://api.devnet.solana.com";

async function loadKeypairSigner(): Promise<KeyPairSigner> {
  const keypairPath = process.env.RELAYER_KEYPAIR_PATH ||
    resolve(import.meta.dirname, "..", "relayer-keypair.json");

  const raw = readFileSync(keypairPath, "utf-8");
  const secretKey = new Uint8Array(JSON.parse(raw));

  const keypair = await createKeyPairFromBytes(secretKey);
  return await createSignerFromKeyPair(keypair);
}

async function main() {
  console.log("Loading relayer keypair...");
  const authority = await loadKeypairSigner();
  console.log(`Authority: ${authority.address}`);

  const rpc = createSolanaRpc(DEVNET_RPC);
  const rpcSubscriptions = createSolanaRpcSubscriptions(DEVNET_WS);
  const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });

  // Check balance
  const balance = await rpc.getBalance(authority.address).send();
  console.log(`Balance: ${Number(balance.value) / 1e9} SOL`);

  if (Number(balance.value) < 0.05e9) {
    console.error("Insufficient balance. Run: solana airdrop 2");
    process.exit(1);
  }

  // Helper: build, sign, send, confirm a transaction
  async function submitTx(
    instructions: ReturnType<typeof getCreateCredentialInstruction>[],
  ): Promise<string> {
    const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

    const txMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (msg) => setTransactionMessageFeePayerSigner(authority, msg),
      (msg) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, msg),
      (msg) => appendTransactionMessageInstructions(instructions, msg),
    );

    const signedTx = await signTransactionMessageWithSigners(txMessage);
    await sendAndConfirm(signedTx, { commitment: "confirmed" });
    return getSignatureFromTransaction(signedTx);
  }

  // 1. Create Entros Credential
  console.log("\n--- Creating Entros Credential ---");
  const credentialName = "entros-protocol";
  const [credentialPda] = await deriveCredentialPda({
    authority: authority.address,
    name: credentialName,
  });
  console.log(`Credential PDA: ${credentialPda}`);

  const credentialAccount = await rpc.getAccountInfo(credentialPda, { encoding: "base64" }).send();
  if (credentialAccount.value) {
    console.log("Credential already exists, skipping creation.");
  } else {
    const sig = await submitTx([
      getCreateCredentialInstruction({
        payer: authority,
        credential: credentialPda,
        authority: authority,
        name: credentialName,
        signers: [authority.address],
      }),
    ]);
    console.log(`Credential created: ${sig}`);
  }

  // 2. Create Entros Schema
  console.log("\n--- Creating Entros Schema ---");
  const schemaName = "iam-humanity-v2";
  const schemaVersion = 1;
  const [schemaPda] = await deriveSchemaPda({
    credential: credentialPda,
    name: schemaName,
    version: schemaVersion,
  });
  console.log(`Schema PDA: ${schemaPda}`);

  const schemaAccount = await rpc.getAccountInfo(schemaPda, { encoding: "base64" }).send();
  if (schemaAccount.value) {
    console.log("Schema already exists, skipping creation.");
  } else {
    const sig = await submitTx([
      getCreateSchemaInstruction({
        authority: authority,
        payer: authority,
        name: schemaName,
        credential: credentialPda,
        description: "Entros Protocol Proof-of-Personhood attestation",
        fieldNames: ["isHuman", "trustScore", "verifiedAt", "mode"],
        schema: schemaPda,
        layout: Buffer.from([10, 1, 8, 12]), // Bool=10, U16=1, I64=8, String=12
      }),
    ]);
    console.log(`Schema created: ${sig}`);
  }

  // 3. Output for .env
  console.log("\n=== Add these to executor-node .env ===");
  console.log(`SAS_CREDENTIAL_PDA=${credentialPda}`);
  console.log(`SAS_SCHEMA_PDA=${schemaPda}`);
  console.log(`SAS_ATTESTATION_TTL_DAYS=30`);
  console.log(`SAS_PROGRAM_ID=${SOLANA_ATTESTATION_SERVICE_PROGRAM_ADDRESS}`);
}

main().catch((err) => {
  console.error("Setup failed:", err);
  process.exit(1);
});
