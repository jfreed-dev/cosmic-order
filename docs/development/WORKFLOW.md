# Development Workflow

This document describes the development and release workflow for COSMIC Tweaks.

## Repository Status

| Phase | Branch | Visibility | Status |
|-------|--------|------------|--------|
| Alpha | `alpha` | Private | **Current** |
| Beta | `beta` | Private | Planned |
| Release | `main` | Public | Planned |

## Current Phase: Alpha

During alpha development:

- **Single developer** workflow (no PR reviews required)
- **Direct commits** to `alpha` branch
- **Private repository** - not publicly visible
- Focus on **feature development** and **API exploration**
- Breaking changes are expected

### Commit Workflow

```bash
# Work on alpha branch
git checkout alpha

# Make changes and commit
git add <files>
git commit -m "feat: description of change"

# Push to remote
git push
```

### Commit Message Format

Use conventional commits for clarity:

```
feat: add new feature
fix: fix a bug
docs: update documentation
refactor: code refactoring
test: add tests
chore: maintenance tasks
```

## Planned: Beta Phase

When transitioning to beta:

- [ ] Create `beta` branch from `alpha`
- [ ] Enable branch protection on `beta`
- [ ] Require passing CI checks
- [ ] Begin external testing
- [ ] Repository remains **private**

### Beta Requirements

Before moving to beta:

1. Core features complete (Phases 1-4 of roadmap)
2. No critical bugs
3. Basic test coverage
4. Documentation complete

## Planned: Public Release

When transitioning to public:

- [ ] Move repository to **public** visibility
- [ ] Enable branch protection on `main`
- [ ] Configure security settings:
  - [ ] Dependabot alerts
  - [ ] Secret scanning
  - [ ] Code scanning (optional)
- [ ] Add SECURITY.md
- [ ] Add LICENSE file review
- [ ] Remove any sensitive data from history
- [ ] Create GitHub release with changelog

### Public Repository Checklist

Before going public, verify:

- [ ] No secrets in code or history
- [ ] No AI attribution in commits/comments
- [ ] LICENSE file present (GPL-3.0-only)
- [ ] README is user-friendly
- [ ] Contributing guidelines complete
- [ ] Issue templates configured

## Branch Strategy

```
main (public release)
  │
  └── beta (pre-release testing)
        │
        └── alpha (active development) ← YOU ARE HERE
```

### Merging Up

When ready to promote:

```bash
# Alpha to Beta
git checkout beta
git merge alpha
git push

# Beta to Main (release)
git checkout main
git merge beta
git tag -a v1.0.0 -m "Release v1.0.0"
git push --tags
```

## Quick Reference

```bash
# Current development
cd ~/Repos/cosmic-tweaks
git checkout alpha

# Build and test
just build
just run

# Commit changes
git add -A
git commit -m "feat: your change"
git push
```
