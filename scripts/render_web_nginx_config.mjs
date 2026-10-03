import { readFileSync, writeFileSync } from "node:fs";

const templatePath = process.argv[2];
const outputPath = process.argv[3];
if (!templatePath || !outputPath) {
  throw new Error("Usage: render_web_nginx_config.mjs <template> <output>");
}

const environment = (process.env.VITE_DEPLOYMENT_ENVIRONMENT ?? "").trim();
const issuerValue = (process.env.VITE_OIDC_ISSUER ?? "").trim();
let oidcConnectSource = "";

if (issuerValue) {
  const issuer = new URL(issuerValue);
  if (issuer.username || issuer.password || issuer.search || issuer.hash) {
    throw new Error("VITE_OIDC_ISSUER must not contain credentials, query parameters, or a fragment.");
  }
  const loopbackHosts = new Set(["localhost", "127.0.0.1", "[::1]"]);
  const localHttp = issuer.protocol === "http:" && loopbackHosts.has(issuer.hostname);
  if (issuer.protocol !== "https:" && !(environment === "local" && localHttp)) {
    throw new Error("VITE_OIDC_ISSUER must use HTTPS outside local loopback development.");
  }
  oidcConnectSource = ` ${issuer.origin}`;
}

const template = readFileSync(templatePath, "utf8");
if (!template.includes("__OIDC_CONNECT_SOURCE__")) {
  throw new Error("NGINX template has no OIDC CSP placeholder.");
}

const rendered = template.replaceAll("__OIDC_CONNECT_SOURCE__", oidcConnectSource);
if (rendered.includes("__OIDC_CONNECT_SOURCE__") || /connect-src 'self' https:(?:\s|;)/u.test(rendered)) {
  throw new Error("Rendered NGINX CSP is unresolved or overly broad.");
}
writeFileSync(outputPath, rendered, "utf8");
