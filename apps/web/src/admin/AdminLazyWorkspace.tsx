import { ReactNode, Suspense } from "react";
import WorkspaceErrorBoundary from "../WorkspaceErrorBoundary";

interface AdminLazyWorkspaceProps {
  workspace: string;
  fallback: ReactNode;
  children: ReactNode;
}

export default function AdminLazyWorkspace({ workspace, fallback, children }: AdminLazyWorkspaceProps) {
  return <WorkspaceErrorBoundary workspace={workspace}><Suspense fallback={fallback}>{children}</Suspense></WorkspaceErrorBoundary>;
}
