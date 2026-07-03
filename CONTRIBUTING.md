# Contributing

Contributions are welcome. Anyone can open issues and pull requests, but merges into the project’s release flow are handled by maintainers.

This guide describes the normal workflow for contributing to `agentc`, the checks expected before opening a pull request, and the commit and signoff requirements enforced in CI.

## Before You Start

For most non-trivial changes, start with an issue. That gives maintainers a chance to confirm the direction before implementation begins, especially for larger features, refactors, or architectural changes.

Smaller fixes and straightforward documentation improvements can go directly to a pull request when appropriate, but if you are unsure, open an issue first.

## Contribution Workflow

The normal workflow is:

1. Open or join an issue describing the change.
2. Fork the repository.
3. Create a branch from `dev` in your fork.
4. Make the change and run the required checks.
5. Open a pull request targeting `dev`.

Contributors should always work from a fork. Direct pushes to the main repository are restricted to maintainers.

## Branching and Pull Requests

Open pull requests against `dev`.

Maintainers handle the project’s release flow through maintainer-controlled merges. Unless a maintainer explicitly tells you otherwise, do not target release branches directly.

When you open a pull request:

- Keep the scope focused and reviewable.
- Link the relevant issue when one exists.
- Include documentation updates when behavior or usage changes.
- Be prepared to revise the PR based on maintainer feedback.

## Commit Messages

This repository uses Conventional Commits, and commit linting runs in both local hooks and CI.

Use commit messages such as:

- `feat: Add HTTP health check documentation`
- `fix: Handle missing runtime configuration`
- `docs: Update standalone deployment guide`
- `test: Add coverage for manifest parsing`

The subject after the type must start with a capital letter. Lowercase subjects will fail commit linting.

PR titles must follow the same format. Pull requests merged into `dev` are squash-merged, and the PR title becomes the final commit message on `dev`.

## Development Checks

Before opening or updating a pull request, run the project checks locally:

```sh
cargo test
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo audit
```

Your commits and PR title must also satisfy the repository’s commit-lint rules.

## Pre-commit Hooks

Using the project’s pre-commit hooks is strongly recommended so formatting, linting, and commit-message checks run before CI.

Install `prek` by following the upstream installation instructions:

https://prek.j178.dev/guide/getting-started

Then install the repository hooks:

```sh
prek install --install-hooks
```

## Documentation

Update the relevant documentation whenever behavior, APIs, configuration, or user-facing workflows change.

If a change affects how users build, configure, deploy, or operate `agentc`, include the documentation update in the same pull request when possible.

## Large Changes

Large features, major refactors, and architectural changes should be discussed with maintainers before implementation starts.

If a change is broad enough to affect project direction, do not assume a completed implementation will be merged simply because it works. Align on the approach first.

## Review and Merge Process

Review requirements are case by case, but one maintainer approval is the general expectation for routine changes.

Maintainers decide when a pull request is ready to merge. Pull requests into `dev` are typically squash-merged using the PR title as the commit message. Promotions through the rest of the release flow happen through maintainer-controlled merges.

## Developer Certificate of Origin

This project uses the Developer Certificate of Origin (DCO). By contributing, you certify that you have the right to submit the work under the project’s license.

Sign off your commits by using `-s` when you commit:

```sh
git commit -s -m "feat: Add standalone runtime docs"
```

No separate CLA is currently required.

## Versioning Expectations

`agentc` is still pre-1.0. Breaking changes may occur before `v1.0.0` without prior notice.

If you are contributing work that depends on a stable public API or stable internal interfaces, check with maintainers first.
