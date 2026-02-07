use std::{
    fs,
    path::{Path, PathBuf},
};

use umm::java::{Project, paths::ProjectPaths};
use uuid::Uuid;

fn temp_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("umm-java-resolve-{}", Uuid::new_v4()));
    fs::create_dir_all(root.join("src/pkg1")).expect("create pkg1");
    fs::create_dir_all(root.join("src/pkg2")).expect("create pkg2");
    root
}

fn write_java(path: &PathBuf, package: &str, class_name: &str) {
    let source = format!(
        "package {package};\n\npublic class {class_name} {{\n    public static void main(String[] \
         args) {{}}\n}}\n"
    );
    fs::write(path, source).expect("write java file");
}

fn build_project(root: &Path) -> Project {
    let paths = ProjectPaths::from_parts(
        root.to_path_buf(),
        Some(root.join("src")),
        None,
        None,
        None,
        None,
        None,
    );
    Project::from_paths(paths).expect("build project")
}

#[test]
fn project_discovery_order_is_deterministic() {
    let root = temp_root();
    write_java(&root.join("src/pkg1/Main.java"), "pkg1", "Main");
    write_java(&root.join("src/pkg2/Main.java"), "pkg2", "Main");

    let first = build_project(&root);
    let second = build_project(&root);

    let first_names: Vec<String> = first.files().iter().map(|f| f.proper_name()).collect();
    let second_names: Vec<String> = second.files().iter().map(|f| f.proper_name()).collect();
    assert_eq!(first_names, second_names);
    assert_eq!(first_names, vec!["pkg1.Main".to_string(), "pkg2.Main".to_string()]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn identify_requires_disambiguation_for_duplicate_simple_names() {
    let root = temp_root();
    write_java(&root.join("src/pkg1/Main.java"), "pkg1", "Main");
    write_java(&root.join("src/pkg2/Main.java"), "pkg2", "Main");

    let project = build_project(&root);

    let err = project
        .identify("Main")
        .expect_err("simple name should be ambiguous")
        .to_string();
    assert!(err.contains("Ambiguous file reference 'Main'"));
    assert!(err.contains("pkg1.Main"));
    assert!(err.contains("pkg2.Main"));

    assert!(project.identify("pkg1.Main").is_ok());
    assert!(
        project
            .identify(root.join("src/pkg2/Main.java").to_str().unwrap())
            .is_ok()
    );

    let _ = fs::remove_dir_all(root);
}
