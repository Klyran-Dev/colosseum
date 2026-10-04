import { afterEach, describe, expect, it, vi } from "vitest";
import { fetchWithTimeout, MAX_RESPONSE_ERROR_MESSAGE_LENGTH, REQUEST_TIMEOUT_MS, responseError } from "./api";

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});

describe("API failure boundary", () => {
  it("returns the server message with a canonical UUIDv7 support reference", () => {
    const response = new Response(null, {
      status: 409,
      headers: { "x-request-id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13" },
    });
    expect(responseError(response, { error: { message: "Quote expired" } }, "Fallback"))
      .toBe("Quote expired (support reference: 01992f1b-4a70-7e30-8ea2-b6e37a9d9d13)");
  });

  it("does not render an invalid or caller-controlled request ID", () => {
    const response = new Response(null, {
      status: 500,
      headers: { "x-request-id": "caller-controlled-value" },
    });
    expect(responseError(response, null, "Service failed")).toBe("Service failed");
  });

  it("bounds and normalizes untrusted server error text before rendering", () => {
    const response = new Response(null, { status: 500 });
    const message = `  provider\n${"x".repeat(MAX_RESPONSE_ERROR_MESSAGE_LENGTH + 100)}\u0000`;

    const rendered = responseError(response, { error: { message } }, "Fallback");

    expect(rendered).toHaveLength(MAX_RESPONSE_ERROR_MESSAGE_LENGTH);
    expect(rendered.startsWith("provider x")).toBe(true);
    expect(rendered.endsWith("…")).toBe(true);
    expect(rendered).not.toMatch(/[\u0000-\u001f\u007f\n\r]/);
  });

  it("uses the bounded fallback when a server message is empty", () => {
    const response = new Response(null, { status: 503 });
    expect(responseError(response, { error: { message: " \n\t" } }, "Service unavailable"))
      .toBe("Service unavailable");
  });

  it("aborts a hanging request at the bounded deadline", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("fetch", vi.fn((_input: RequestInfo | URL, init?: RequestInit) => new Promise<Response>((_resolve, reject) => {
      init?.signal?.addEventListener("abort", () => reject(init.signal?.reason), { once: true });
    })));

    const pending = fetchWithTimeout("/api/v1/tax/quote", {}, 25);
    const rejection = expect(pending).rejects.toMatchObject({ name: "TimeoutError", message: "Request timed out" });
    await vi.advanceTimersByTimeAsync(25);

    await rejection;
  });

  it("preserves caller cancellation instead of converting it to a timeout", async () => {
    const caller = new AbortController();
    const reason = new DOMException("Navigation changed", "AbortError");
    vi.stubGlobal("fetch", vi.fn((_input: RequestInfo | URL, init?: RequestInit) => new Promise<Response>((_resolve, reject) => {
      init?.signal?.addEventListener("abort", () => reject(init.signal?.reason), { once: true });
    })));

    const pending = fetchWithTimeout("/api/v1/tax/quote", { signal: caller.signal }, REQUEST_TIMEOUT_MS.default);
    caller.abort(reason);

    await expect(pending).rejects.toBe(reason);
  });

  it("rejects invalid timeout budgets before making a request", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    await expect(fetchWithTimeout("/api/healthz", {}, 0)).rejects.toThrow("positive safe integer");
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
