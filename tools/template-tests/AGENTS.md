# Template maintenance instructions

Read the repository CONTRIBUTING.md and the exact maintainer-provided development standard before editing this scope. This package is an isolated test workspace, excluded from the shared application workspace and from generated projects.

Preserve existing behavior assertions and the bootstrap parity contract. Derive fixture paths from this package's repository root. Generated projects, controls, incorrect versions, and their target directories must be physically independent; verify the actual executed binary.

Use Cargo's native format, strict Clippy, and test commands. Follow the repository task authorization, preserve quality limits, and validate critical protections with runnable incorrect implementations. A new, independent reviewer session checks changes to test contracts and coverage policy.

Keep diagnostics and source content in English. Use the MIT license consistently. Record raw verification outside source control and report missing platforms or remote checks explicitly.
