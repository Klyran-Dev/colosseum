#!/bin/sh

set -eu

contract_dir=$(mktemp -d)
cleanup() {
    rm -rf "$contract_dir"
}
trap cleanup EXIT HUP INT TERM

render() {
    VITE_DEPLOYMENT_ENVIRONMENT="$1" VITE_OIDC_ISSUER="$2" \
        node scripts/render_web_nginx_config.mjs \
        ops/nginx/nginx.conf "$contract_dir/$3"
}

render local "" local.conf
if ! grep -Fq "connect-src 'self';" "$contract_dir/local.conf"; then
    echo "local CSP did not remain same-origin" >&2
    exit 1
fi
if grep -Fq "__OIDC_CONNECT_SOURCE__" "$contract_dir/local.conf"; then
    echo "local CSP contains an unresolved OIDC placeholder" >&2
    exit 1
fi

render staging "https://identity.example.test/" staging.conf
if ! grep -Fq "connect-src 'self' https://identity.example.test;" "$contract_dir/staging.conf"; then
    echo "staging CSP did not contain the exact configured issuer origin" >&2
    exit 1
fi
if grep -Eq "connect-src 'self' https:([;[:space:]])" "$contract_dir/staging.conf"; then
    echo "staging CSP contains a scheme-wide HTTPS source" >&2
    exit 1
fi

if VITE_DEPLOYMENT_ENVIRONMENT=staging VITE_OIDC_ISSUER=http://identity.example.test \
    node scripts/render_web_nginx_config.mjs ops/nginx/nginx.conf "$contract_dir/unsafe.conf" >/dev/null 2>&1; then
    echo "unsafe non-local OIDC issuer was accepted" >&2
    exit 1
fi

echo "web CSP origin contract passed"
