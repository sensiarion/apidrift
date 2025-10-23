use apidrift::matcher;
use apidrift::render::html::HtmlRenderer;
use clap::{Parser, ValueEnum};
use env_logger::Env;
use oas3::OpenApiV3Spec;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Available output formats
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OutputFormat {
    /// Generate an HTML report
    Html,
    // Future: add more (e.g. Json, Markdown, etc.)
    // Json,
}

#[derive(Parser)]
#[command(name = "apidrift")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Compare two OpenAPI specifications and generate a detailed diff report", long_about = None)]
#[command(author = "sensiarion <izertmi@gmail.com>")]
struct Cli {
    /// Path to the base OpenAPI specification file (JSON or YAML format)
    #[arg(value_name = "BASE_SPEC")]
    base_spec: PathBuf,

    /// Path to the current OpenAPI specification file (JSON or YAML format)
    #[arg(value_name = "CURRENT_SPEC")]
    current_spec: PathBuf,

    /// Output HTML report file path
    #[arg(
        short = 'o',
        long = "output",
        value_name = "FILE",
        default_value = "apidrift_report.html"
    )]
    output: PathBuf,

    /// Open the report in browser after generation
    #[arg(long = "open")]
    open: bool,

    /// Open the report in Chrome (requires --open flag)
    #[arg(long = "chrome")]
    chrome: bool,

    /// Enable verbose output
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,

    /// More verbose output
    #[arg(long = "vv")]
    more_verbose: bool,

    /// Output format
    #[arg(
        short = 'f',
        long = "format",
        value_enum,
        default_value = "html",
        value_name = "FORMAT"
    )]
    pub format: OutputFormat,
}

fn detect_format(path: &Path) -> Result<&'static str, String> {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("Unable to determine file format for: {}", path.display()))?;

    match extension.to_lowercase().as_str() {
        "json" => Ok("json"),
        "yaml" | "yml" => Ok("yaml"),
        _ => Err(format!(
            "Unsupported file format '{}'. Supported formats: json, yaml, yml",
            extension
        )),
    }
}

fn parse_openapi(path: &Path, verbose: bool) -> Result<OpenApiV3Spec, String> {
    if verbose {
        println!("📖 Reading OpenAPI spec from: {}", path.display());
    }

    let openapi_content = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read file \"{}\". Error: {}", path.display(), err))?;

    let format = detect_format(path)?;

    if verbose {
        println!("   Detected format: {}", format.to_uppercase());
    }

    match format {
        "json" => oas3::from_json(openapi_content).map_err(|err| {
            format!(
                "Invalid OpenAPI JSON schema in \"{}\". Error: {}",
                path.display(),
                err
            )
        }),
        "yaml" => oas3::from_yaml(openapi_content).map_err(|err| {
            format!(
                "Invalid OpenAPI YAML schema in \"{}\". Error: {}",
                path.display(),
                err
            )
        }),
        _ => unreachable!(),
    }
}

fn open_in_browser(path: &Path, use_chrome: bool) {
    println!("🌐 Opening report in browser...");

    // Try Chrome if requested
    if use_chrome {
        let chrome_result = if cfg!(target_os = "macos") {
            std::process::Command::new("open")
                .arg("-a")
                .arg("Google Chrome")
                .arg(path)
                .spawn()
        } else if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                .args(["/C", "start", "chrome", &path.display().to_string()])
                .spawn()
        } else {
            // Linux/Unix
            std::process::Command::new("google-chrome")
                .arg(path)
                .spawn()
                .or_else(|_| std::process::Command::new("chromium").arg(path).spawn())
                .or_else(|_| {
                    std::process::Command::new("chromium-browser")
                        .arg(path)
                        .spawn()
                })
        };

        match chrome_result {
            Ok(_) => {
                println!("✨ Opened in Chrome!");
                return;
            }
            Err(e) => {
                eprintln!("⚠️  Failed to open Chrome: {}", e);
                println!("Falling back to default browser...");
            }
        }
    }

    // Try default browser using the 'open' crate
    if open::that(path).is_ok() {
        println!("✨ Opened in default browser!");
    } else {
        eprintln!("⚠️  Failed to open browser automatically");
        println!("Please open the file manually: {}", path.display());
    }
}

fn main() {
    let cli = Cli::parse();

    let log_level = match (cli.verbose, cli.more_verbose) {
        (true, false) => "info",
        (true, true) | (false, true) => "debug",
        _ => "error",
    };
    env_logger::init_from_env(Env::default().default_filter_or(log_level));

    println!(
        "🔍 ApiDrift - OpenAPI Diff Tool v{}\n",
        env!("CARGO_PKG_VERSION")
    );

    // Validate input files exist
    if !cli.base_spec.exists() {
        eprintln!(
            "❌ Error: Base specification file does not exist: {}",
            cli.base_spec.display()
        );
        std::process::exit(1);
    }

    if !cli.current_spec.exists() {
        eprintln!(
            "❌ Error: Current specification file does not exist: {}",
            cli.current_spec.display()
        );
        std::process::exit(1);
    }

    // Parse OpenAPI specifications
    if cli.verbose {
        println!("🔄 Parsing OpenAPI specifications...\n");
    }

    let base = match parse_openapi(&cli.base_spec, cli.verbose) {
        Ok(spec) => spec,
        Err(err) => {
            eprintln!("❌ Error parsing base specification: {}", err);
            std::process::exit(1);
        }
    };

    let current = match parse_openapi(&cli.current_spec, cli.verbose) {
        Ok(spec) => spec,
        Err(err) => {
            eprintln!("❌ Error parsing current specification: {}", err);
            std::process::exit(1);
        }
    };

    if cli.verbose {
        println!("✅ Successfully parsed both specifications\n");
    }

    // Get schemas from both versions
    let empty_schemas = Default::default();
    let base_schemas = base
        .components
        .as_ref()
        .map(|c| &c.schemas)
        .unwrap_or(&empty_schemas);
    let current_schemas = current
        .components
        .as_ref()
        .map(|c| &c.schemas)
        .unwrap_or(&empty_schemas);

    if base_schemas.is_empty() {
        eprintln!("⚠️  Warning: Base specification has no schemas defined");
    }
    if current_schemas.is_empty() {
        eprintln!("⚠️  Warning: Current specification has no schemas defined");
    }

    // Create schema matcher and compare schemas
    let schema_matcher =
        matcher::SchemaMatcher::new(base_schemas, current_schemas, &base, &current);
    let schema_results = schema_matcher.match_schemas();
    let full_schema_infos = schema_matcher.build_full_schema_infos(&schema_results);

    // Create route matcher and compare routes
    let route_matcher = matcher::RouteMatcher::new(&base, &current);
    let route_results = route_matcher.match_routes_with_schema_violations(&schema_results);
    let route_infos = route_matcher.get_all_routes_with_schemas();

    // Display stats
    println!("=== Schema Comparison Stats ===\n");
    println!("  Base schemas:         {}", base_schemas.len());
    println!("  Current schemas:      {}", current_schemas.len());
    println!("  Schemas with changes: {}", schema_results.len());

    println!("\n=== Route Comparison Stats ===\n");
    println!("  Total routes:         {}", route_infos.len());
    println!("  Routes with changes:  {}", route_results.len());

    // Render to HTML
    println!("\n📄 Generating HTML report...");
    let renderer = match HtmlRenderer::new() {
        Ok(r) => r,
        Err(err) => {
            eprintln!("❌ Error: Failed to create HTML renderer: {}", err);
            std::process::exit(1);
        }
    };

    let html_output = match renderer.render_with_routes(
        &schema_results,
        &route_results,
        &route_infos,
        &full_schema_infos,
    ) {
        Ok(html) => html,
        Err(err) => {
            eprintln!("❌ Error: Failed to render HTML: {}", err);
            std::process::exit(1);
        }
    };

    // Write to file
    if let Err(err) = fs::write(&cli.output, html_output) {
        eprintln!("❌ Error: Failed to write HTML file: {}", err);
        std::process::exit(1);
    }

    let absolute_path =
        match std::env::current_dir().and_then(|cwd| cwd.join(&cli.output).canonicalize()) {
            Ok(path) => path,
            Err(_) => cli.output.clone(),
        };

    println!("✅ Report generated: {}", absolute_path.display());

    // Validate flag combination
    if cli.chrome && !cli.open {
        println!("\n⚠️  Warning: --chrome flag requires --open flag to take effect");
    }

    // Open in browser if --open flag is set
    if cli.open {
        println!();
        open_in_browser(&absolute_path, cli.chrome);
    }

    println!("\n✨ Done!");
}
