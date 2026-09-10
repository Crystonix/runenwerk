#![forbid(unsafe_code)]

use std::{
    env,
    fmt::Write as _,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::{Duration, Instant},
};

const TOOLING_CARGO_STEPS: &[(&str, &[&str])] = &[
    (
        "tooling fmt",
        &[
            "fmt",
            "--manifest-path",
            "tools/xtask/Cargo.toml",
            "--check",
        ],
    ),
    (
        "tooling tests",
        &[
            "test",
            "--manifest-path",
            "tools/xtask/Cargo.toml",
            "--locked",
        ],
    ),
    (
        "tooling clippy",
        &[
            "clippy",
            "--manifest-path",
            "tools/xtask/Cargo.toml",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    ),
];

const PRODUCT_CARGO_STEPS: &[(&str, &[&str])] = &[
    ("workspace fmt", &["fmt", "--all", "--check"]),
    ("workspace tests", &["test", "--workspace", "--locked"]),
    (
        "workspace clippy",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    ),
];

const RETIRED_PATHS: &[&str] = &[
    "Taskfile.yml",
    "tools/workflow",
    "workflow",
    "workflow.cmd",
    "quiet_editor_gate.sh",
    "quiet_full_gate.sh",
    ".github/workflows/runensdf-transfer-artifact.yml",
    ".github/workflows/issue-133-census.yml",
    ".github/workflows/runengpu-native-conformance.yml",
    "domain/sdf",
    "docs-site/src/content/docs/domain/sdf",
    "docs-site/src/content/docs/workspace/execution-contract-packs",
    "docs-site/src/content/docs/workspace/execution-locks",
    "docs-site/src/content/docs/workspace/track-execution-manifests",
    "docs-site/src/content/docs/workspace/truth-conformance-specs",
    "docs-site/src/content/docs/reports/track-execution-manifests",
    "docs-site/src/content/docs/reports/track-execution-runs",
    "docs-site/src/content/docs/reports/truth-certificates",
    "docs-site/src/content/docs/workspace/roadmap-items.yaml",
    "docs-site/src/content/docs/workspace/roadmap-archive.yaml",
    "docs-site/src/content/docs/workspace/roadmap-deferred.yaml",
    "docs-site/src/content/docs/workspace/production-tracks.yaml",
];

fn main() -> ExitCode {
    let command = env::args().nth(1).unwrap_or_else(|| "help".to_owned());

    let result = match command.as_str() {
        "validate" => validate(),
        "docs" => repository_root().and_then(|root| validate_docs(&root)),
        "audit" => repository_root().and_then(|root| audit_repository(&root)),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown xtask command: {other}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn validate() -> Result<(), String> {
    let validation_started = Instant::now();
    let mut timings = Vec::new();
    let result = (|| {
        let root = repository_root()?;

        for (name, args) in TOOLING_CARGO_STEPS {
            measure_validation_stage(&mut timings, name, || run(&root, "cargo", args))?;
        }

        for (name, args) in PRODUCT_CARGO_STEPS {
            measure_validation_stage(&mut timings, name, || run(&root, "cargo", args))?;
        }

        measure_validation_stage(&mut timings, "docs validation", || validate_docs(&root))?;
        measure_validation_stage(&mut timings, "repository audit", || audit_repository(&root))
    })();

    eprint!(
        "{}",
        format_validation_timings(&timings, validation_started.elapsed())
    );
    result
}

#[derive(Debug, PartialEq, Eq)]
struct ValidationTiming {
    name: &'static str,
    elapsed: Duration,
}

fn measure_validation_stage<F>(
    timings: &mut Vec<ValidationTiming>,
    name: &'static str,
    stage: F,
) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    let started = Instant::now();
    let result = stage();
    timings.push(ValidationTiming {
        name,
        elapsed: started.elapsed(),
    });
    result
}

fn format_validation_timings(timings: &[ValidationTiming], total: Duration) -> String {
    let label_width = timings
        .iter()
        .map(|timing| timing.name.len())
        .chain(std::iter::once("total".len()))
        .max()
        .unwrap_or_default();
    let mut report = String::from("validation timings:\n");
    for timing in timings {
        writeln!(
            report,
            "  {:label_width$}  {:>8.2}s",
            timing.name,
            timing.elapsed.as_secs_f64()
        )
        .expect("writing validation timing to a String cannot fail");
    }
    writeln!(
        report,
        "  {:label_width$}  {:>8.2}s",
        "total",
        total.as_secs_f64()
    )
    .expect("writing validation total to a String cannot fail");
    report
}

fn validate_docs(root: &Path) -> Result<(), String> {
    let script = "tools/docs/validate_docs.py";
    let candidates: &[(&str, &[&str])] = &[
        ("python3", &[script]),
        ("python", &[script]),
        ("py", &["-3", script]),
    ];

    let mut unavailable = Vec::new();
    for (program, args) in candidates {
        match run_status(root, program, args) {
            Ok(true) => return Ok(()),
            Ok(false) => {
                return Err(format!(
                    "documentation validation failed: {} {}",
                    program,
                    args.join(" ")
                ));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => unavailable.push(*program),
            Err(error) => return Err(format!("failed to run {program}: {error}")),
        }
    }

    Err(format!(
        "documentation validation requires Python 3; unavailable commands: {}",
        unavailable.join(", ")
    ))
}

fn audit_repository(root: &Path) -> Result<(), String> {
    for required in [
        "Cargo.toml",
        "Cargo.lock",
        ".cargo/config.toml",
        ".github/workflows/ci.yml",
        ".github/workflows/docs-validation.yml",
        "README.md",
        "AGENTS.md",
        "ARCHITECTURE.md",
        "TESTING.md",
        "tools/checks/ux_lab_terminology.py",
        "docs-site/src/content/docs/workspace/start-here.md",
        "docs-site/src/content/docs/workspace/documentation-structure.md",
        "docs-site/src/content/docs/workspace/planning/roadmap.md",
        "docs-site/src/content/docs/guidelines/dependency-rules.md",
        "docs-site/src/content/docs/reports/closeouts/pt-runensdf-004-internal-sdf-retirement-closeout.md",
    ] {
        if !root.join(required).is_file() {
            return Err(format!(
                "repository audit: missing required file {required}"
            ));
        }
    }

    for retired in RETIRED_PATHS {
        if root.join(retired).exists() {
            return Err(format!(
                "repository audit: retired workflow path must not exist: {retired}"
            ));
        }
    }

    require_text(
        root,
        ".cargo/config.toml",
        "validate = \"run --manifest-path tools/xtask/Cargo.toml --locked -- validate\"",
        "the root Cargo alias must own the canonical baseline",
    )?;
    require_text(
        root,
        ".github/workflows/ci.yml",
        "uses: dornglut/github-workflows/.github/workflows/reusable-rust-cargo-validate.yml@624cb41adeed21a6461eb838bc7330bd0a5079fd",
        "CI must invoke the accepted shared orchestration through an immutable revision",
    )?;
    require_text(
        root,
        ".github/workflows/ci.yml",
        "  pull_request:\n    branches:\n      - main",
        "CI should run for pull requests targeting main",
    )?;
    require_text_count(
        root,
        ".github/workflows/docs-validation.yml",
        "      - 'docs-site/**'",
        2,
        "the documentation build must be path-scoped for pull requests and main pushes",
    )?;
    require_text_count(
        root,
        ".github/workflows/docs-validation.yml",
        "      - '.github/workflows/docs-validation.yml'",
        2,
        "the documentation workflow must validate changes to itself",
    )?;
    require_text(
        root,
        ".github/workflows/docs-validation.yml",
        "pnpm --dir docs-site build",
        "the documentation workflow must own the Astro/Starlight production build",
    )?;
    require_text(
        root,
        "AGENTS.md",
        "cargo validate",
        "the agent entrypoint must name the canonical baseline",
    )?;
    require_text(
        root,
        "AGENTS.md",
        "dornglut/engineering",
        "the agent entrypoint must delegate organization-wide governance to Engineering",
    )?;
    require_text(
        root,
        "TESTING.md",
        "Root-only documentation changes outside that path scope do not automatically trigger Documentation Build",
        "the local validation map must state the maintained documentation-build trigger scope",
    )?;
    require_text(
        root,
        "docs-site/src/content/docs/workspace/start-here.md",
        "https://github.com/dornglut/engineering",
        "the local cold-start entrypoint must route organization-wide governance to Engineering",
    )?;

    forbid_text(
        root,
        ".github/workflows/docs-validation.yml",
        "validate_docs.py",
        "the documentation build must not duplicate baseline documentation validation",
    )?;

    validate_sdf_retirement(root)?;
    validate_sdf_gitlinks(root)?;

    eprintln!("> repository audit passed");
    Ok(())
}

fn validate_sdf_retirement(root: &Path) -> Result<(), String> {
    forbid_text(
        root,
        "Cargo.toml",
        "\"domain/sdf\"",
        "the retired internal SDF package must not return to workspace membership",
    )?;
    forbid_text(
        root,
        "Cargo.lock",
        "name = \"sdf\"",
        "the retired internal SDF package must not return to the lockfile",
    )?;

    let mut files = Vec::new();
    collect_repository_files(root, root, &mut files)?;

    for path in files {
        if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
            let contents = fs::read_to_string(&path).map_err(|error| {
                format!(
                    "repository audit: failed to read manifest {}: {error}",
                    path.display()
                )
            })?;
            if let Some(line) = sdf_manifest_violation(&contents) {
                return Err(format!(
                    "repository audit: retired SDF dependency authority in {}: {}",
                    repository_relative(root, &path)?,
                    line.trim()
                ));
            }
        }

        let relative = repository_relative(root, &path)?;
        if is_product_rust_source(&relative) {
            let contents = fs::read_to_string(&path).map_err(|error| {
                format!(
                    "repository audit: failed to read Rust source {}: {error}",
                    path.display()
                )
            })?;
            if contents.contains("domain/sdf") {
                return Err(format!(
                    "repository audit: Rust source forwards to retired domain/sdf: {relative}"
                ));
            }
        }
    }

    require_text(
        root,
        "docs-site/src/content/docs/workspace/planning/roadmap.md",
        "Runenwerk now contains no `domain/sdf` package",
        "the current roadmap must record the retirement-only cutover",
    )?;
    require_text(
        root,
        "docs-site/src/content/docs/reports/closeouts/pt-runensdf-004-internal-sdf-retirement-closeout.md",
        "zero real code consumers",
        "the permanent closeout must record the census decision gate",
    )
}

fn collect_repository_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "repository audit: failed to inspect {}: {error}",
            directory.display()
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "repository audit: failed to inspect an entry in {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|value| value.to_str());
            if matches!(name, Some(".git" | "target" | "node_modules" | "context")) {
                continue;
            }
            collect_repository_files(root, &path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }

    if directory == root {
        files.sort();
    }
    Ok(())
}

fn is_product_rust_source(relative: &str) -> bool {
    relative.ends_with(".rs")
        && [
            "adapters/",
            "apps/",
            "domain/",
            "engine/",
            "engine_render_macros/",
            "foundation/",
            "net/",
        ]
        .iter()
        .any(|prefix| relative.starts_with(prefix))
}

fn sdf_manifest_violation(contents: &str) -> Option<&str> {
    contents.lines().find(|raw_line| {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        line.starts_with("sdf =")
            || line.starts_with("runen-sdf =")
            || line.starts_with("runen_sdf =")
            || line.contains("package = \"sdf\"")
            || line.contains("package = \"runen-sdf\"")
            || line.contains("domain/sdf")
    })
}

fn validate_sdf_gitlinks(root: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .args(["ls-files", "-s"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("repository audit: failed to inspect git index: {error}"))?;
    if !output.status.success() {
        return Err("repository audit: git ls-files -s failed".to_owned());
    }

    let index = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = index.lines().find(|line| is_sdf_gitlink(line)) {
        return Err(format!(
            "repository audit: retired SDF authority must not return as a gitlink: {line}"
        ));
    }

    let gitmodules = root.join(".gitmodules");
    if gitmodules.is_file() {
        let contents = fs::read_to_string(&gitmodules).map_err(|error| {
            format!(
                "repository audit: failed to read {}: {error}",
                gitmodules.display()
            )
        })?;
        let lowercase = contents.to_ascii_lowercase();
        if lowercase.contains("runen-sdf") || lowercase.contains("domain/sdf") {
            return Err(
                "repository audit: retired SDF authority must not return through .gitmodules"
                    .to_owned(),
            );
        }
    }

    Ok(())
}

fn is_sdf_gitlink(line: &str) -> bool {
    if !line.starts_with("160000 ") {
        return false;
    }
    let path = line.split_once('\t').map_or("", |(_, path)| path);
    let lowercase = path.to_ascii_lowercase();
    lowercase.contains("runen-sdf") || lowercase.contains("domain/sdf")
}

fn repository_relative(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|error| {
            format!(
                "repository audit: failed to relativize {}: {error}",
                path.display()
            )
        })
}

fn require_text(root: &Path, relative: &str, marker: &str, reason: &str) -> Result<(), String> {
    let text = read_text(root, relative)?;
    if text.contains(marker) {
        Ok(())
    } else {
        Err(format!(
            "repository audit: {relative} is missing required marker {marker:?}: {reason}"
        ))
    }
}

fn require_text_count(
    root: &Path,
    relative: &str,
    marker: &str,
    expected: usize,
    reason: &str,
) -> Result<(), String> {
    let text = read_text(root, relative)?;
    let found = text.matches(marker).count();
    if found == expected {
        Ok(())
    } else {
        Err(format!(
            "repository audit: {relative} expected {expected} occurrences of {marker:?}, found {found}: {reason}"
        ))
    }
}

fn forbid_text(root: &Path, relative: &str, marker: &str, reason: &str) -> Result<(), String> {
    let text = read_text(root, relative)?;
    if text.contains(marker) {
        Err(format!(
            "repository audit: {relative} contains forbidden marker {marker:?}: {reason}"
        ))
    } else {
        Ok(())
    }
}

fn read_text(root: &Path, relative: &str) -> Result<String, String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("repository audit: failed to read {relative}: {error}"))
}

fn repository_root() -> Result<PathBuf, String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "xtask must live at tools/xtask".to_owned())?;

    if root.join("Cargo.toml").is_file() {
        Ok(root.to_path_buf())
    } else {
        Err(format!(
            "resolved repository root does not contain Cargo.toml: {}",
            root.display()
        ))
    }
}

fn run(root: &Path, program: &str, args: &[&str]) -> Result<(), String> {
    eprintln!("> {program} {}", args.join(" "));
    match run_status(root, program, args) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("{program} {} failed", args.join(" "))),
        Err(error) => Err(format!(
            "failed to run {program} {}: {error}",
            args.join(" ")
        )),
    }
}

fn run_status(root: &Path, program: &str, args: &[&str]) -> std::io::Result<bool> {
    Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map(|status| status.success())
}

fn print_usage() {
    eprintln!("Runenwerk repository tasks:");
    eprintln!("  cargo validate       required baseline");
    eprintln!("  cargo xtask docs     documentation validation only");
    eprintln!("  cargo xtask audit    deterministic repository audit only");
}

#[cfg(test)]
mod tests {
    use super::{
        ValidationTiming, format_validation_timings, is_product_rust_source, is_sdf_gitlink,
        measure_validation_stage, sdf_manifest_violation,
    };
    use std::time::Duration;

    #[test]
    fn validation_timing_report_preserves_stage_order_and_total() {
        let timings = [
            ValidationTiming {
                name: "tooling fmt",
                elapsed: Duration::from_millis(240),
            },
            ValidationTiming {
                name: "workspace tests",
                elapsed: Duration::from_millis(184_220),
            },
        ];

        assert_eq!(
            format_validation_timings(&timings, Duration::from_millis(264_360)),
            "validation timings:\n  tooling fmt          0.24s\n  workspace tests    184.22s\n  total              264.36s\n"
        );
    }

    #[test]
    fn failed_validation_stage_is_timed_and_keeps_its_error() {
        let mut timings = Vec::new();
        let result = measure_validation_stage(&mut timings, "workspace tests", || {
            Err("workspace tests failed".to_owned())
        });

        assert_eq!(result, Err("workspace tests failed".to_owned()));
        assert_eq!(timings.len(), 1);
        assert_eq!(timings[0].name, "workspace tests");
    }

    #[test]
    fn sdf_manifest_guard_rejects_retired_dependency_forms() {
        assert_eq!(
            sdf_manifest_violation("sdf = { path = \"domain/sdf\" }\n"),
            Some("sdf = { path = \"domain/sdf\" }")
        );
        assert_eq!(
            sdf_manifest_violation("runen-sdf = { git = \"https://example.invalid\" }\n"),
            Some("runen-sdf = { git = \"https://example.invalid\" }")
        );
        assert_eq!(
            sdf_manifest_violation("field = { package = \"sdf\", version = \"0.1\" }\n"),
            Some("field = { package = \"sdf\", version = \"0.1\" }")
        );
    }

    #[test]
    fn sdf_manifest_guard_allows_world_sdf_terminology() {
        assert_eq!(
            sdf_manifest_violation(
                "world_sdf = { path = \"domain/world_sdf\" }\nname = \"world_sdf\"\n"
            ),
            None
        );
    }

    #[test]
    fn product_source_scope_excludes_repository_tooling() {
        assert!(is_product_rust_source("domain/world_sdf/src/lib.rs"));
        assert!(is_product_rust_source("engine/src/lib.rs"));
        assert!(!is_product_rust_source("tools/xtask/src/main.rs"));
        assert!(!is_product_rust_source("docs-site/example.rs"));
    }

    #[test]
    fn sdf_gitlink_guard_is_path_specific() {
        assert!(is_sdf_gitlink(
            "160000 0123456789012345678901234567890123456789 0\tdomain/sdf"
        ));
        assert!(!is_sdf_gitlink(
            "100644 0123456789012345678901234567890123456789 0\tdomain/sdf.txt"
        ));
        assert!(!is_sdf_gitlink(
            "160000 0123456789012345678901234567890123456789 0\ttools/vendor"
        ));
    }
}
