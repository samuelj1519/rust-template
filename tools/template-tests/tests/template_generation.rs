use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

type TestResult = Result<(), Box<dyn Error>>;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct Fixture {
    root: PathBuf,
    parent: PathBuf,
    target: PathBuf,
}

impl Fixture {
    fn new() -> io::Result<Self> {
        let root = std::env::temp_dir().join(format!(
            "rust-template-contract-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root)?;
        let root = root.canonicalize()?;
        let parent = root.join("parent with spaces");
        let target = root.join("isolated build");
        fs::create_dir(&parent)?;
        fs::create_dir(&target)?;
        fs::write(target.join("empty-git-config"), "")?;
        Ok(Self {
            root,
            parent,
            target,
        })
    }

    fn generate(&self, template: &Path, name: &str) -> io::Result<Output> {
        let mut command = cargo();
        command.args(["generate", "--path"]).arg(template).args([
            "--name",
            name,
            "--no-workspace",
            "--vcs",
            "git",
            "--silent",
        ]);
        run(&mut command, &self.parent, &self.target)
    }

    fn check(&self, project: &Path, arguments: &[&str]) -> io::Result<Output> {
        let mut command = cargo();
        command.args(arguments);
        run(&mut command, project, &self.target)
    }

    fn git(&self, project: &Path, arguments: &[&str]) -> io::Result<Output> {
        run(Command::new("git").args(arguments), project, &self.target)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const CLEARED_GIT_ENV: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn template() -> PathBuf {
    repository().join("template")
}

fn cargo() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

fn run(command: &mut Command, directory: &Path, target: &Path) -> io::Result<Output> {
    for key in [
        "CARGO_BUILD_TARGET_DIR",
        "CARGO_BUILD_BUILD_DIR",
        "RUSTFLAGS",
        "RUSTDOCFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_ENCODED_RUSTDOCFLAGS",
        "LLVM_PROFILE_FILE",
    ] {
        command.env_remove(key);
    }
    for key in CLEARED_GIT_ENV {
        command.env_remove(key);
    }
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("CARGO_LLVM_COV_") {
            command.env_remove(key);
        }
    }
    command
        .current_dir(directory)
        .env("RUSTC_WRAPPER", "")
        .env("RUSTC_WORKSPACE_WRAPPER", "")
        .env("CARGO_TARGET_DIR", target)
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TERM_COLOR", "never")
        .env("RUSTUP_AUTO_INSTALL", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", target.join("empty-git-config"))
        .output()
}

fn diagnostic(output: &Output) -> String {
    format!(
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn succeeds(output: &Output) -> io::Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(diagnostic(output)))
    }
}

fn assert_identity(fixture: &Fixture, project: &Path, name: &str) -> TestResult {
    let lock_before = fs::read(project.join("Cargo.lock"))?;
    let manifest = fs::read_to_string(project.join("crates/app/Cargo.toml"))?;
    assert!(
        manifest.contains(&format!("name = \"{name}\"")),
        "{manifest}"
    );
    let lock = String::from_utf8(lock_before.clone())?;
    assert!(lock.contains(&format!("name = \"{name}\"")), "{lock}");
    for maintenance_package in ["rust-template-tools", "rust-template-tests"] {
        assert!(
            !lock.contains(&format!("name = \"{maintenance_package}\"")),
            "{lock}"
        );
    }
    assert_eq!(lock.matches("[[package]]").count(), 1, "{lock}");
    assert!(manifest.contains("license.workspace = true"), "{manifest}");
    let metadata = fixture.check(
        project,
        &[
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--locked",
            "--offline",
        ],
    )?;
    succeeds(&metadata)?;
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout)?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| io::Error::other("Cargo metadata has no package array"))?;
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["name"], name);
    assert_eq!(packages[0]["license"], "MIT");
    let license = fs::read_to_string(project.join("LICENSE"))?;
    assert!(license.starts_with("MIT License\n\nCopyright (c) 2026 samuelj1519\n"));
    assert_eq!(license.as_bytes(), fs::read(template().join("LICENSE"))?);
    let tree = fixture.check(
        project,
        &[
            "tree",
            "--workspace",
            "--all-features",
            "--target",
            "all",
            "--edges",
            "all",
            "--prefix",
            "none",
            "--format",
            "{p}",
            "--locked",
            "--offline",
        ],
    )?;
    succeeds(&tree)?;
    let tree_output = String::from_utf8(tree.stdout)?;
    let actual_path = tree_output
        .trim()
        .strip_prefix(&format!("{name} v0.1.0 ("))
        .and_then(|output| output.strip_suffix(')'))
        .ok_or_else(|| io::Error::other(format!("unexpected dependency tree: {tree_output}")))?;
    assert_eq!(
        Path::new(actual_path).canonicalize()?,
        project.join("crates/app").canonicalize()?
    );
    assert_eq!(fs::read(project.join("Cargo.lock"))?, lock_before);
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let output = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &output)?;
        } else {
            fs::copy(entry.path(), output)?;
        }
    }
    Ok(())
}

fn unignored_files(root: &Path, empty_git_config: &Path) -> io::Result<BTreeMap<PathBuf, Vec<u8>>> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "-c", "-o", "--exclude-standard"]);
    for key in CLEARED_GIT_ENV {
        command.env_remove(key);
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", empty_git_config);
    let output = command.output()?;
    succeeds(&output)?;
    let mut files = BTreeMap::new();
    for entry in output.stdout.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        let relative = PathBuf::from(std::str::from_utf8(entry).map_err(io::Error::other)?);
        let contents = fs::read(root.join(&relative))?;
        files.insert(relative, contents);
    }
    Ok(files)
}

fn generated_files(root: &Path) -> io::Result<BTreeMap<PathBuf, Vec<u8>>> {
    fn collect(
        root: &Path,
        directory: &Path,
        files: &mut BTreeMap<PathBuf, Vec<u8>>,
    ) -> io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if [OsStr::new(".git"), OsStr::new("target")].contains(&entry.file_name().as_os_str()) {
                continue;
            }
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                collect(root, &path, files)?;
            } else {
                let relative = path
                    .strip_prefix(root)
                    .map_err(io::Error::other)?
                    .to_path_buf();
                files.insert(relative, fs::read(path)?);
            }
        }
        Ok(())
    }

    let mut files = BTreeMap::new();
    collect(root, root, &mut files)?;
    Ok(files)
}

fn write_file(root: &Path, relative: &str, contents: &str) -> io::Result<()> {
    let path = root.join(relative);
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("fixture file has no parent"))?;
    fs::create_dir_all(parent)?;
    fs::write(path, contents)
}

fn isolated_test_artifact(
    output: &str,
    project: &Path,
    target: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let messages = output
        .lines()
        .filter(|line| line.starts_with('{'))
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    let artifacts = messages
        .iter()
        .filter(|message| {
            message["reason"] == "compiler-artifact" && message["profile"]["test"] == true
        })
        .collect::<Vec<_>>();
    let [artifact] = artifacts.as_slice() else {
        return Err(io::Error::other(format!(
            "expected one generated test artifact, found {}",
            artifacts.len()
        ))
        .into());
    };
    if artifact["fresh"] != false {
        return Err(io::Error::other("generated test artifact was not freshly compiled").into());
    }
    for (actual, expected) in [
        (
            &artifact["manifest_path"],
            project.join("crates/app/Cargo.toml"),
        ),
        (
            &artifact["target"]["src_path"],
            project.join("crates/app/src/lib.rs"),
        ),
    ] {
        let actual = actual
            .as_str()
            .ok_or_else(|| io::Error::other("generated test artifact has no source path"))?;
        if Path::new(actual).canonicalize()? != expected.canonicalize()? {
            return Err(io::Error::other(format!(
                "generated test artifact source mismatch: {actual}, expected {}",
                expected.display()
            ))
            .into());
        }
    }
    let executable = artifact["executable"]
        .as_str()
        .ok_or_else(|| io::Error::other("generated test artifact has no executable"))?;
    let executable = Path::new(executable).canonicalize()?;
    if !executable.is_file() || !executable.starts_with(target.canonicalize()?) {
        return Err(io::Error::other(format!(
            "generated test executable is outside its isolated target: {}",
            executable.display()
        ))
        .into());
    }
    Ok(executable)
}

fn assert_original_tests(output: &Output) {
    let output = String::from_utf8_lossy(&output.stdout);
    for name in [
        "collapses_internal_whitespace_and_trims_ends",
        "already_normalized_text_stays_put",
        "blank_input_is_empty",
    ] {
        assert!(
            output.contains(&format!("test tests::{name} ... ok")),
            "{output}"
        );
    }
    assert!(output.contains("3 passed; 0 failed; 0 ignored"), "{output}");
}

#[test]
fn generated_project_runs_original_tests_and_all_documentation_tests() -> TestResult {
    let fixture = Fixture::new()?;
    succeeds(&fixture.generate(&template(), "sample-project")?)?;
    let project = fixture.parent.join("sample-project");
    assert_identity(&fixture, &project, "sample-project")?;
    for arguments in [
        vec!["fmt", "--all", "--check"],
        vec![
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--offline",
            "--",
            "-D",
            "warnings",
        ],
    ] {
        succeeds(&fixture.check(&project, &arguments)?)?;
    }
    let tests = fixture.check(
        &project,
        &[
            "test",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--offline",
            "--message-format=json",
        ],
    )?;
    succeeds(&tests)?;
    let executable = isolated_test_artifact(
        &String::from_utf8_lossy(&tests.stdout),
        &project,
        &fixture.target,
    )?;
    assert_original_tests(&tests);
    let replay = run(&mut Command::new(&executable), &project, &fixture.target)?;
    succeeds(&replay)?;
    assert_original_tests(&replay);
    assert!(!project.join("target").exists());
    let doc_arguments = [
        "test",
        "--workspace",
        "--doc",
        "--all-features",
        "--locked",
        "--offline",
    ];
    succeeds(&fixture.check(&project, &doc_arguments)?)?;
    let source = project.join("crates/app/src/lib.rs");
    let mut contents = fs::read_to_string(&source)?;
    contents.push_str(
        "\n/// ```\n/// assert_eq!(2 + 2, 5);\n/// ```\npub fn failing_documentation_example() {}\n",
    );
    fs::write(source, contents)?;
    let failed = fixture.check(&project, &doc_arguments)?;
    assert!(!failed.status.success(), "{}", diagnostic(&failed));
    let output = String::from_utf8_lossy(&failed.stdout);
    assert!(
        output.contains("failing_documentation_example"),
        "{}",
        diagnostic(&failed)
    );
    assert!(
        output.contains("test result: FAILED"),
        "{}",
        diagnostic(&failed)
    );
    assert!(
        output.contains("assertion `left == right` failed"),
        "{}",
        diagnostic(&failed)
    );
    Ok(())
}

#[test]
fn generated_package_inherits_workspace_rust_and_clippy_lints() -> TestResult {
    let inherited = Fixture::new()?;
    succeeds(&inherited.generate(&template(), "inherited-project")?)?;
    let inherited_project = inherited.parent.join("inherited-project");
    let control = Fixture::new()?;
    succeeds(&control.generate(&template(), "unrestricted-control")?)?;
    let control_project = control.parent.join("unrestricted-control");
    let manifest_path = control_project.join("crates/app/Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)?;
    let unrestricted = manifest.replace("[lints]\nworkspace = true\n", "");
    fs::write(manifest_path, &unrestricted)?;
    let original = fs::read_to_string(inherited_project.join("crates/app/src/lib.rs"))?;
    for (command, probe, lint) in [
        (
            "check",
            "\npub fn workspace_unsafe_probe() -> u8 { unsafe { std::ptr::read(&1_u8) } }\n",
            "-F unsafe-code",
        ),
        (
            "clippy",
            "\npub fn workspace_clippy_probe(value: Option<u8>) -> u8 { value.unwrap() }\n",
            "-D clippy::unwrap-used",
        ),
    ] {
        let source = format!("{original}{probe}");
        for project in [&inherited_project, &control_project] {
            fs::write(project.join("crates/app/src/lib.rs"), &source)?;
        }
        let arguments = [
            command,
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--offline",
        ];
        succeeds(&control.check(&control_project, &arguments)?)?;
        let rejected = inherited.check(&inherited_project, &arguments)?;
        assert!(!rejected.status.success(), "{}", diagnostic(&rejected));
        assert!(
            String::from_utf8_lossy(&rejected.stderr).contains(lint),
            "{}",
            diagnostic(&rejected)
        );
    }
    assert_ne!(manifest, unrestricted);
    Ok(())
}

#[test]
fn multiple_names_and_space_paths_have_matching_locked_identity() -> TestResult {
    let fixture = Fixture::new()?;
    for (input, normalized) in [
        ("second-project", "second-project"),
        ("Another Project", "another-project"),
    ] {
        succeeds(&fixture.generate(&template(), input)?)?;
        assert_identity(&fixture, &fixture.parent.join(normalized), normalized)?;
    }
    Ok(())
}

#[test]
fn generated_payload_has_no_maintenance_files_history_or_automatic_hooks() -> TestResult {
    let fixture = Fixture::new()?;
    succeeds(&fixture.generate(&template(), "clean-project")?)?;
    let project = fixture.parent.join("clean-project");
    for forbidden in [
        "tests",
        "tools",
        "scripts",
        "CONTRIBUTING.md",
        "template",
        ".github/workflows/template.yml",
        ".github/coverage-baseline",
        ".handoff",
        ".worktrees",
        "target",
        "docs/internal-maintenance.md",
        "docs/template-adoption.md",
        "docs/template-internals.md",
        "docs/adr",
        "project-checks.toml",
        "cargo-generate.toml",
    ] {
        assert!(!project.join(forbidden).exists(), "exported {forbidden}");
    }
    assert_eq!(
        fs::read(project.join(".github/workflows/ci.yml"))?,
        fs::read(template().join(".github/workflows/ci.yml"))?
    );
    assert_eq!(
        fs::read(project.join(".githooks/pre-commit"))?,
        fs::read(template().join(".githooks/pre-commit"))?
    );
    assert!(
        !fixture
            .git(&project, &["rev-parse", "--verify", "HEAD"])?
            .status
            .success()
    );
    let hook = fixture.git(&project, &["config", "--local", "--get", "core.hooksPath"])?;
    assert!(!hook.status.success(), "{}", diagnostic(&hook));
    assert!(hook.stdout.is_empty(), "{}", diagnostic(&hook));
    Ok(())
}

#[test]
fn synthetic_credentials_inside_payload_are_excluded() -> TestResult {
    let fixture = Fixture::new()?;
    let source = fixture.root.join("synthetic-template");
    copy_directory(&template(), &source)?;
    let excluded = [
        ".env",
        ".aws/credentials",
        ".cargo/credentials",
        ".cargo/credentials.toml",
    ];
    for path in excluded {
        write_file(&source, path, "synthetic secret sentinel\n")?;
    }
    write_file(
        &source,
        ".cargo/config.toml",
        "# synthetic retained configuration\n",
    )?;
    succeeds(&fixture.generate(&source, "safe-project")?)?;
    let project = fixture.parent.join("safe-project");
    for path in excluded {
        assert!(!project.join(path).exists(), "exported {path}");
    }
    assert_eq!(
        fs::read_to_string(project.join(".cargo/config.toml"))?,
        "# synthetic retained configuration\n"
    );
    Ok(())
}

#[test]
fn existing_destination_is_not_overwritten() -> TestResult {
    let fixture = Fixture::new()?;
    let destination = fixture.parent.join("existing-project");
    fs::create_dir(&destination)?;
    write_file(&destination, "keep.txt", "synthetic user data")?;
    let output = fixture.generate(&template(), "existing-project")?;
    assert!(!output.status.success(), "{}", diagnostic(&output));
    assert_eq!(
        fs::read_to_string(destination.join("keep.txt"))?,
        "synthetic user data"
    );
    assert_eq!(fs::read_dir(destination)?.count(), 1);
    Ok(())
}

#[test]
fn template_itself_is_not_overwritten() -> TestResult {
    let fixture = Fixture::new()?;
    let source = fixture.parent.join("source-template");
    copy_directory(&template(), &source)?;
    let manifest = fs::read(source.join("Cargo.toml"))?;
    write_file(&source, "keep.txt", "synthetic source data")?;
    let output = fixture.generate(&source, "source-template")?;
    assert!(!output.status.success(), "{}", diagnostic(&output));
    assert_eq!(fs::read(source.join("Cargo.toml"))?, manifest);
    assert_eq!(
        fs::read_to_string(source.join("keep.txt"))?,
        "synthetic source data"
    );
    Ok(())
}

#[test]
fn destination_symlinks_are_neither_followed_nor_replaced() -> TestResult {
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    #[cfg(windows)]
    use std::os::windows::fs::symlink_dir as symlink;

    for live in [false, true] {
        let fixture = Fixture::new()?;
        let outside = fixture.root.join("outside-target");
        if live {
            fs::create_dir(&outside)?;
            write_file(&outside, "keep.txt", "synthetic outside data")?;
        }
        let destination = fixture.parent.join("linked-project");
        symlink(&outside, &destination)?;
        let output = fixture.generate(&template(), "linked-project")?;
        assert!(!output.status.success(), "{}", diagnostic(&output));
        let actual = fs::read_link(&destination)?;
        assert!(
            actual.is_absolute(),
            "link target is not absolute: {actual:?}"
        );
        assert_eq!(actual.file_name(), outside.file_name());
        assert_eq!(
            actual
                .parent()
                .ok_or_else(|| io::Error::other("link target has no parent"))?
                .canonicalize()?,
            outside
                .parent()
                .ok_or_else(|| io::Error::other("expected link target has no parent"))?
                .canonicalize()?
        );
        if live {
            assert_eq!(actual.canonicalize()?, outside.canonicalize()?);
            assert_eq!(
                fs::read_to_string(outside.join("keep.txt"))?,
                "synthetic outside data"
            );
            assert_eq!(fs::read_dir(outside)?.count(), 1);
        } else {
            assert!(!outside.exists());
        }
    }
    Ok(())
}

#[test]
fn native_name_normalization_stays_within_parent() -> TestResult {
    let fixture = Fixture::new()?;
    succeeds(&fixture.generate(&template(), "../escape")?)?;
    let names = fs::read_dir(&fixture.parent)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<io::Result<Vec<_>>>()?;
    assert_eq!(names, [OsStr::new("escape")]);
    assert!(!fixture.root.join("escape").exists());
    assert_identity(&fixture, &fixture.parent.join("escape"), "escape")?;
    Ok(())
}

#[test]
fn parent_workspace_is_not_automatically_modified() -> TestResult {
    let fixture = Fixture::new()?;
    let workspace = "[workspace]\nresolver = \"3\"\nmembers = []\n";
    write_file(&fixture.parent, "Cargo.toml", workspace)?;
    succeeds(&fixture.generate(&template(), "nested-project")?)?;
    assert_eq!(
        fs::read_to_string(fixture.parent.join("Cargo.toml"))?,
        workspace
    );
    assert_identity(
        &fixture,
        &fixture.parent.join("nested-project"),
        "nested-project",
    )?;
    Ok(())
}

#[test]
fn rendered_template_reproduces_repository_common_files() -> TestResult {
    let fixture = Fixture::new()?;
    succeeds(&fixture.generate(&template(), "rust-template")?)?;
    let project = fixture.parent.join("rust-template");
    assert_identity(&fixture, &project, "rust-template")?;
    let files = generated_files(&project)?;
    assert!(!files.is_empty());
    let mut repository_files =
        unignored_files(&repository(), &fixture.target.join("empty-git-config"))?;
    repository_files.retain(|relative, _| {
        !relative.starts_with("template")
            && !relative.starts_with("tools/template-tests")
            && relative != Path::new("CONTRIBUTING.md")
            && relative != Path::new(".github/workflows/template.yml")
    });
    assert_eq!(
        repository_files.keys().collect::<Vec<_>>(),
        files.keys().collect::<Vec<_>>(),
        "repository and rendered template have different common files"
    );
    for (relative, contents) in files {
        let actual = &repository_files[&relative];
        assert_eq!(
            actual,
            &contents,
            "repository common file drift: {}",
            relative.display()
        );
    }
    Ok(())
}

#[test]
fn independent_generations_have_identical_complete_file_trees() -> TestResult {
    for name in ["rust-template", "repeatable-project"] {
        let first = Fixture::new()?;
        let second = Fixture::new()?;
        succeeds(&first.generate(&template(), name)?)?;
        succeeds(&second.generate(&template(), name)?)?;
        let first_project = first.parent.join(name);
        let second_project = second.parent.join(name);
        assert_identity(&first, &first_project, name)?;
        assert_identity(&second, &second_project, name)?;
        assert_eq!(
            generated_files(&first_project)?,
            generated_files(&second_project)?,
            "same-name generation changed with the working directory for {name}"
        );
    }
    Ok(())
}

#[test]
fn isolated_artifact_check_rejects_other_sources_and_targets() -> TestResult {
    let fixture = Fixture::new()?;
    let project = fixture.parent.join("synthetic-project");
    write_file(&project, "crates/app/Cargo.toml", "synthetic manifest")?;
    write_file(&project, "crates/app/src/lib.rs", "synthetic source")?;
    write_file(
        &fixture.target,
        "synthetic-executable",
        "synthetic artifact",
    )?;
    let executable = fixture.target.join("synthetic-executable");
    let artifact = serde_json::json!({
        "reason": "compiler-artifact",
        "profile": { "test": true },
        "fresh": false,
        "manifest_path": project.join("crates/app/Cargo.toml"),
        "target": { "src_path": project.join("crates/app/src/lib.rs") },
        "executable": executable,
    });
    assert_eq!(
        isolated_test_artifact(&artifact.to_string(), &project, &fixture.target)?,
        executable.canonicalize()?
    );
    let mut wrong_manifest = artifact.clone();
    wrong_manifest["manifest_path"] = serde_json::json!(project.join("crates/app/src/lib.rs"));
    let mut wrong_source = artifact.clone();
    wrong_source["target"]["src_path"] = serde_json::json!(project.join("crates/app/Cargo.toml"));
    let mut wrong_executable = artifact;
    wrong_executable["executable"] = serde_json::json!(project.join("crates/app/src/lib.rs"));
    for incorrect in [wrong_manifest, wrong_source, wrong_executable] {
        assert!(
            isolated_test_artifact(&incorrect.to_string(), &project, &fixture.target).is_err(),
            "accepted an artifact from another source or target: {incorrect}"
        );
    }
    Ok(())
}
