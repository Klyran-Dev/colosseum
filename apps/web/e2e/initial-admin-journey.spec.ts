import { expect, test, type Page } from "@playwright/test";
import { readdir, readFile } from "node:fs/promises";

interface LocalOtpMessage {
  schema: "solana-tax-protocol/local-email-inbox/v1";
  challenge_id: string;
  recipient: string;
  code: string;
  expires_at_unix: number;
}

const INBOX_DIR = process.env.AUTH_INBOX_DIR ?? "/workspace/auth-inbox";
const INITIAL_ADMIN_EMAIL = "initial-admin@local.test";

async function readOtpFor(recipient: string, challengeId?: string): Promise<LocalOtpMessage> {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    const names = await readdir(INBOX_DIR).catch(() => []);
    for (const name of names.filter((candidate) => candidate.endsWith(".json"))) {
      const raw = await readFile(`${INBOX_DIR}/${name}`, "utf8").catch(() => "");
      try {
        const candidate = JSON.parse(raw) as Partial<LocalOtpMessage>;
        if (candidate.schema === "solana-tax-protocol/local-email-inbox/v1"
          && candidate.recipient === recipient
          && (!challengeId || candidate.challenge_id === challengeId)
          && typeof candidate.challenge_id === "string"
          && typeof candidate.code === "string"
          && /^[0-9]{6}$/u.test(candidate.code)
          && typeof candidate.expires_at_unix === "number") {
          return candidate as LocalOtpMessage;
        }
      } catch {
        // The sender may still be replacing the file; retry until the deadline.
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`No synthetic OTP email was delivered for ${recipient}.`);
}

async function completeInitialAdminOtp(page: Page): Promise<void> {
  await page.goto("/auth/signup?next=%2Fadmin%2Faccess");
  await expect(page.getByRole("heading", { name: "Create your workspace account", exact: true })).toBeVisible();
  await page.getByLabel("Work email").fill(INITIAL_ADMIN_EMAIL);
  const requestResponse = page.waitForResponse((response) => response.url().endsWith("/v1/auth/email/request"));
  await page.getByRole("button", { name: "Create account" }).click();
  expect((await requestResponse).status()).toBe(202);
  await expect(page.getByRole("status")).toContainText("Use the newest code sent to this email address.");
  const message = await readOtpFor(INITIAL_ADMIN_EMAIL);
  await page.getByLabel("One-time code").fill(message.code);
  const verifyResponse = page.waitForResponse((response) => response.url().endsWith("/v1/auth/email/verify"));
  await page.getByRole("button", { name: "Verify code" }).click();
  expect((await verifyResponse).status()).toBe(200);
  await expect(page).toHaveURL(/\/admin\/access$/u);
}

test("gives the configured bootstrap email an actionable setup outcome after sign-in OTP verification", async ({ page }) => {
  await page.goto("/auth/signin?next=%2Fadmin%2Faccess");
  await expect(page.getByRole("heading", { name: "Sign in to your workspace", exact: true })).toBeVisible();
  await page.getByLabel("Work email").fill(INITIAL_ADMIN_EMAIL);
  const requestResponse = page.waitForResponse((response) => response.url().endsWith("/v1/auth/email/request"));
  await page.getByRole("button", { name: "Continue with email" }).click();
  const requested = await requestResponse;
  expect(requested.status()).toBe(202);
  const challenge = await requested.json() as { challenge_id: string };
  const message = await readOtpFor(INITIAL_ADMIN_EMAIL, challenge.challenge_id);
  await page.getByLabel("One-time code").fill(message.code);
  const verifyResponse = page.waitForResponse((response) => response.url().endsWith("/v1/auth/email/verify"));
  await page.getByRole("button", { name: "Verify code" }).click();
  expect((await verifyResponse).status()).toBe(409);
  await expect(page.getByRole("alert")).toContainText("Your email was verified. Finish account setup to continue.");
  await expect(page.getByRole("button", { name: "Finish account setup" })).toBeVisible();
  await page.getByRole("button", { name: "Finish account setup" }).click();
  await expect(page).toHaveURL(/\/auth\/signup\?next=%2Fadmin%2Faccess$/u);
});

test("provisions the first administrator through the configured mailbox boundary", async ({ page }) => {
  await completeInitialAdminOtp(page);
  await expect(page.getByRole("heading", { level: 1, name: "Access & approvals", exact: true })).toBeVisible();
  await expect(page.getByText("Access confirmed", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
});

