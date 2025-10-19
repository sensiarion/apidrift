use oas3::OpenApiV3Spec;
use openapi_diff::matcher::SchemaMatcher;

fn main() {
    // Load test schemas
    let base_content = std::fs::read_to_string("tests/base_test_schema.json").unwrap();
    let current_content = std::fs::read_to_string("tests/current_test_schema.json").unwrap();

    let base: OpenApiV3Spec = oas3::from_json(base_content).unwrap();
    let current: OpenApiV3Spec = oas3::from_json(current_content).unwrap();

    let base_schemas = &base.components.as_ref().unwrap().schemas;
    let current_schemas = &current.components.as_ref().unwrap().schemas;

    let matcher = SchemaMatcher::new(base_schemas, current_schemas, &base, &current);
    let results = matcher.match_schemas();

    println!("=== OpenAPI Schema Changes ===\n");
    println!("Found {} schema(s) with changes\n", results.len());

    for result in results {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📋 Schema: {}", result.name);
        println!("🔖 Change Level: {:?}", result.change_level);
        println!();
        
        if result.violations.is_empty() {
            println!("  No violations");
        } else {
            for (i, violation) in result.violations.iter().enumerate() {
                println!("  {}. [{}] {}", i + 1, violation.name(), violation.description());
                println!("      Context: {}", violation.context());
                println!("      Level: {:?}", violation.change_level());
            }
        }
        println!();
    }
}

