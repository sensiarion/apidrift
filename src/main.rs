use oas3::OpenApiV3Spec;
use openapi_diff::matcher;
use openapi_diff::render::{html::HtmlRenderer, Renderer};
use std::fs;
use std::path::Path;

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
    println!("🔍 OpenAPI Diff Tool\n");

    let base = parse_openapi("/Users/mansur/projects/rust/openapi_diff/base_openapi.json");
    let current = parse_openapi("/Users/mansur/projects/rust/openapi_diff/current_openapi.json");

    // Get schemas from both versions
    let base_schemas = &base.components.as_ref().unwrap().schemas;
    let current_schemas = &current.components.as_ref().unwrap().schemas;

    // Create matcher and compare schemas
    let matcher = matcher::SchemaMatcher::new(base_schemas, current_schemas, &base, &current);
    let results = matcher.match_schemas();

    // Display stats
    println!("=== Schema Comparison Stats ===\n");
    println!("Base schemas: {}", base_schemas.len());
    println!("Current schemas: {}", current_schemas.len());
    println!("Schemas with changes: {}", results.len());
    
    // Render to HTML
    println!("\n📄 Generating HTML report...");
    let renderer = HtmlRenderer::new().expect("Failed to create HTML renderer");
    let html_output = renderer.render(&results).expect("Failed to render HTML");
    
    // Write to file
    let output_path = Path::new("openapi_diff_report.html");
    fs::write(output_path, html_output).expect("Failed to write HTML file");
    
    println!("✅ Report generated: {}", output_path.display());
    
    // Open in Chrome
    println!("🌐 Opening report in Chrome...");
    let absolute_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join(output_path)
        .canonicalize()
        .expect("Failed to get canonical path");
    
    match std::process::Command::new("open")
        .arg("-a")
        .arg("Google Chrome")
        .arg(&absolute_path)
        .spawn()
    {
        Ok(_) => println!("✨ Done!"),
        Err(e) => {
            eprintln!("⚠️  Failed to open in Chrome: {}", e);
            println!("Trying default browser...");
            if let Err(e) = std::process::Command::new("open")
                .arg(&absolute_path)
                .spawn()
            {
                eprintln!("⚠️  Failed to open file: {}", e);
                println!("Please open the file manually: {}", absolute_path.display());
            } else {
                println!("✨ Opened in default browser!");
            }
        }
    }
}
