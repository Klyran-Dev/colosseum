import { Component, ErrorInfo, ReactNode } from "react";
import { DEFAULT_LOCALE } from "./formatters";
import { shellOperationalMessagesFor } from "./shellOperationalMessages";

type WorkspaceErrorBoundaryProps = {
  workspace: string;
  locale?: string;
  children: ReactNode;
};

type WorkspaceErrorBoundaryState = {
  error: Error | null;
};

const CHUNK_RECOVERY_KEY = "solana-tax-protocol:chunk-recovery";
const CHUNK_RECOVERY_COOLDOWN_MS = 30_000;

function isStaleChunkFailure(error: Error): boolean {
  const message = error.message.toLowerCase();
  return message.includes("dynamically imported module") || (message.includes("chunk") && message.includes("load"));
}

function recoverFromStaleChunk(error: Error): void {
  if (!isStaleChunkFailure(error)) return;
  try {
    const previousRecoveryAt = Number(window.sessionStorage.getItem(CHUNK_RECOVERY_KEY));
    if (Number.isFinite(previousRecoveryAt) && Date.now() - previousRecoveryAt < CHUNK_RECOVERY_COOLDOWN_MS) return;
    window.sessionStorage.setItem(CHUNK_RECOVERY_KEY, String(Date.now()));
    window.location.reload();
  } catch {
    // A storage or reload failure must leave the redacted recovery surface
    // available rather than turning a deployment mismatch into a new error.
  }
}

export default class WorkspaceErrorBoundary extends Component<WorkspaceErrorBoundaryProps, WorkspaceErrorBoundaryState> {
  state: WorkspaceErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): WorkspaceErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, _errorInfo: ErrorInfo): void {
    // Deliberately do not render or log exception details in the user surface.
    // A deployment may add a redacted, bounded telemetry sink at this boundary.
    recoverFromStaleChunk(error);
  }

  render() {
    if (this.state.error) {
      const messages = shellOperationalMessagesFor(this.props.locale ?? DEFAULT_LOCALE);
      return (
        <section className="panel workspace-error" role="alert" aria-labelledby="workspace-error-title">
          <p className="eyebrow">{messages.error.eyebrow}</p>
          <h2 id="workspace-error-title">{messages.error.title}</h2>
          <p>{messages.error.copy(this.props.workspace)}</p>
          <button type="button" onClick={() => window.location.reload()}>{messages.error.reload}</button>
        </section>
      );
    }
    return this.props.children;
  }
}
