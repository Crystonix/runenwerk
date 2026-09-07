//! Guard non-production Runenwerk consumers against stale predecessor RunenGPU paths.

use std::fs;
use std::path::{Path, PathBuf};

fn collect_rust_sources(root: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}

#[test]
fn tests_examples_and_benches_do_not_read_or_import_the_predecessor_gpu_tree() {
    let engine = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = engine.parent().expect("engine must be a workspace member");
    let roots = [
        engine.join("tests"),
        engine.join("examples"),
        engine.join("benches"),
        workspace.join("apps/runenwerk_draw/tests"),
        workspace.join("apps/runenwerk_draw/examples"),
        workspace.join("apps/runenwerk_editor/tests"),
        workspace.join("apps/runenwerk_editor/examples"),
    ];

    let retired = [
        concat!("engine/src/plugins/", "gpu"),
        concat!("src/plugins/", "gpu"),
        concat!("crate::plugins::", "gpu"),
        concat!("engine::plugins::", "gpu"),
    ];
    let mut offenders = Vec::new();

    for root in roots {
        let mut paths = Vec::new();
        collect_rust_sources(&root, &mut paths);
        paths.sort();
        for path in paths {
            let relative = path.strip_prefix(workspace).unwrap_or(&path);
            if relative == Path::new("engine/tests/gpu_cutover_guards.rs")
                || relative == Path::new("engine/tests/gpu_cutover_nonproduction_guards.rs")
            {
                continue;
            }
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            for token in retired {
                if source.contains(token) {
                    offenders.push(format!("{}: {token}", relative.display()));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "non-production consumers retained predecessor RunenGPU source/module paths: {offenders:#?}"
    );
}
