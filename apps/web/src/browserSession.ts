import { createClientUuid } from "./clientIdentity";

export const BROWSER_SESSION_CHANNEL = "solana-tax.browser-session.v1";
export const BROWSER_SESSION_STORAGE_KEY = "solana-tax.browser-session-event.v1";

export type BrowserSessionSignal = {
  kind: "claimed" | "released";
  sessionId: string;
  issuedAtUnix: number;
};

type BrowserSessionChannel = {
  postMessage: (value: unknown) => void;
  addEventListener: (type: "message", listener: (event: MessageEvent<unknown>) => void) => void;
  removeEventListener: (type: "message", listener: (event: MessageEvent<unknown>) => void) => void;
  close: () => void;
};

type BrowserSessionStorage = Pick<Storage, "setItem">;

export interface BrowserSessionCoordinator {
  claim: () => string;
  release: () => void;
  dispose: () => void;
}

interface BrowserSessionDependencies {
  channel?: BrowserSessionChannel | null;
  storage?: BrowserSessionStorage | null;
  createSessionId?: () => string;
  nowUnix?: () => number;
}

function validSessionSignal(value: unknown): value is BrowserSessionSignal {
  if (!value || typeof value !== "object") return false;
  const signal = value as Partial<BrowserSessionSignal>;
  return (signal.kind === "claimed" || signal.kind === "released")
    && typeof signal.sessionId === "string"
    && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u.test(signal.sessionId)
    && typeof signal.issuedAtUnix === "number"
    && Number.isSafeInteger(signal.issuedAtUnix)
    && signal.issuedAtUnix > 0;
}

function browserChannel(): BrowserSessionChannel | null {
  if (typeof globalThis.BroadcastChannel !== "function") return null;
  return new globalThis.BroadcastChannel(BROWSER_SESSION_CHANNEL);
}

function browserStorage(): BrowserSessionStorage | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}

/**
 * Coordinates one active browser identity without persisting or transmitting its bearer token.
 * The signal is advisory UI state; every API request remains protected by server-side JWT checks.
 */
export function createBrowserSessionCoordinator(
  onSignal: (signal: BrowserSessionSignal) => void,
  dependencies: BrowserSessionDependencies = {},
): BrowserSessionCoordinator {
  const channel = dependencies.channel === undefined ? browserChannel() : dependencies.channel;
  const storage = dependencies.storage === undefined ? browserStorage() : dependencies.storage;
  const createSessionId = dependencies.createSessionId ?? createClientUuid;
  const nowUnix = dependencies.nowUnix ?? (() => Math.floor(Date.now() / 1000));
  let currentSessionId: string | null = null;
  let disposed = false;

  const receive = (value: unknown) => {
    if (disposed || !validSessionSignal(value)) return;
    if (value.kind === "released" && value.sessionId !== currentSessionId) return;
    onSignal(value);
  };
  const channelListener = (event: MessageEvent<unknown>) => receive(event.data);
  channel?.addEventListener("message", channelListener);
  const storageListener = (event: StorageEvent) => {
    if (event.key === BROWSER_SESSION_STORAGE_KEY && event.newValue) {
      try {
        receive(JSON.parse(event.newValue) as unknown);
      } catch {
        // Ignore malformed cross-tab data; it is never an authority signal.
      }
    }
  };
  if (typeof window !== "undefined") window.addEventListener("storage", storageListener);

  const publish = (signal: BrowserSessionSignal) => {
    channel?.postMessage(signal);
    try {
      storage?.setItem(BROWSER_SESSION_STORAGE_KEY, JSON.stringify(signal));
    } catch {
      // Cross-tab signalling remains available through BroadcastChannel when storage is blocked.
    }
  };

  return {
    claim: () => {
      const sessionId = createSessionId();
      currentSessionId = sessionId;
      publish({ kind: "claimed", sessionId, issuedAtUnix: nowUnix() });
      return sessionId;
    },
    release: () => {
      if (!currentSessionId) return;
      const sessionId = currentSessionId;
      currentSessionId = null;
      publish({ kind: "released", sessionId, issuedAtUnix: nowUnix() });
    },
    dispose: () => {
      if (disposed) return;
      disposed = true;
      channel?.removeEventListener("message", channelListener);
      channel?.close();
      if (typeof window !== "undefined") window.removeEventListener("storage", storageListener);
      currentSessionId = null;
    },
  };
}
