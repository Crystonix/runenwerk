use std::fs;
use std::path::Path;

const RETIRED_AUTHORITY_MARKERS: &[&str] = &[
    "workflow-lifecycle.md",
    "complete-investigation-gate.md",
    "complete-design-gate.md",
    "evidence-quality-taxonomy.md",
    "complete-merge-readiness-gate.md",
    "track-orchestration-routine.md",
    "investigation-routine.md",
    "implementation-routine.md",
    "architecture-governance-review-routine.md",
    "code-refactor-routine.md",
    "docs-refactor-routine.md",
    "roadmap-update-routine.md",
    "phase-completion-drift-check-routine.md",
    "pr-review-routine.md",
    "commit-splitting-routine.md",
    "public-api-review-routine.md",
    "crate-implementation-routine.md",
    "parallel-roadmap-batch-routine.md",
    "routines/README.md",
    "task-cards/README.md",
    "codex-task.md",
    "docs-cleanup-task.md",
    "github-connector-task.md",
    "implementation-task.md",
    "phase-closeout-task.md",
    "review-task.md",
    "roadmap-update-task.md",
    "track-manager-task.md",
    "prompt-templates/implementation-batch.md",
    "authority-model.md",
    "engineering-workflow.md",
];

const TEXT_EXTENSIONS: &[&str] = &[
    "md", "mdx", "ron", "rs", "py", "toml", "yaml", "yml", "json", "ts", "tsx", "js", "mjs", "cjs",
    "sh", "ps1",
];

#[test]
fn current_repository_authority_does_not_reference_retired_workflow_pages() {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask must remain under <repository>/tools/xtask");
    let own_path = repository_root.join("tools/xtask/tests/docs_authority_cutover.rs");
    let mut violations = Vec::new();

    inspect_tree(repository_root, repository_root, &own_path, &mut violations);
    violations.sort();

    assert!(
        violations.is_empty(),
        "current repository authority still references retired workflow artifacts:\n{}",
        violations.join("\n")
    );
}

fn inspect_tree(
    repository_root: &Path,
    directory: &Path,
    own_path: &Path,
    violations: &mut Vec<String>,
) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", directory.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("could not enumerate {}: {error}", directory.display()));
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            if should_skip_directory(repository_root, &path) {
                continue;
            }
            inspect_tree(repository_root, &path, own_path, violations);
            continue;
        }
        if path == own_path || !is_text_authority(&path) {
            continue;
        }

        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if is_retained_historical_authority(repository_root, &path, &text) {
            continue;
        }
        let operational_text = without_markdown_frontmatter(&path, &text);
        for marker in RETIRED_AUTHORITY_MARKERS {
            if operational_text.contains(marker) {
                let relative = path.strip_prefix(repository_root).unwrap_or(&path);
                violations.push(format!("{}: {marker}", relative.display()));
            }
        }
    }
}

fn should_skip_directory(repository_root: &Path, directory: &Path) -> bool {
    let relative = directory.strip_prefix(repository_root).unwrap_or(directory);
    let normalized = relative.to_string_lossy().replace('\\', "/");

    matches!(
        normalized.as_str(),
        ".git" | "target" | "tools/xtask/target"
    ) || normalized.ends_with("/target")
        || normalized.contains("/node_modules")
        || normalized.starts_with("docs-site/src/content/docs/reports")
        || normalized.starts_with("docs-site/src/content/docs/design/archived")
        || normalized.starts_with("docs-site/src/content/docs/design/rejected")
        || normalized.starts_with("docs-site/src/content/docs/design/superseded")
}

fn is_retained_historical_authority(repository_root: &Path, path: &Path, text: &str) -> bool {
    let relative = path.strip_prefix(repository_root).unwrap_or(path);
    let normalized = relative.to_string_lossy().replace('\\', "/");

    if normalized.starts_with("docs-site/src/content/docs/workspace/specs/")
        && path.extension().and_then(|extension| extension.to_str()) == Some("ron")
    {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name.starts_with("pt-")
            || normalized
                == "docs-site/src/content/docs/workspace/specs/templates/phase-implementation-spec.ron"
        {
            return true;
        }
    }

    matches!(
        markdown_frontmatter_status(text),
        Some("superseded" | "archived" | "rejected")
    )
}

fn without_markdown_frontmatter<'a>(path: &Path, text: &'a str) -> &'a str {
    if !matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("md" | "mdx")
    ) {
        return text;
    }

    text.strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map_or(text, |(_, body)| body)
}

fn markdown_frontmatter_status(text: &str) -> Option<&str> {
    let frontmatter = text.strip_prefix("---\n")?.split_once("\n---\n")?.0;
    frontmatter.lines().find_map(|line| {
        line.strip_prefix("status:")
            .map(str::trim)
            .filter(|status| !status.is_empty())
    })
}

fn is_text_authority(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| TEXT_EXTENSIONS.contains(&extension))
}
