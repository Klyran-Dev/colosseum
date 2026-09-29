#!/bin/sh

set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
cd "$repo_dir"

compose_profiles="--profile cache --profile observability"

docker compose $compose_profiles config --quiet
docker compose $compose_profiles run --rm --no-deps \
    --entrypoint /bin/promtool prometheus \
    check config /etc/prometheus/prometheus.yml
docker compose $compose_profiles run --rm --no-deps \
    --entrypoint /bin/promtool prometheus \
    check rules /etc/prometheus/alert-rules.yml
docker compose $compose_profiles run --rm --no-deps \
    --entrypoint /bin/amtool alertmanager \
    check-config /etc/alertmanager/alertmanager.yml

echo "Prometheus, Alertmanager, and alert-rule configuration are valid"
