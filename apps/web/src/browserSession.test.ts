import { afterEach, describe, expect, it, vi } from "vitest";
import { createBrowserSessionCoordinator, BrowserSessionSignal } from "./browserSession";

type FakeChannel = {
  postMessage: (value: unknown) => void;
  addEventListener: (type: "message", listener: (event: MessageEvent<unknown>) => void) => void;
  removeEventListener: (type: "message", listener: (event: MessageEvent<unknown>) => void) => void;
  close: () => void;
};

function channelHarness() {
  const listeners = new Set<(event: MessageEvent<unknown>) => void>();
  const channel: FakeChannel = {
    postMessage: (value) => listeners.forEach((listener) => listener(new MessageEvent("message", { data: value }))),
    addEventListener: (_type, listener) => listeners.add(listener),
    removeEventListener: (_type, listener) => listeners.delete(listener),
    close: () => listeners.clear(),
  };
  return channel;
}

function storageHarness() {
  const values = new Map<string, string>();
  return {
    setItem: (key: string, value: string) => values.set(key, value),
    getItem: (key: string) => values.get(key) ?? null,
  };
}

afterEach(() => vi.restoreAllMocks());

describe("browser session coordination", () => {
  it("publishes only a non-secret claim and revokes it on release", () => {
    const channel = channelHarness();
    const storage = storageHarness();
    const coordinator = createBrowserSessionCoordinator(() => undefined, { channel, storage });

    const sessionId = coordinator.claim();
    expect(sessionId).toMatch(/^[0-9a-f-]{36}$/u);
    const claim = JSON.parse(storage.getItem("solana-tax.browser-session-event.v1") ?? "null") as BrowserSessionSignal;
    expect(claim).toMatchObject({ kind: "claimed", sessionId });
    expect(JSON.stringify(claim)).not.toContain("token");
    expect(JSON.stringify(claim)).not.toContain("bearer");

    coordinator.release();
    expect(JSON.parse(storage.getItem("solana-tax.browser-session-event.v1") ?? "null")).toMatchObject({ kind: "released", sessionId });
    coordinator.dispose();
  });

  it("ignores malformed cross-tab signals", () => {
    const channel = channelHarness();
    const storage = storageHarness();
    const onSignal = vi.fn();
    const coordinator = createBrowserSessionCoordinator(onSignal, { channel, storage });

    channel.postMessage({ kind: "claimed", sessionId: "not-a-uuid" });
    channel.postMessage({ kind: "claimed", sessionId: "0192e6ec-4d8f-7d2e-a40e-2c095f871405", issuedAtUnix: -1 });

    expect(onSignal).not.toHaveBeenCalled();
    coordinator.dispose();
  });

  it("stops delivering signals after disposal", () => {
    const channel = channelHarness();
    const storage = storageHarness();
    const onSignal = vi.fn();
    const coordinator = createBrowserSessionCoordinator(onSignal, { channel, storage });
    coordinator.dispose();

    channel.postMessage({ kind: "claimed", sessionId: "0192e6ec-4d8f-7d2e-a40e-2c095f871405", issuedAtUnix: 1 });

    expect(onSignal).not.toHaveBeenCalled();
  });
});
