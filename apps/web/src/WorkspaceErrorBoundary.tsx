import { Component, ErrorInfo, ReactNode } from "react";

type WorkspaceErrorBoundaryProps = {
  workspace: string;
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
      return (
        <section className="panel workspace-error" role="alert" aria-labelledby="workspace-error-title">
          <p className="eyebrow">Workspace unavailable</p>
          <h2 id="workspace-error-title">This workspace could not be loaded</h2>
          <p>The {this.props.workspace} workspace failed before it could render. No calculation, wallet action, settlement, or evidence state was inferred.</p>
          <button type="button" onClick={() => window.location.reload()}>Reload application</button>
        </section>
      );
    }
    return this.props.children;
  }
}
