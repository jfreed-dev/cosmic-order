#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# Registers (once) and runs a GitHub Actions runner. Configuration is
# written into the runner's home, so mounting a named volume at
# /home/runner lets the container restart without re-registering.
#
# Works at either scope (the URL + token decide which):
#   - Repo level (personal accounts): RUNNER_URL is a repo URL.
#   - Org level (requires a GitHub org): RUNNER_URL is an org URL, plus an
#     optional RUNNER_GROUP.
#
# Required env (first run only):
#   RUNNER_URL    repo or org URL, e.g. https://github.com/jfreed-dev/cosmic-order
#   RUNNER_TOKEN  short-lived registration token. Mint with one of:
#     gh api -X POST repos/jfreed-dev/cosmic-order/actions/runners/registration-token --jq .token
#     gh api -X POST orgs/<org>/actions/runners/registration-token --jq .token
#
# Optional env:
#   RUNNER_NAME    runner display name (default: container hostname)
#   RUNNER_GROUP   org runner group (org-level only)
#   RUNNER_LABELS  extra comma-separated labels. The defaults
#                  (self-hosted, Linux, X64) are always added by the runner,
#                  which is all release.yml / ci.yml match on. A host label
#                  like "thor" (mirroring spark's) is a useful convention.
set -euo pipefail

cd /home/runner/actions-runner

if [[ ! -f .runner ]]; then
    if [[ -z "${RUNNER_URL:-}" || -z "${RUNNER_TOKEN:-}" ]]; then
        echo "First run requires RUNNER_URL and RUNNER_TOKEN env vars." >&2
        exit 1
    fi

    config_args=(
        --unattended
        --replace
        --url "${RUNNER_URL}"
        --token "${RUNNER_TOKEN}"
        --name "${RUNNER_NAME:-$(hostname)}"
        --work _work
    )
    if [[ -n "${RUNNER_GROUP:-}" ]]; then
        config_args+=(--runnergroup "${RUNNER_GROUP}")
    fi
    if [[ -n "${RUNNER_LABELS:-}" ]]; then
        config_args+=(--labels "${RUNNER_LABELS}")
    fi

    ./config.sh "${config_args[@]}"
fi

# Best-effort clean deregistration when the container is stopped. The
# registration token may already have expired, in which case the runner is
# simply left showing "offline" until pruned from the repo/org settings.
cleanup() {
    if [[ -n "${RUNNER_TOKEN:-}" ]]; then
        ./config.sh remove --token "${RUNNER_TOKEN}" || true
    fi
}
trap 'cleanup; exit 0' INT TERM

./run.sh &
wait $!
