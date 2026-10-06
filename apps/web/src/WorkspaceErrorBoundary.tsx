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

export default class WorkspaceErrorBoundary extends Component<WorkspaceErrorBoundaryProps, WorkspaceErrorBoundaryState> {
  state: WorkspaceErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): WorkspaceErrorBoundaryState {
    return { error };
  }

  componentDidCatch(_error: Error, _errorInfo: ErrorInfo): void {
    // Deliberately do not render or log exception details in the user surface.
    // A deployment may add a redacted, bounded telemetry sink at this boundary.
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
