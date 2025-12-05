use std::{fs, path::PathBuf};

use umm::java::paths::ProjectPaths;
use uuid::Uuid;

fn temp_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("umm-paths-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create temp root");
    root
}

#[test]
fn project_paths_defaults_are_consistent() {
    let root = temp_root();

    let via_new = ProjectPaths::new(root.clone());
    let via_parts = ProjectPaths::from_parts(root.clone(), None, None, None, None, None, None);
    let via_builder = umm::java::paths::project_paths()
        .root_dir(root.clone())
        .build();

    let snapshot = |p: &ProjectPaths| {
        (
            p.root_dir().to_path_buf(),
            p.source_dir().to_path_buf(),
            p.build_dir().to_path_buf(),
            p.test_dir().to_path_buf(),
            p.lib_dir().to_path_buf(),
            p.umm_dir().to_path_buf(),
            p.report_dir().to_path_buf(),
        )
    };

    assert_eq!(snapshot(&via_new), snapshot(&via_parts));
    assert_eq!(snapshot(&via_new), snapshot(&via_builder));

    let _ = fs::remove_dir_all(root);
}
