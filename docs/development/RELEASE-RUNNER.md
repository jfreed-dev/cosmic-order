# Release Runner (amd64)

`.github/workflows/release.yml` builds the architecture-specific `.deb` and
publishes the GitHub Release. It targets `runs-on: [self-hosted, linux, X64]`
because the package is amd64 and the default self-hosted fleet is arm64.

## Fleet layout (2026-09-27)

The homelab fleet moved off spark and Thor. Current hosts:

| Host | Arch | Serves |
|---|---|---|
| thalamus (Mac, OrbStack) | arm64 | 4 freed-dev-llc org runners + repo runners for personal repos |
| cerebrum (Mac) | arm64 macOS | 2 org runners (native, launchd) |
| mesh (VPS) | x86_64 | 2 org runners + this repo's amd64 runner |

This repo is under a **personal account**, so org runners can't serve it — it
keeps **repo-level runners**: `thalamus-cosmic-order` (arm64, what `ci.yml`
pins via the `thalamus` label) and `mesh-cosmic-order` (amd64, what
`release.yml` and `runner-smoke.yml` match via the auto `X64` label).

All runners are Docker containers built from the shared `fleet-gh-runner`
image (ubuntu-noble + build tools + node 22 + rust + just), non-ephemeral:
they register once and persist. Compose stacks live in `~/gh-runners/` on each
host (`docker-compose.yml` documents the exact services).

## Cutting a release (tag-triggered)

```bash
git tag -a v0.18.0 -m "v0.18.0" && git push origin v0.18.0
```

The tag push triggers `release.yml`, which builds the amd64 `.deb` on the mesh
runner and publishes a GitHub Release with notes pulled from `CHANGELOG.md`.
Bump the version first (`Cargo.toml`, `Cargo.lock`, `debian/changelog`, the
`SECURITY.md` Supported Versions table) and promote the CHANGELOG
`[Unreleased]` section — see the checklist in
[WORKFLOW.md](WORKFLOW.md#release-checklist). Probe the amd64 runner anytime
with `gh workflow run runner-smoke.yml`.

## Operations

- **Check runner state:**
  ```bash
  gh api repos/jfreed-dev/cosmic-order/actions/runners \
    --jq '.runners[] | {name,status,labels:[.labels[].name]}'
  ```
- **Restart the stack on a host:** `docker compose -f ~/gh-runners/docker-compose.yml up -d`
  (mesh has a systemd unit `gh-runners.service`; thalamus a LaunchAgent —
  both just do this at boot).
- **Recreating containers** (after editing the compose file) wipes the
  in-container registration: put a fresh token in `~/gh-runners/runner-token.env`
  first. Repo token:
  `gh api -X POST repos/jfreed-dev/cosmic-order/actions/runners/registration-token --jq .token`
- **Remove a runner cleanly:** stop the container, then
  `gh api -X DELETE repos/jfreed-dev/cosmic-order/actions/runners/<id>`.
- **Sizing:** the libcosmic/iced build is memory-hungry — keep at least 4 GB
  RAM (8 GB comfortable) and ~10 GB free disk available to the container.
