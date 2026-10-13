// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import GovernancePanel from "./GovernancePanel";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("governance workbench submission boundary", () => {
  function fillReleaseEvidence() {
    fireEvent.change(screen.getByLabelText("Source record"), { target: { value: "controlled-record://test/rule-001" } });
    fireEvent.change(screen.getByLabelText(/^Source SHA-256 digest/), { target: { value: "ab".repeat(32) } });
    fireEvent.change(screen.getByLabelText("Legal/tax owner"), { target: { value: "tax-owner:test" } });
    fireEvent.change(screen.getByLabelText("Reviewed on"), { target: { value: "2026-09-27T10:00" } });
    fireEvent.change(screen.getByLabelText("Review record"), { target: { value: "review://test/rule-001" } });
  }

  it("makes governance authority actions visibly unavailable before sign-in", () => {
    render(<GovernancePanel apiBase="/api" serviceOnline accessToken="" />);

    expect(screen.getByRole("note").textContent).toContain("Sign in to continue");
    expect(screen.getByLabelText("Rule name or reference").closest("fieldset")).toHaveProperty("disabled", true);
    expect(screen.getByLabelText("Saved rule reference").closest("fieldset")).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Create governed draft" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Load rule details" })).toHaveProperty("disabled", true);
  });

  it("presents rule authoring as a guided tax task", () => {
    render(<GovernancePanel apiBase="/api" serviceOnline accessToken="author-token" />);

    const authoring = screen.getByText("Create a governed draft", { exact: true, selector: "summary" }).closest("details");
    expect(authoring).toBeTruthy();
    expect(authoring?.hasAttribute("open")).toBe(false);
    fireEvent.click(screen.getByText("Create a governed draft", { exact: true, selector: "summary" }));
    expect(authoring?.hasAttribute("open")).toBe(true);
    expect(screen.getByRole("heading", { name: "How the tax is calculated" })).toBeTruthy();
    expect(screen.getByText("Checked before publishing")).toBeTruthy();
    expect(screen.getByLabelText("Source record")).toBeTruthy();
    expect(screen.getByLabelText(/^Source SHA-256 digest/)).toBeTruthy();
    expect(screen.getByLabelText("Legal/tax owner")).toBeTruthy();
    expect(screen.getByLabelText("Reviewed on")).toBeTruthy();
    expect(screen.getByLabelText("Review record")).toBeTruthy();
    expect(screen.getByLabelText("How the rate changes")).toBeTruthy();
    expect(screen.getByLabelText("When rounding happens")).toBeTruthy();
    const simulation = screen.getByText("Try the saved rule", { exact: true }).closest("details");
    expect(simulation).toBeTruthy();
    expect(simulation?.hasAttribute("open")).toBe(false);
    fireEvent.click(screen.getByText("Try the saved rule", { exact: true }));
    expect(simulation?.hasAttribute("open")).toBe(true);
    expect(screen.queryByText("Structured Tax Engine authoring")).toBeNull();
    expect(screen.queryByText("Rule components and semantics")).toBeNull();
    expect(screen.queryByText(/canonical rule document from explicit controls/i)).toBeNull();
  });

  it("keeps complex rule components progressively disclosed without changing the draft", () => {
    render(<GovernancePanel apiBase="/api" serviceOnline accessToken="author-token" />);

    fireEvent.click(screen.getByText("Create a governed draft", { exact: true, selector: "summary" }));
    const firstSummary = screen.getByText("Component 1", { exact: true, selector: ".workbench-component-summary-title" }).closest("summary");
    expect(firstSummary).toBeTruthy();
    if (!firstSummary) throw new Error("Component 1 disclosure summary was not rendered");
    expect(firstSummary.closest("details")?.hasAttribute("open")).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "Add tax component" }));
    const secondSummary = screen.getByText("Component 2", { exact: true, selector: ".workbench-component-summary-title" }).closest("summary");
    expect(secondSummary).toBeTruthy();
    if (!secondSummary) throw new Error("Component 2 disclosure summary was not rendered");
    expect(secondSummary.closest("details")?.hasAttribute("open")).toBe(true);

    fireEvent.click(firstSummary);
    expect(firstSummary.closest("details")?.hasAttribute("open")).toBe(false);
    expect(secondSummary.closest("details")?.hasAttribute("open")).toBe(true);
  });

  it("localizes governance authoring while preserving protocol values", () => {
    render(<GovernancePanel apiBase="/api" serviceOnline accessToken="author-token" locale="de-DE" />);

    expect(screen.getByRole("heading", { name: "Regel prüfen und veröffentlichen" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "So wird die Steuer berechnet" })).toBeTruthy();
    expect(screen.getByLabelText("Quelldatensatz")).toBeTruthy();

    const editor = screen.getByLabelText("Wie soll die Regel eingegeben werden?") as HTMLSelectElement;
    expect(editor.value).toBe("structured");
    expect((screen.getByRole("option", { name: "Geführter Regel-Builder" }) as HTMLOptionElement).value).toBe("structured");
    expect((screen.getByRole("option", { name: "Erweitertes JSON" }) as HTMLOptionElement).value).toBe("json");

    const priceMode = screen.getByLabelText("Wie wird der Preis angezeigt?") as HTMLSelectElement;
    expect(priceMode.value).toBe("Exclusive");
    expect((screen.getByRole("option", { name: "Steuer wird aufgeschlagen" }) as HTMLOptionElement).value).toBe("Exclusive");
  });

  it("submits the structured canonical rule document through the authenticated API", async () => {
    const draftResponse = {
      rule_set_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
      external_id: "new-sales-tax",
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest: Array.from({ length: 32 }, (_, index) => index),
      effective_from: "2026-09-27T00:00:00.000Z",
      effective_until: null,
      governance_revision: 0,
    };
    const governanceResponse = {
      rule_set_id: draftResponse.rule_set_id,
      external_id: draftResponse.external_id,
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest_hex: "ab".repeat(32),
      author_subject: "author",
      reviewer_subject: null,
      approver_subject: null,
      governance_revision: 0,
      effective_from: draftResponse.effective_from,
      effective_until: null,
      events: [],
    };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(draftResponse), { status: 201 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(governanceResponse), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    render(<GovernancePanel apiBase="/api" serviceOnline accessToken="author-token" />);
    fillReleaseEvidence();
    fireEvent.click(screen.getByRole("button", { name: "Create governed draft" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    const requestBody = JSON.parse(fetchMock.mock.calls[0][1].body as string) as Record<string, unknown>;
    const ruleSet = requestBody.rule_set as Record<string, unknown>;
    const component = (ruleSet.components as Array<Record<string, unknown>>)[0];

    expect(fetchMock.mock.calls[0][0]).toBe("/api/v1/tax/rules");
    expect(requestBody.authorization).toBeUndefined();
    expect(ruleSet.id).toBe("new-sales-tax");
    expect(ruleSet.version).toBe(1);
    expect(component.rate).toEqual({ Flat: { numerator: "10", denominator: "100" } });
    expect(component.base).toEqual({ sources: ["Net"] });
    expect(component.credit_atomic).toBe("0");
    expect(requestBody.provenance).toEqual({
      schema: "tax-rule-provenance/v1",
      source_reference: "controlled-record://test/rule-001",
      source_digest_hex: "ab".repeat(32),
      legal_tax_owner: "tax-owner:test",
      jurisdiction_scope: "US-CA",
      reviewed_at_unix: Math.floor(Date.parse("2026-09-27T10:00") / 1000),
      review_reference: "review://test/rule-001",
    });
    expect(fetchMock.mock.calls[0][1].headers).toMatchObject({ authorization: "Bearer author-token" });
    expect(screen.getByText(/Draft new-sales-tax v1 was created/)).toBeTruthy();
  });

  it("simulates the server-persisted rule after the governance snapshot is loaded", async () => {
    const draftResponse = {
      rule_set_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
      external_id: "new-sales-tax",
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest: Array.from({ length: 32 }, (_, index) => index),
      effective_from: "2026-09-27T00:00:00.000Z",
      effective_until: null,
      governance_revision: 0,
    };
    const governanceResponse = {
      rule_set_id: draftResponse.rule_set_id,
      external_id: draftResponse.external_id,
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest_hex: "ab".repeat(32),
      author_subject: "author",
      reviewer_subject: null,
      approver_subject: null,
      governance_revision: 0,
      effective_from: draftResponse.effective_from,
      effective_until: null,
      events: [],
    };
    const simulationResponse = {
      calculation_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d16",
      status: "simulated_unattested",
      result: {
        rule_set_id: "new-sales-tax",
        rule_version: 1,
        engine_version: "0.1.0",
        atomic_decimals: 2,
        currency: "USD",
        jurisdiction: "US-CA",
        facts_digest: Array.from({ length: 32 }, (_, index) => index),
        rule_digest: Array.from({ length: 32 }, (_, index) => 255 - index),
        gross_atomic: "1100",
        total_tax_atomic: "100",
        total_net_atomic: "1000",
        components: [{ id: "sales-tax", label: "Sales tax", base_atomic: "1000", tax_atomic: "100", explanation: "10% of net" }],
      },
    };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(draftResponse), { status: 201 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(governanceResponse), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(simulationResponse), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    render(<GovernancePanel apiBase="/api" serviceOnline accessToken="author-token" />);
    fillReleaseEvidence();
    fireEvent.click(screen.getByRole("button", { name: "Create governed draft" }));
    await waitFor(() => expect((screen.getByRole("button", { name: "Run controlled simulation" }) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.change(screen.getAllByLabelText("Amount before tax")[0], { target: { value: "1100" } });
    fireEvent.change(screen.getAllByLabelText("Amount subject to tax")[0], { target: { value: "1000" } });
    fireEvent.change(screen.getAllByLabelText("Currency")[0], { target: { value: "USD" } });
    fireEvent.change(screen.getAllByRole("textbox", { name: /Asset precision/ })[0], { target: { value: "2" } });
    fireEvent.click(screen.getByRole("button", { name: "Run controlled simulation" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    expect(fetchMock.mock.calls[2][0]).toBe(`/api/v1/tax/rules/${draftResponse.rule_set_id}/simulate`);
    expect(JSON.parse(fetchMock.mock.calls[2][1].body as string)).toEqual({
      facts: { gross_atomic: "1100", net_atomic: "1000", atomic_decimals: 2, currency: "USD", jurisdiction: "US-CA" },
    });
    await waitFor(() => expect(screen.getByText(/controlled scenario result/i)).toBeTruthy());
  });

  it("redacts provider and transport exceptions from governance recovery copy", async () => {
    const providerDetail = "provider-internal governance diagnostic";
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error(providerDetail)));
    render(<GovernancePanel apiBase="/api" serviceOnline accessToken="author-token" />);

    fireEvent.change(screen.getByLabelText("Saved rule reference"), { target: { value: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15" } });
    fireEvent.click(screen.getByRole("button", { name: "Load rule details" }));

    await screen.findByText("Governance lookup failed");
    expect(screen.queryByText(providerDetail)).toBeNull();
  });

  it("shows the decision trail with who did what and when after the snapshot loads", async () => {
    const draftResponse = {
      rule_set_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
      external_id: "new-sales-tax",
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest: Array.from({ length: 32 }, (_, index) => index),
      effective_from: "2026-09-27T00:00:00.000Z",
      effective_until: null,
      governance_revision: 0,
    };
    const governanceResponse = {
      rule_set_id: draftResponse.rule_set_id,
      external_id: draftResponse.external_id,
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "approved",
      rule_digest_hex: "ab".repeat(32),
      author_subject: "author",
      reviewer_subject: "reviewer",
      approver_subject: "approver",
      governance_revision: 2,
      effective_from: draftResponse.effective_from,
      effective_until: null,
      events: [
        { id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d31", actor_subject: "author", action: "create", from_status: "none", to_status: "draft", evidence: { source: "controlled-record://test/rule-001" }, occurred_at: "2026-09-27T10:00:00.000Z" },
        { id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d32", actor_subject: "reviewer", action: "review", from_status: "draft", to_status: "reviewed", evidence: { review: "review://test/rule-001" }, occurred_at: "2026-09-27T11:00:00.000Z" },
        { id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d33", actor_subject: "approver", action: "approve", from_status: "reviewed", to_status: "approved", evidence: { approval: "approval://test/rule-001" }, occurred_at: "2026-09-27T12:00:00.000Z" },
      ],
    };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(draftResponse), { status: 201 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(governanceResponse), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    const { container } = render(<GovernancePanel apiBase="/api" serviceOnline accessToken="author-token" />);
    fillReleaseEvidence();
    fireEvent.click(screen.getByRole("button", { name: "Create governed draft" }));

    await waitFor(() => expect(container.querySelectorAll(".governance-timeline > li")).toHaveLength(3));
    expect(container.querySelectorAll(".timeline-marker")).toHaveLength(3);
    expect(container.querySelector(".governance-timeline")?.textContent).toMatch(/reviewer/);
    expect(container.querySelector(".governance-timeline")?.textContent).toMatch(/approver/);
  });

  it("shows editable rate bands when the rate is made progressive", () => {
    const { container } = render(<GovernancePanel apiBase="/api" serviceOnline accessToken="author-token" />);

    expect(container.querySelectorAll(".bracket-row")).toHaveLength(0);
    fireEvent.change(screen.getByLabelText("How the rate changes"), { target: { value: "progressive" } });

    expect(container.querySelector(".bracket-list")).toBeTruthy();
    expect(container.querySelectorAll(".bracket-row").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByLabelText("Lower amount").length).toBeGreaterThanOrEqual(1);
  });

  it("keeps a base-source share fraction together as one unit", () => {
    const { container } = render(<GovernancePanel apiBase="/api" serviceOnline accessToken="author-token" />);

    const divider = container.querySelector(".base-share-divider");
    expect(divider).toBeTruthy();
    const group = divider?.parentElement as HTMLElement;
    expect(group.classList.contains("base-share-fraction")).toBe(true);
    expect(group.querySelectorAll("label")).toHaveLength(2);
    expect(group.querySelector("input")).toBeTruthy();
  });
});
