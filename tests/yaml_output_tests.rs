use apidrift::diff::{diff_openapi, OpenApiInputFormat, ReportFormat};

#[test]
fn yaml_agent_output_has_expected_shape() {
    let base = std::fs::read_to_string("tests/base_test_schema.json").unwrap();
    let current = std::fs::read_to_string("tests/current_test_schema.json").unwrap();

    let (yaml_output, stats) = diff_openapi(
        &base,
        &current,
        OpenApiInputFormat::Json,
        OpenApiInputFormat::Json,
        false,
        ReportFormat::YamlAgent,
    )
    .unwrap();

    assert!(stats.schemas_with_changes > 0);

    let yaml: serde_yaml::Value = serde_yaml::from_str(&yaml_output).unwrap();
    assert_eq!(
        yaml.get("schema_version")
            .and_then(serde_yaml::Value::as_i64),
        Some(1)
    );
    assert_eq!(
        yaml.get("generator").and_then(serde_yaml::Value::as_str),
        Some("apidrift")
    );

    let report = yaml.get("report").unwrap();
    assert!(report.get("stats").is_some());
    assert!(report.get("grouped_changes").is_some());
    assert!(report.get("schemas").is_none());
    assert!(report.get("routes").is_none());
    assert!(report.get("full_schemas").is_none());

    // YAML should be compact and emoji-free for agent consumption
    assert!(!yaml_output.contains("emoji:"));
}
