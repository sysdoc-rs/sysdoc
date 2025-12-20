use std::path::PathBuf;

/// Test that the embedded SDD template exists in src/templates
#[test]
fn test_sdd_template_exists() {
    let template_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/templates/sdd-standard-v1.toml");

    assert!(
        template_path.exists(),
        "SDD template should exist at {:?}",
        template_path
    );
}

/// Test that the embedded SDD template can be loaded and parsed
#[test]
fn test_sdd_template_loads() {
    let template_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/templates/sdd-standard-v1.toml");

    let content =
        std::fs::read_to_string(&template_path).expect("Should be able to read template file");

    let _config: toml::Value = toml::from_str(&content).expect("Template should be valid TOML");
}
