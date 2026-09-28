const UUIDV7_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export const REQUEST_TIMEOUT_MS = {
  health: 2_000,
  default: 5_000,
  settlementSubmit: 15_000,
} as const;

function timeoutError(): DOMException {
  return new DOMException("Request timed out", "TimeoutError");
}

/**
 * Enforces a bounded browser-to-API deadline while preserving caller-driven
 * cancellation for route changes, unmounts, and stale form submissions.
 * The API remains the authority; a timeout never creates a client fallback.
 */
export async function fetchWithTimeout(
  input: RequestInfo | URL,
  init: RequestInit = {},
  timeoutMs: number = REQUEST_TIMEOUT_MS.default,
): Promise<Response> {
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0) {
    throw new RangeError("Request timeout must be a positive safe integer");
  }

  const controller = new AbortController();
  const externalSignal = init.signal;
  let timedOut = false;
  let timeoutId: ReturnType<typeof globalThis.setTimeout> | undefined;

  const abortFromCaller = () => controller.abort(externalSignal?.reason);
  if (externalSignal?.aborted) {
    abortFromCaller();
  } else {
    externalSignal?.addEventListener("abort", abortFromCaller, { once: true });
  }

  timeoutId = globalThis.setTimeout(() => {
    timedOut = true;
    controller.abort(timeoutError());
  }, timeoutMs);

  try {
    return await fetch(input, { ...init, signal: controller.signal });
  } catch (cause) {
    if (timedOut) throw timeoutError();
    throw cause;
  } finally {
    if (timeoutId !== undefined) globalThis.clearTimeout(timeoutId);
    externalSignal?.removeEventListener("abort", abortFromCaller);
  }
}

function payloadMessage(payload: unknown): string | null {
  if (
    typeof payload === "object" && payload !== null && "error" in payload
    && typeof payload.error === "object" && payload.error !== null
    && "message" in payload.error && typeof payload.error.message === "string"
  ) {
    return payload.error.message;
  }
  return null;
}

export function responseError(response: Response, payload: unknown, fallback: string): string {
  const message = payloadMessage(payload) ?? fallback;
  const requestId = response.headers.get("x-request-id")?.trim() ?? "";
  return UUIDV7_PATTERN.test(requestId)
    ? `${message} (support reference: ${requestId})`
    : message;
}
