export type DeploymentEnvironment = "local" | "staging" | "production";
export type SolanaNetwork = "localnet" | "devnet" | "testnet" | "mainnet-beta";

export interface RuntimeContextValue<T extends string> {
  value: T | null;
  label: string;
  configured: boolean;
}

export interface RuntimeContext {
  environment: RuntimeContextValue<DeploymentEnvironment>;
  network: RuntimeContextValue<SolanaNetwork>;
  configured: boolean;
  warnings: readonly string[];
}

const ENVIRONMENT_LABELS: Readonly<Record<DeploymentEnvironment, string>> = {
  local: "Local development",
  staging: "Staging",
  production: "Production",
};

const NETWORK_LABELS: Readonly<Record<SolanaNetwork, string>> = {
  localnet: "Local Solana ledger",
  devnet: "Solana Devnet",
  testnet: "Solana Testnet",
  "mainnet-beta": "Solana Mainnet",
};

function parseValue<T extends string>(
  raw: unknown,
  labels: Readonly<Record<T, string>>,
  missingWarning: string,
  invalidWarning: string,
): { value: T | null; label: string; configured: boolean; warning?: string } {
  if (typeof raw !== "string" || raw.trim() === "") {
    return { value: null, label: "Not configured", configured: false, warning: missingWarning };
  }
  const value = raw.trim() as T;
  if (!Object.prototype.hasOwnProperty.call(labels, value)) {
    return { value: null, label: "Not configured", configured: false, warning: invalidWarning };
  }
  return { value, label: labels[value], configured: true };
}

export function parseRuntimeContext(raw: Readonly<Record<string, unknown>>): RuntimeContext {
  const environment = parseValue(
    raw.VITE_DEPLOYMENT_ENVIRONMENT,
    ENVIRONMENT_LABELS,
    "The deployment environment was not supplied by the release build.",
    "The deployment environment is invalid for this release build.",
  );
  const network = parseValue(
    raw.VITE_SOLANA_NETWORK,
    NETWORK_LABELS,
    "The Solana network was not supplied by the release build.",
    "The Solana network is invalid for this release build.",
  );
  const warnings = [environment.warning, network.warning].filter((warning): warning is string => Boolean(warning));
  return {
    environment: { value: environment.value, label: environment.label, configured: environment.configured },
    network: { value: network.value, label: network.label, configured: network.configured },
    configured: environment.configured && network.configured,
    warnings,
  };
}

export const BROWSER_RUNTIME_CONTEXT = parseRuntimeContext({
  VITE_DEPLOYMENT_ENVIRONMENT: import.meta.env.VITE_DEPLOYMENT_ENVIRONMENT,
  VITE_SOLANA_NETWORK: import.meta.env.VITE_SOLANA_NETWORK,
});
