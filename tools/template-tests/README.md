# Template contracts

This isolated package maintains and tests the generator. Its manifest defines a separate workspace.

`cargo test --all-targets --all-features --locked` runs the generation contracts and coverage-report unit tests. The bootstrap contract renders `template/` as `rust-template`, compares its complete core file set and bytes with the repository files Git does not ignore, including untracked files, and compares independent same-name generations. Ignored working-tree files are not part of that set. Extra maintainer files are excluded explicitly from the reverse repository scan.

The generated application remains a single standard-library example. Tests preserve its original three assertions and execute formatting, strict Clippy, target tests, and separate doctests. Negative probes verify real lint inheritance and runtime assertion failures. Sensitive-file and symlink cases use synthetic fixtures.

Application coverage is enforced by the shared CI and starts at 100% for lines, functions, and regions. This test-host package is measured separately because integration tests spawn application processes. Its reports must contain nonempty counters and only its own Rust source files. The comparator uses exact integer ratios and rejects empty or contaminated reports. Record the first measurement as the baseline.

A complete milestone also reviews architecture/documentation consistency, exceptions, test strength, complexity, duplication, and regression data. Review changes to tools, compilers, and measurement scope before updating the baseline.

Feature-branch coverage compares with the published default branch, pull requests compare with their target merge base, and default-branch pushes compare with the preceding publication. A failing compatible baseline remains a failure. A new contract host records a validated first measurement.
