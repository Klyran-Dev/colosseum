import { operatorFacingError, OperatorFacingError } from "./operatorError";

const UUIDV7_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
export const MAX_RESPONSE_ERROR_MESSAGE_LENGTH = 512;

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

function boundedDisplayMessage(value: string): string | null {
  const normalized = value
    .replace(/[\u0000-\u001f\u007f]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (!normalized) return null;
  if (normalized.length <= MAX_RESPONSE_ERROR_MESSAGE_LENGTH) return normalized;
  return `${normalized.slice(0, MAX_RESPONSE_ERROR_MESSAGE_LENGTH - 1)}…`;
}

export function responseError(response: Response, payload: unknown, fallback: string): string {
  // The response payload is intentionally not a presentation authority. It
  // may contain server/provider diagnostics even when bounded and normalized.
  void payload;
  const message = boundedDisplayMessage(fallback)
    ?? "The request could not be completed.";
  const withoutHttpStatus = message.replace(/\s+\(\d{3}\)(?=[.!?]?$)/u, "");
  const requestId = response.headers.get("x-request-id")?.trim() ?? "";
  return UUIDV7_PATTERN.test(requestId)
    ? `${withoutHttpStatus} (support reference: ${requestId})`
    : withoutHttpStatus;
}

export function operatorResponseError(response: Response, payload: unknown, fallback: string): OperatorFacingError {
  const supportReference = response.headers.get("x-request-id")?.trim() ?? "";
  return operatorFacingError(
    responseError(response, payload, fallback),
    UUIDV7_PATTERN.test(supportReference) ? supportReference : null,
  );
}
