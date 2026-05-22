# Development Workflow

This document describes the development and release workflow for COSMIC ORDER.

## Repository Status

| Branch | Purpose | Status |
|--------|---------|--------|
| `main` | Stable development | **Active** |

## Current Workflow

- **Single maintainer** workflow on `main`
- **Direct commits** to `main` for routine work
- External contributions via fork + pull request
- Focus on **feature development** and **stability**

### Commit Workflow

```bash
# Work on main branch
git checkout main

# Make changes and commit
git add <files>
git commit -m "feat: description of change"

# Push to remote
git push
```

### Commit Message Format

Use conventional commits for clarity:

```text
feat: add new feature
fix: fix a bug
docs: update documentation
refactor: code refactoring
test: add tests
chore: maintenance tasks
```

## Quality Gates

Before pushing, run the pre-commit checks:

```bash
just pre-commit         # fmt-check + clippy + tests
./scripts/health-check.sh --quick   # build checks
```

## Feature Branches

For larger changes, use feature branches:

```bash
git checkout -b feat/my-feature
# develop and test
git checkout main
git merge feat/my-feature
git push
git branch -d feat/my-feature
```

## Releases

Releases are **tag-triggered**: pushing a `v*` tag runs
`.github/workflows/release.yml` on the self-hosted amd64 runner, which builds
the `.deb` and publishes the GitHub Release (notes from `CHANGELOG.md`). See
[RELEASE-RUNNER.md](RELEASE-RUNNER.md).

### Release checklist

1. Bump the version everywhere it is pinned:
   - `Cargo.toml` — `version = "X.Y.Z"`
   - `Cargo.lock` — the `cosmic-order` package entry (or let a build update it)
   - `debian/changelog` — add a new top entry
   - `CHANGELOG.md` — promote `[Unreleased]` to `## [X.Y.Z] — DATE`
   - `SECURITY.md` — bump the Supported Versions table to `X.Y.x`
2. Commit (`chore(release): X.Y.Z`) and push `main`.
3. Tag and push — this builds and publishes:
   ```bash
   git tag -a vX.Y.Z -m "vX.Y.Z" && git push origin vX.Y.Z
   ```
   If a build hits a transient runner-egress blip, re-run it (`gh run rerun <id>`).

If the runner is unavailable, build locally instead: `just release VERSION`
builds the `.deb` in the `ubuntu:noble` Docker image and creates the tag; then
publish with `gh release create vX.Y.Z --notes-file <notes> dist/cosmic-order_*.deb`.

## Distribution Packaging

| Format  | Status        | Notes                                                       |
|---------|---------------|-------------------------------------------------------------|
| `.deb`  | **Shipped**   | `just release-deb VERSION` builds in `ubuntu:noble` Docker. |
| Flatpak | Deferred      | See below.                                                  |

### .deb (primary)

The Debian package targets Ubuntu noble / Pop!_OS. The `debian/`
directory holds `control`, `rules`, `copyright`, `source/format`, and
`changelog`. `just release-deb VERSION` runs `dpkg-buildpackage` inside
a pinned `ubuntu:noble` builder image (`scripts/Dockerfile.deb-builder`)
and lands artifacts in `dist/`. This is the local-build fallback; the
primary release flow is tag-triggered (see the release checklist above).

### Flatpak (deferred)

A Flatpak target is feasible but not yet implemented because COSMIC
ORDER has several capabilities that need careful sandbox planning:

- D-Bus access to UPower, logind, and (optionally) system76-power
- cosmic-config writes to the user config dir and to other COSMIC
  components' config namespaces (e.g. `com.system76.CosmicBackground`
  for the wallpaper apply path)
- `ext-idle-notify-v1` Wayland protocol access
- Spawning installed shell scripts (`launch-fullscreen.sh`,
  `screensaver-ctl.sh`, `cosmic-screensaver.sh`)
- Tool-sync writes to user config dirs of unrelated tools
  (`~/.config/ghostty`, `~/.config/btop`, `~/.config/nvim`, etc.)

Several of these go against the Flatpak sandbox model in ways that
either break the feature or require broad `--filesystem=home` /
`--talk-name=...` permissions. Until that surface area is reduced (or
explicit portal flows are designed for each capability), `.deb` is the
recommended distribution format.

## Contributing

External contributions follow the standard fork-and-PR model. See
[CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Quick Reference

```bash
# Current development
cd ~/Repos/cosmic-order

# Build and test
just
just run

# Quality checks
just pre-commit

# Commit changes
git add <files>
git commit -m "feat: your change"
git push
```
