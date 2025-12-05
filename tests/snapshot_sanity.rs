use std::fs;

#[test]
fn snapshots_do_not_contain_rune_wrapper_errors() {
    let snapshots_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    let mut offending = Vec::new();

    if let Ok(entries) = fs::read_dir(&snapshots_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata()
                && meta.is_file()
                && entry.path().extension().and_then(|e| e.to_str()) == Some("snap")
                && let Ok(contents) = fs::read_to_string(entry.path())
                && contents.contains("Failed to execute Rune script")
            {
                offending.push(entry.file_name());
            }
        }
    }

    assert!(offending.is_empty(), "Snapshots still contain wrapper errors: {:?}", offending);
}
