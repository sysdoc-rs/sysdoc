use std::path::PathBuf;

fn get_workspace_root() -> PathBuf {
    // Get the workspace root by going up from the manifest directory
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).parent().unwrap().to_path_buf()
}

#[test]
fn test_minimal_sdd_exists() {
    let workspace_root = get_workspace_root();
    let example_path = workspace_root.join("examples/minimal-sdd");
    assert!(example_path.exists(), "minimal-sdd example should exist");
    assert!(
        example_path.join("01-introduction/index.md").exists(),
        "minimal-sdd should have introduction"
    );
    assert!(
        example_path.join("02-architecture/index.md").exists(),
        "minimal-sdd should have architecture"
    );
    assert!(
        example_path
            .join("02-architecture/system-diagram.drawio.svg")
            .exists(),
        "minimal-sdd should have diagram"
    );
}

#[test]
fn test_complete_sdd_exists() {
    let workspace_root = get_workspace_root();
    let example_path = workspace_root.join("examples/complete-sdd");
    assert!(example_path.exists(), "complete-sdd example should exist");
    assert!(
        example_path
            .join("02-architecture/tables/components.csv")
            .exists(),
        "complete-sdd should have CSV tables"
    );
    assert!(
        example_path
            .join("02-architecture/diagrams/system-context.drawio.svg")
            .exists(),
        "complete-sdd should have diagrams"
    );
    assert!(
        example_path
            .join("03-detailed-design/01-ui-component/ui-screenshot.png")
            .exists(),
        "complete-sdd should have PNG images"
    );
}

#[test]
fn test_template_exists() {
    let workspace_root = get_workspace_root();
    let template_path = workspace_root.join("examples/templates/DI-IPSC-81435B");
    assert!(
        template_path.exists(),
        "DI-IPSC-81435B template should exist"
    );
    assert!(
        template_path.join("01-scope/index.md").exists(),
        "template should have scope section"
    );
    assert!(
        template_path
            .join("03-software-design/01-system-wide-design/index.md")
            .exists(),
        "template should have nested sections"
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
