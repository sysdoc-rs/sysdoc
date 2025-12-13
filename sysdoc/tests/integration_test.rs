use std::path::PathBuf;

fn get_workspace_root() -> PathBuf {
    // Get the workspace root by going up from the manifest directory
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).parent().unwrap().to_path_buf()
}

#[test]
fn test_minimal_sdd_exists() {
    let workspace_root = get_workspace_root();
    let example_path = workspace_root.join("examples");
    assert!(example_path.exists(), "examples directory should exist");
    assert!(
        example_path
            .join("src/minimal-sdd/01-introduction/01.01_purpose.md")
            .exists(),
        "minimal-sdd should have introduction section"
    );
    assert!(
        example_path
            .join("src/minimal-sdd/02-architecture/02.01_overview.md")
            .exists(),
        "minimal-sdd should have architecture section"
    );
    assert!(
        example_path
            .join("src/minimal-sdd/02-architecture/system-diagram.drawio.svg")
            .exists(),
        "minimal-sdd should have diagram"
    );
}

#[test]
fn test_complete_sdd_exists() {
    let workspace_root = get_workspace_root();
    let example_path = workspace_root.join("examples");
    assert!(example_path.exists(), "examples directory should exist");
    assert!(
        example_path
            .join("src/complete-sdd/02-architecture/tables/components.csv")
            .exists(),
        "complete-sdd should have CSV tables"
    );
    assert!(
        example_path
            .join("src/complete-sdd/02-architecture/diagrams/system-context.drawio.svg")
            .exists(),
        "complete-sdd should have diagrams"
    );
    assert!(
        example_path
            .join("src/complete-sdd/03-detailed-design/ui-screenshot.png")
            .exists(),
        "complete-sdd should have PNG images"
    );
}

#[test]
fn test_template_exists() {
    let workspace_root = get_workspace_root();
    let template_path = workspace_root.join("examples");
    assert!(template_path.exists(), "examples directory should exist");
    assert!(
        template_path
            .join("src/templates/DI-IPSC-81435B/01-scope/01.01_identification.md")
            .exists(),
        "template should have scope section"
    );
    assert!(
        template_path
            .join("src/templates/DI-IPSC-81435B/03-software-design/03.01_system-wide-design.md")
            .exists(),
        "template should have software design sections"
    );
}

// TODO: Add tests for actual build functionality once implemented
// #[test]
// fn test_build_minimal_sdd() {
//     let example_path = Path::new("examples/minimal-sdd");
//     let output_path = Path::new("target/test-output/minimal-sdd.docx");
//     let result = build_document(example_path, output_path);
//     assert!(result.is_ok(), "build should succeed");
//     assert!(output_path.exists(), "output file should be created");
// }
