import { expect, test, type Page } from "@playwright/test";
import { readdir, readFile } from "node:fs/promises";

interface LocalOtpMessage {
  schema: "solana-tax-protocol/local-email-inbox/v1";
  challenge_id: string;
  recipient: string;
  code: string;
  expires_at_unix: number;
}

interface LocalInvitationMessage {
  schema: "solana-tax-protocol/local-invitation-inbox/v1";
  invitation_id: string;
  recipient: string;
  role: string;
  expires_at_unix: number;
}

const INBOX_DIR = process.env.AUTH_INBOX_DIR ?? "/workspace/auth-inbox";
const SYNTHETIC_DOMAIN = "auth-browser.example.test";

function syntheticEmail(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}@${SYNTHETIC_DOMAIN}`;
}

async function readOtpFor(recipient: string): Promise<LocalOtpMessage> {
  const deadline = Date.now() + 15_000;
  const observedRecipients = new Set<string>();
  while (Date.now() < deadline) {
    const names = await readdir(INBOX_DIR).catch(() => []);
    let latest: LocalOtpMessage | undefined;
    for (const name of names.filter((candidate) => candidate.endsWith(".json"))) {
      const raw = await readFile(`${INBOX_DIR}/${name}`, "utf8").catch(() => "");
      try {
        const candidate = JSON.parse(raw) as Partial<LocalOtpMessage>;
        if (typeof candidate.recipient === "string") observedRecipients.add(candidate.recipient);
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
  throw new Error(`No synthetic OTP email was delivered for ${recipient}. Observed recipients: ${[...observedRecipients].join(", ") || "none"}.`);
}

async function readInvitationFor(recipient: string): Promise<LocalInvitationMessage> {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    const names = await readdir(INBOX_DIR).catch(() => []);
    for (const name of names.filter((candidate) => candidate.startsWith("invitation-") && candidate.endsWith(".json"))) {
      const raw = await readFile(`${INBOX_DIR}/${name}`, "utf8").catch(() => "");
      try {
        const candidate = JSON.parse(raw) as Partial<LocalInvitationMessage>;
        if (candidate.schema === "solana-tax-protocol/local-invitation-inbox/v1"
          && candidate.recipient === recipient
          && typeof candidate.invitation_id === "string"
          && typeof candidate.role === "string"
          && typeof candidate.expires_at_unix === "number") {
          return candidate as LocalInvitationMessage;
        }
      } catch {
        // The sender may still be replacing the file; retry until the deadline.
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`No invitation email was delivered for ${recipient}.`);
}

async function completeOtp(page: Page, intent: "signin" | "signup", email: string, nextPath: string, invitationId?: string, expectedInvitationRole?: string): Promise<void> {
  const otpRequests: string[] = [];
  page.on("request", (request) => {
    if (request.url().endsWith("/v1/auth/email/request")) otpRequests.push(request.postData() ?? "");
  });
  const invitationQuery = invitationId ? `&invite=${encodeURIComponent(invitationId)}` : "";
  await page.goto(`/auth/${intent}?next=${encodeURIComponent(nextPath)}${invitationQuery}`);
  await expect(page.getByRole("heading", {
    name: intent === "signup" ? "Create your workspace account" : "Sign in to Solana Tax Protocol",
    exact: true,
  })).toBeVisible();
  await expect(page.getByLabel("Work email")).toBeVisible();
  if (invitationId) {
    await expect(page.getByText("Invitation received", { exact: true })).toBeVisible();
  }
  await page.getByLabel("Work email").fill(email);
  const requestResponse = page.waitForResponse((response) => response.url().endsWith("/v1/auth/email/request"));
  await page.getByRole("button", { name: intent === "signup" ? "Create account" : "Continue with email" }).click();
  const response = await requestResponse;
  if (!response.ok()) {
    throw new Error(`OTP request for ${email} failed with HTTP ${response.status()} (${otpRequests.length} browser request(s)): ${await response.text()}`);
  }
  if (invitationId) expect(JSON.parse(otpRequests.at(-1) ?? "{}").invitation_id).toBe(invitationId);
  await expect(page.getByRole("heading", { name: "Check your email" })).toBeVisible();
  if (expectedInvitationRole) await expect(page.getByText(`${expectedInvitationRole} responsibility`, { exact: true })).toBeVisible();

  const message = await readOtpFor(email);
  expect(message.expires_at_unix).toBeGreaterThan(Math.floor(Date.now() / 1000));
  await page.getByLabel("One-time code").fill(message.code);
  const verifyResponse = page.waitForResponse((response) => response.url().endsWith("/v1/auth/email/verify"));
  await page.getByRole("button", { name: "Verify code" }).click();
  const verification = await verifyResponse;
  if (!verification.ok()) {
    throw new Error(`OTP verification for ${email} failed with HTTP ${verification.status()}: ${await verification.text()}`);
  }
  await expect(page).toHaveURL(new RegExp(`${nextPath.replaceAll("/", "\\/")}$`));
  await expect(page.getByRole("button", { name: "Open account menu" })).toBeVisible();
}

async function signOut(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Open account menu" }).click();
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByRole("button", { name: "Email sign-in" })).toBeVisible();
}

test.describe("Docker-backed email OTP authentication", () => {
  test("signs up a standard operator, signs out, then signs in again", async ({ page }) => {
    const email = syntheticEmail("operator");
    await completeOtp(page, "signup", email, "/operations/tax-simulator");
    await expect(page.getByRole("button", { name: "Open account menu" })).toBeVisible();
    await signOut(page);
    await completeOtp(page, "signin", email, "/operations/tax-quotes");
    await expect(page.getByRole("heading", { name: "Tax quotes", exact: true })).toBeVisible();
  });

  test("resends an OTP through the server challenge boundary", async ({ page }) => {
    const email = syntheticEmail("resend");
    await page.goto("/auth/signup?next=%2Foperations%2Ftax-simulator");
    await page.getByLabel("Work email").fill(email);
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByRole("heading", { name: "Check your email" })).toBeVisible();

    const firstMessage = await readOtpFor(email);
    await expect(page.getByRole("button", { name: "Resend code" })).toBeEnabled();
    const resendResponse = page.waitForResponse((response) => response.url().endsWith("/v1/auth/email/request"));
    await page.getByRole("button", { name: "Resend code" }).click();
    expect([200, 202]).toContain((await resendResponse).status());

    const resentMessage = await readOtpFor(email);
    expect(resentMessage.challenge_id).not.toBe(firstMessage.challenge_id);
    await page.getByLabel("One-time code").fill(resentMessage.code);
    await page.getByRole("button", { name: "Verify code" }).click();
    await expect(page).toHaveURL(/\/operations\/tax-simulator$/u);
    await expect(page.getByRole("button", { name: "Open account menu" })).toBeVisible();
  });

  test("bootstraps an administrator, creates an invitation, and provisions a moderator", async ({ page }) => {
    const administratorEmail = "admin@local.test";
    const moderatorEmail = syntheticEmail("moderator");
    const revokedEmail = syntheticEmail("revoked");
    const operatorEmail = syntheticEmail("operator");

    await completeOtp(page, "signup", administratorEmail, "/rules/access-approvals");
    await expect(page.getByRole("heading", { name: "People and responsibilities" })).toBeVisible();
    await page.getByLabel("Invite by email").fill(revokedEmail);
    await page.getByLabel("Initial responsibility").selectOption("merchant_operator");
    await page.getByRole("button", { name: "Create invitation" }).click();
    const revokedRow = page.getByRole("row", { name: new RegExp(revokedEmail.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&")) });
    await expect(revokedRow).toContainText("Pending");
    await revokedRow.getByRole("button", { name: `Revoke invitation for ${revokedEmail}` }).click();
    await expect(revokedRow).toContainText("Revoked");

    await page.getByLabel("Invite by email").fill(operatorEmail);
    await page.getByLabel("Initial responsibility").selectOption("merchant_operator");
    await page.getByRole("button", { name: "Create invitation" }).click();
    await expect(page.locator(".role-management .inline-success")).toContainText(`Invitation created for ${operatorEmail}`);
    await signOut(page);

    await completeOtp(page, "signup", operatorEmail, "/rules/access-approvals");
    await signOut(page);
    await completeOtp(page, "signin", administratorEmail, "/rules/access-approvals");
    await page.getByRole("button", { name: "Open account menu" }).click();
    const administratorResponsibilities = page.getByLabel("Responsibilities", { exact: true });
    await expect(administratorResponsibilities.getByText("Responsibilities", { exact: true })).toBeVisible();
    await expect(administratorResponsibilities.getByText("Administrator", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Open account menu" }).click();
    const peopleTable = page.getByRole("table", { name: "People and assigned responsibilities" });
    const operatorRow = peopleTable.getByRole("row", { name: new RegExp(operatorEmail.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&")) });
    await expect(operatorRow).toContainText("Merchant operator");
    await operatorRow.getByRole("button", { name: "Assign moderator" }).click();
    await expect(operatorRow).toContainText("moderator");
    await operatorRow.getByRole("button", { name: "Revoke moderator" }).click();
    await expect(operatorRow.getByRole("button", { name: "Assign moderator" })).toBeVisible();
    await operatorRow.getByRole("combobox", { name: `Responsibility for ${operatorEmail}` }).selectOption("tax_rule_author");
    await operatorRow.getByRole("button", { name: `Assign tax rule author to ${operatorEmail}` }).click();
    await expect(operatorRow).toContainText("Tax rule author");
    await signOut(page);
    await completeOtp(page, "signin", operatorEmail, "/rules/access-approvals");
    await page.getByRole("button", { name: "Open account menu" }).click();
    const operatorResponsibilities = page.getByLabel("Responsibilities", { exact: true });
    await expect(operatorResponsibilities.getByText("Tax rule author", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Open account menu" }).click();
    await signOut(page);
    await completeOtp(page, "signin", administratorEmail, "/rules/access-approvals");

    await page.getByLabel("Invite by email").fill(moderatorEmail);
    await page.getByLabel("Initial responsibility").selectOption("moderator");
    await page.getByRole("button", { name: "Create invitation" }).click();
    await expect(page.locator(".role-management .inline-success")).toContainText(`Invitation created for ${moderatorEmail}`);
    const moderatorInvitation = await readInvitationFor(moderatorEmail);
    expect(moderatorInvitation.role).toBe("moderator");
    await signOut(page);

    await completeOtp(page, "signup", moderatorEmail, "/rules/access-approvals", moderatorInvitation.invitation_id, "Moderator");
    await page.getByRole("button", { name: "Open account menu" }).click();
    const moderatorResponsibilities = page.getByLabel("Responsibilities", { exact: true });
    await expect(moderatorResponsibilities.getByText("Responsibilities", { exact: true })).toBeVisible();
    await expect(moderatorResponsibilities.getByText("Moderator", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Open account menu" }).click();
    await expect(page.getByText("Administrator approval required", { exact: true })).toBeVisible();
    await expect(page.getByText("Your account can view access rules, but the service has not granted role-management authority.", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Create invitation" })).toHaveCount(0);

    await page.getByRole("button", { name: "Expand navigation" }).click();
    await page.getByRole("navigation", { name: "Primary" }).getByText("Records & issues", { exact: true }).click();
    await page.getByRole("button", { name: "Evidence & audit" }).click();
    await expect(page.getByRole("heading", { name: "Review evidence access", exact: true })).toBeVisible();
    const auditResponse = page.waitForResponse((response) => response.url().includes("/v1/audit/evidence-access"));
    await page.getByRole("button", { name: "Load access history" }).click();
    const auditResult = await auditResponse;
    expect(auditResult.status()).toBe(200);
    await expect(page.locator(".evidence-control-panel .inline-success")).toContainText("0 records");
  });
});

