# Release Runner (amd64)

`.github/workflows/release.yml` builds the architecture-specific `.deb` and
publishes the GitHub Release. It targets `runs-on: [self-hosted, linux, X64]`
because the package is amd64 and the homelab's default runner (spark) is
arm64. This guide registers an **amd64 self-hosted runner** as a long-lived
Docker container on an x86_64 host (Thor).

## Scope: repo-level vs org-level

A self-hosted runner is bound to exactly one scope:

- **Repo-level** — the only option for repos under a **personal account**
  (`jfreed-dev` is a User, not an Org). The runner serves one repo. This is
  how the existing `spark-cosmic-order` runner is registered. "Spark serves
  all my repos" really means *spark is a host running one runner instance
  per repo*. The amd64 setup mirrors that: **Thor is the host; run one
  container per repo** that needs an amd64 builder.
- **Org-level** — a single runner usable by many repos via a runner group.
  Requires a **GitHub Organization** (move the repos into one). If you go
  this route, the same image works: pass an org `RUNNER_URL` and an org
  registration token (and optionally `RUNNER_GROUP`).

## 1. Build the image (once, on Thor)

The build context is `scripts/`. Clone the repo on Thor (or copy
`scripts/Dockerfile.runner` + `scripts/runner-entrypoint.sh`), then:

```bash
docker build -f scripts/Dockerfile.runner -t cosmic-order-runner scripts/
```

The image pins the official `actions/runner` release and verifies its
checksum. To bump the runner version, edit the `RUNNER_VERSION` /
`RUNNER_SHA256` build args (current values from
`gh api repos/actions/runner/releases/latest`).

## Constrained builders (QNAP Container Station — the live Thor setup)

Thor's Container Station **cannot build this image**: its BuildKit unpacks
into a 64 MB `/tmp` tmpfs and the ~280 MB runner extraction overflows it
(`tar` exits 2). Non-interactive `docker build`/`run` over SSH also fail on
a per-user-home wrapper. So on Thor the image is **built elsewhere and
loaded**, and the stack runs it as a prebuilt `image:` (no `build:`):

1. Build on any normal amd64 docker host (e.g. the dev laptop):
   ```bash
   docker build -f scripts/Dockerfile.runner -t cosmic-order-runner:latest scripts/
   ```
2. Ship + load it (these work non-interactively; only `build`/`run` hit the
   wrapper):
   ```bash
   docker save cosmic-order-runner:latest | gzip | ssh thor 'cat > ~/runner.tar.gz'
   ssh thor '<container-station>/bin/docker load -i ~/runner.tar.gz'
   ```
3. Use an `image:`-only compose (drop `build:`, add `pull_policy: never` so
   it never tries a registry for this local-only tag). On Thor this is a
   **Dockge stack** at `/share/appdata/dockge/opt/stacks/cosmic-order-runner/`
   (`compose.yaml` + `.env`); deploy from the Dockge UI.

> **Network note (important — not build-ready yet):** Thor's outbound network
> is currently too unstable for builds. (1) Port 80 (HTTP) is **blocked**, so
> the image uses HTTPS apt mirrors (the `sed` in `Dockerfile.runner`). (2) Even
> on 443, **bulk parallel downloads fail intermittently** (`Unable to connect
> …:443` on most packages of an `apt-get install`), so dependency installs and
> `cargo` crate fetches fail — a single connection works (curl 200, ~6.5 s slow
> first-connect) but many-connection workloads don't. (3) The Actions long-poll
> drops every ~15–20 min, so the runner flaps `offline`; `docker restart
> cosmic-order-runner` reconnects it and it picks up queued jobs. Net: the
> runner registers and runs jobs, but **builds fail until Thor's WAN/outbound
> is stabilized**. Until then, cut releases locally (`just release VERSION`),
> and CI is pinned to the spark runner. Re-test with `gh workflow run
> runner-smoke.yml` after fixing the network.

## 2. Register a runner for this repo

Mint a short-lived registration token from any machine with `gh` + admin on
the repo (e.g. your laptop):

```bash
gh api -X POST repos/jfreed-dev/cosmic-order/actions/runners/registration-token --jq .token
```

Then deploy with Compose (recommended — reboot-surviving, and the token
stays out of the config). With the three build files staged in one
directory on Thor:

```bash
echo 'RUNNER_TOKEN=<token-from-above>' > .env
docker compose -f compose.runner.yaml up -d --build
```

The named volume persists the registration, so later `up`s (and reboots)
reconnect without a token — leaving the now-expired token in `.env` is
harmless. Equivalent plain `docker run`, if you'd rather not use Compose:

```bash
docker run -d --name cosmic-order-runner \
  --restart unless-stopped \
  -e RUNNER_URL=https://github.com/jfreed-dev/cosmic-order \
  -e RUNNER_TOKEN=<token-from-above> \
  -e RUNNER_NAME=thor-cosmic-order \
  -e RUNNER_LABELS=thor \
  -v cosmic-order-runner:/home/runner \
  cosmic-order-runner
```

The runner self-assigns the default labels `self-hosted, Linux, X64` (what
`release.yml` and `ci.yml` match on); `RUNNER_LABELS=thor` adds a host tag
by convention, mirroring spark's `spark` label. Per the chosen setup the
runner is **shared with CI** — it's eligible for `ci.yml` jobs too, which is
harmless (those checks are arch-agnostic) and just adds capacity.

> **Sizing:** the libcosmic/iced build is memory-hungry. Give the container
> at least 4 GB RAM (8 GB comfortable) and ~10 GB free disk for the cargo
> registry + `target`. On Container Station, set this on the container.

## 3. Verify and use

```bash
gh api repos/jfreed-dev/cosmic-order/actions/runners \
  --jq '.runners[] | {name,status,labels:[.labels[].name]}'
```

Once it shows `online`/`idle`, cut a release the normal way — push a `v*`
tag (see [WORKFLOW.md](WORKFLOW.md) / `just release`) and `release.yml` will
build and publish on this runner instead of queuing.

## Adding another repo (personal account)

Repo-level runners aren't shared, so each additional repo gets its own
container — same image, different name/URL/token/volume:

```bash
docker run -d --name <repo>-runner --restart unless-stopped \
  -e RUNNER_URL=https://github.com/jfreed-dev/<repo> \
  -e RUNNER_TOKEN=<repo-token> \
  -e RUNNER_NAME=thor-<repo> -e RUNNER_LABELS=thor \
  -v <repo>-runner:/home/runner \
  cosmic-order-runner
```

## Maintenance

- **Update the runner agent:** rebuild the image with a new
  `RUNNER_VERSION`/`RUNNER_SHA256`, then recreate the container. (GitHub
  also auto-updates the agent in place for minor releases.)
- **Remove a runner cleanly:** `docker stop` triggers a best-effort
  deregister. If the original token has expired, prune the stale entry from
  the repo's Settings → Actions → Runners (or `gh api -X DELETE
  repos/jfreed-dev/cosmic-order/actions/runners/<id>`).
