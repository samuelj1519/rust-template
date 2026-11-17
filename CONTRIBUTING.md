# Contributing

The repository is a workspace rendered from its own `template/` input. Its core files and quality policy are the same as those of a generated project. `tools/template-tests/`, this guide, and the template workflow are maintainer-only extensions; they do not appear in generated workspaces.

## Core checks

Use the commands in README.md. Changes to shared files belong in `template/` first, then regenerate the repository core using the project name `rust-template`. The bootstrap contract compares every generated core file with the repository files Git does not ignore, including untracked files, and compares two independent generations. Ignored working-tree files are not part of that set.

## Template contracts

```bash
cd tools/template-tests
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked --offline -- -D warnings
cargo test --all-targets --all-features --locked --offline
```

These Rust tests require cargo-generate 0.26.0. They execute real generated projects, including compiler-enforced lint inheritance, isolated binaries, doctests, privacy exclusions, directory safety, and bootstrap parity. Keep the three example behavior assertions unchanged. Only migrate obsolete test contracts when the corresponding specification change has explicit approval, and retain a clear protection mapping for review.

## Review and publication

Template maintenance adopts dev-standard 1.0 draft at commit `039dcb29f7c3681a60c8bf9ad8d3e987b2e37b7a`. Read that exact version before governed work. Each generated project selects its own standard. Confirm the workspace, changes, scope, and task authorization, and use a new, independent reviewer session for nontrivial changes.

Follow AGENTS.md for task authorization and quality-policy changes. Use synthetic fixtures and independent source/build directories. Record verification in the task or pull request and store runtime logs in artifact directories excluded from Git.

Normal commits describe one reviewable change in English. Include the concrete result and relevant verification. History rewrites and force pushes require explicit authorization.
