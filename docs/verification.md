# Verification

Each workspace package inherits the Rust and Clippy lint policy. Generated projects and the template repository use the same core layout, toolchain, license, hooks, and CI policy.

## Local checks

Run the formatting, strict Clippy, target-test, and documentation-test commands in README.md. Documentation tests run separately with `cargo test --doc`. Prepare required tools explicitly and use the locked dependencies.

## CI checks

CI prepares the tools, locked dependencies, and current RustSec advisory database before offline checks. Missing tools, failed checks, invalid reports, and unavailable measurements fail verification.

The example starts at 100% line, function, and region coverage. Preserve those limits and obtain explicit approval for policy changes. Keep application reports scoped to application source, with fixtures, dependencies, and maintenance code measured separately. Compare coverage against the established baseline.

Before integration, review dependency advisories, sources, licenses, lint exceptions, and test changes. Configure and test branch protection and required checks on the hosting service. Follow AGENTS.md for task authorization.

## Review and evidence

Use a new, independent reviewer session for nontrivial changes. Validate critical protections with a failing reproduction and a compilable wrong implementation. Record the tested revision, environment, commands, exits, failures, and remaining checks in the task or pull request.

Retain necessary logs in artifact directories excluded from Git or in CI artifacts before cleaning disposable build output. At milestones, review architecture and documentation consistency, exceptions, test strength, complexity, duplication, and regression data. Report measurements from actual evidence.
