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
const SYNTHETIC_DOMAIN = "auth-browser.example.test";

function syntheticEmail(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}@${SYNTHETIC_DOMAIN}`;
}

async function readOtpFor(recipient: string): Promise<LocalOtpMessage> {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    const names = await readdir(INBOX_DIR).catch(() => []);
    let latest: LocalOtpMessage | undefined;
    for (const name of names.filter((candidate) => candidate.endsWith(".json"))) {
      const raw = await readFile(`${INBOX_DIR}/${name}`, "utf8").catch(() => "");
      try {
        const candidate = JSON.parse(raw) as Partial<LocalOtpMessage>;
        if (candidate.schema === "solana-tax-protocol/local-email-inbox/v1"
          && candidate.recipient === recipient
          && typeof candidate.challenge_id === "string"
          && typeof candidate.code === "string"
          && /^[0-9]{6}$/u.test(candidate.code)
          && typeof candidate.expires_at_unix === "number") {
          const message = candidate as LocalOtpMessage;
          if (!latest || message.challenge_id > latest.challenge_id) latest = message;
        }
      } catch {
        // The sender may still be replacing the file; retry until the deadline.
      }
    }
    if (latest) return latest;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`No synthetic OTP email was delivered for ${recipient}.`);
}

async function completeOtp(page: Page, intent: "signin" | "signup", email: string, nextPath: string): Promise<void> {
  await page.goto(`/auth/${intent}?next=${encodeURIComponent(nextPath)}`);
  await expect(page.getByRole("heading", {
    name: intent === "signup" ? "Create your operator account" : "Sign in to Tax operations",
    exact: true,
  })).toBeVisible();
  await expect(page.getByLabel("Work email")).toBeVisible();
  await page.getByLabel("Work email").fill(email);
  const requestResponse = page.waitForResponse((response) => response.url().endsWith("/v1/auth/email/request"));
  await page.getByRole("button", { name: intent === "signup" ? "Create account" : "Continue with email" }).click();
  expect((await requestResponse).status()).toBe(202);
  await expect(page.getByRole("status")).toContainText("Use the newest code sent to this email address.");

  const message = await readOtpFor(email);
  expect(message.expires_at_unix).toBeGreaterThan(Math.floor(Date.now() / 1000));
  await page.getByLabel("One-time code").fill(message.code);
  const verifyResponse = page.waitForResponse((response) => response.url().endsWith("/v1/auth/email/verify"));
  await page.getByRole("button", { name: "Verify code" }).click();
  expect((await verifyResponse).status()).toBe(200);
  await expect(page).toHaveURL(new RegExp(`${nextPath.replaceAll("/", "\\/")}$`));
}

async function signOut(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page).toHaveURL(/\/auth\/signin\?next=%2Fadmin$/u);
  await expect(page.getByRole("heading", { name: "Sign in to Tax operations", exact: true })).toBeVisible();
}

test.describe("Docker-backed email OTP authentication", () => {
  test("signs up an operator, signs out, then signs in again", async ({ page }) => {
    const email = syntheticEmail("operator");
    await completeOtp(page, "signup", email, "/admin");
    await expect(page.getByRole("heading", { name: "Tax operations", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
    await signOut(page);
    await completeOtp(page, "signin", email, "/admin/tax/quotes");
    await expect(page.getByRole("heading", { name: "Prepare a verified quote", exact: true })).toBeVisible();
  });

  test("resends an OTP through the server challenge boundary", async ({ page }) => {
    const email = syntheticEmail("resend");
    await page.goto("/auth/signup?next=%2Fadmin");
    await page.getByLabel("Work email").fill(email);
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByRole("status")).toContainText("Use the newest code sent to this email address.");

    const firstMessage = await readOtpFor(email);
    await expect(page.getByRole("button", { name: "Resend code" })).toBeEnabled();
    const resendResponse = page.waitForResponse((response) => response.url().endsWith("/v1/auth/email/request"));
    await page.getByRole("button", { name: "Resend code" }).click();
    expect((await resendResponse).status()).toBe(202);

    const resentMessage = await readOtpFor(email);
    expect(resentMessage.challenge_id).not.toBe(firstMessage.challenge_id);
    await page.getByLabel("One-time code").fill(resentMessage.code);
    await page.getByRole("button", { name: "Verify code" }).click();
    await expect(page).toHaveURL(/\/admin$/u);
    await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
  });

  test("keeps protected admin routes behind the authenticated session boundary", async ({ page }) => {
    await page.goto("/admin/access");
    await expect(page).toHaveURL(/\/auth\/signin\?next=%2Fadmin%2Faccess$/u);
    await expect(page.getByRole("heading", { name: "Sign in to Tax operations", exact: true })).toBeVisible();
    await expect(page.getByText("The admin dashboard is protected.", { exact: false })).toHaveCount(0);
  });
});
