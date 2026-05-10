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

When ready to cut a release:

```bash
git tag -a v1.0.0 -m "Release v1.0.0"
git push --tags
```

Create a GitHub release with a changelog from the tag.

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
