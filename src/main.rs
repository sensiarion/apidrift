use oas3::OpenApiV3Spec;
use openapi_diff::matcher;

// TODO propper exit code and CLI wrapper for utlity


fn parse_openapi(path: &str) -> OpenApiV3Spec {
    let openapi_content =
        match std::fs::read_to_string(path)
        {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to read file \"{path}\". Got error: {err}")
            }
        };

    match oas3::from_json(openapi_content) {
        Ok(spec) => spec,
        Err(err) => panic!("Wrong openapi schema \"{path}\". Got error: {err}"),
    }
}

fn main() {
    println!("Hello, world!");

    let base = parse_openapi("/Users/mansur/projects/rust/openapi_diff/base_openapi.json");
    let current = parse_openapi("/Users/mansur/projects/rust/openapi_diff/current_openapi.json");

    // Get schemas from both versions
    let base_schemas = &base.components.as_ref().unwrap().schemas;
    let current_schemas = &current.components.as_ref().unwrap().schemas;

    // Create matcher and compare schemas
    let matcher = matcher::SchemaMatcher::new(base_schemas, current_schemas, &base, &current);
    let results = matcher.match_schemas();

    // Display stats
    println!("\n=== Schema Comparison Stats ===\n");
    println!("Base schemas: {}", base_schemas.len());
    println!("Current schemas: {}", current_schemas.len());
    println!("Schemas with changes: {}", results.len());
    
    // Display results
    println!("\n=== Schema Changes ===\n");
    if results.is_empty() {
        println!("No changes detected.");
    } else {
        for result in results {
            println!("Schema: {}", result.name);
            println!("Change Level: {:?}", result.change_level);
            for diff in &result.differences {
                println!("  - {:?}", diff);
            }
            println!();
        }
    }
}
