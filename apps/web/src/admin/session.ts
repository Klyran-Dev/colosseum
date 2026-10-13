import { createContext, useContext } from "react";
import { AdminSession } from "./model";

export interface SessionContextValue {
  session: AdminSession | null;
  setSession: (session: AdminSession) => void;
  signOut: () => void;
}

export const SessionContext = createContext<SessionContextValue | null>(null);

export function useAdminSession(): SessionContextValue {
  const value = useContext(SessionContext);
  if (!value) throw new Error("Admin session context is unavailable.");
  return value;
}
