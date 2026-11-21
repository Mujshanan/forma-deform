use std::{collections::HashSet, fs, path::Path};

fn declared_dependencies(manifest: &Path) -> HashSet<String> {
    let source = fs::read_to_string(manifest).expect("workspace crate manifest must be readable");
    let mut in_dependency_section = false;
    let mut dependencies = HashSet::new();
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len() - 1];
            in_dependency_section = section == "dependencies"
                || section == "dev-dependencies"
                || section == "build-dependencies"
                || section.ends_with(".dependencies")
                || section.ends_with(".dev-dependencies")
                || section.ends_with(".build-dependencies");
            continue;
        }
        if in_dependency_section {
            if let Some((name, _)) = line.split_once('=') {
                let name = name.trim();
                if !name.is_empty() && !name.starts_with('#') {
                    dependencies.insert(name.trim_matches('"').to_owned());
                }
            }
        }
    }
    dependencies
}

#[test]
fn smart_controller_and_form_deform_core_have_no_direct_dependency_on_each_other() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_dir.parent().and_then(Path::parent).expect("crate is a workspace member");
    let smart_manifest = workspace_root.join("crates/smart-controller-core/Cargo.toml");
    let deform_manifest = workspace_root.join("crates/form-deform-core/Cargo.toml");
    let smart_dependencies = declared_dependencies(&smart_manifest);
    let deform_dependencies = declared_dependencies(&deform_manifest);

    assert!(
        !smart_dependencies.contains("form-deform-core"),
        "smart-controller-core must not depend on form-deform-core"
    );
    assert!(
        !deform_dependencies.contains("smart-controller-core"),
        "form-deform-core must not depend on smart-controller-core"
    );
}
