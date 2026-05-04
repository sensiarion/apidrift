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

    // Top-level fields: stats, changes
    let top_stats = yaml.get("stats").unwrap();
    assert!(top_stats
        .get("total")
        .and_then(serde_yaml::Value::as_i64)
        .is_some());
    assert!(top_stats
        .get("breaking")
        .and_then(serde_yaml::Value::as_i64)
        .is_some());
    assert!(top_stats
        .get("warning")
        .and_then(serde_yaml::Value::as_i64)
        .is_some());
    assert!(top_stats
        .get("non_breaking")
        .and_then(serde_yaml::Value::as_i64)
        .is_some());

    assert!(yaml.get("changes").is_some());

    // Compact format: no verbose metadata, no emoji, no report wrapper
    assert!(yaml.get("schema_version").is_none());
    assert!(yaml.get("generator").is_none());
    assert!(yaml.get("report").is_none());
    assert!(!yaml_output.contains("emoji"));
    assert!(!yaml_output.contains("change_level_class"));
    assert!(!yaml_output.contains("change_key"));
    assert!(!yaml_output.contains("is_route_change"));
    assert!(!yaml_output.contains("is_schema_grouped"));
    assert!(!yaml_output.contains("null"));

    // Changes use compact field names
    let changes = yaml.get("changes").unwrap().as_sequence().unwrap();
    assert!(!changes.is_empty());
    let first = &changes[0];
    assert!(first.get("desc").is_some());
    assert!(first.get("level").is_some());
}
