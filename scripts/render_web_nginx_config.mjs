import { readFileSync, writeFileSync } from "node:fs";

const templatePath = process.argv[2];
const outputPath = process.argv[3];
if (!templatePath || !outputPath) {
  throw new Error("Usage: render_web_nginx_config.mjs <template> <output>");
}

const environment = (process.env.VITE_DEPLOYMENT_ENVIRONMENT ?? "").trim();
const issuerValue = (process.env.VITE_OIDC_ISSUER ?? "").trim();
const authServiceValue = (process.env.VITE_AUTH_SERVICE_BASE_URL ?? "").trim();
const connectSources = new Set();
const loopbackHosts = new Set(["localhost", "127.0.0.1", "[::1]"]);
const localAuthServiceHosts = new Set([...loopbackHosts, "auth-service"]);

if (issuerValue) {
  const issuer = new URL(issuerValue);
  if (issuer.username || issuer.password || issuer.search || issuer.hash) {
    throw new Error("VITE_OIDC_ISSUER must not contain credentials, query parameters, or a fragment.");
  }
  const localHttp = issuer.protocol === "http:" && loopbackHosts.has(issuer.hostname);
  if (issuer.protocol !== "https:" && !(environment === "local" && localHttp)) {
    throw new Error("VITE_OIDC_ISSUER must use HTTPS outside local loopback development.");
  }
  connectSources.add(issuer.origin);
}

if (authServiceValue) {
  const authService = new URL(authServiceValue);
  if (authService.username || authService.password || authService.search || authService.hash) {
    throw new Error("VITE_AUTH_SERVICE_BASE_URL must not contain credentials, query parameters, or a fragment.");
  }
  const localHttp = authService.protocol === "http:" && localAuthServiceHosts.has(authService.hostname);
  if (authService.protocol !== "https:" && !(environment === "local" && localHttp)) {
    throw new Error("VITE_AUTH_SERVICE_BASE_URL must use HTTPS outside local loopback development.");
  }
  connectSources.add(authService.origin);
}

const template = readFileSync(templatePath, "utf8");
if (!template.includes("__OIDC_CONNECT_SOURCE__")) {
  throw new Error("NGINX template has no OIDC CSP placeholder.");
}

const rendered = template.replaceAll("__OIDC_CONNECT_SOURCE__", [...connectSources].map((origin) => ` ${origin}`).join(""));
if (rendered.includes("__OIDC_CONNECT_SOURCE__") || /connect-src 'self' https:(?:\s|;)/u.test(rendered)) {
  throw new Error("Rendered NGINX CSP is unresolved or overly broad.");
}
writeFileSync(outputPath, rendered, "utf8");
