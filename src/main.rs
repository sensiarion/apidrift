use apidrift::diff::{diff_openapi, OpenApiInputFormat, ReportFormat};
use clap::{Parser, ValueEnum};
use env_logger::Env;
use std::fs;
use std::path::{Path, PathBuf};

/// Available output formats
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OutputFormat {
    /// Generate an HTML report
    Html,
    /// Generate a YAML report for AI agents
    Yaml,
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

    /// Output report file path
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

    /// Include description-only schema changes in the report
    #[arg(long = "include-descriptions")]
    include_descriptions: bool,
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

fn openapi_format_from_path(path: &Path) -> Result<OpenApiInputFormat, String> {
    match detect_format(path)? {
        "json" => Ok(OpenApiInputFormat::Json),
        "yaml" | "yml" => Ok(OpenApiInputFormat::Yaml),
        _ => unreachable!(),
    }
}

fn read_openapi_file(path: &Path, verbose: bool) -> Result<(String, OpenApiInputFormat), String> {
    if verbose {
        println!("📖 Reading OpenAPI spec from: {}", path.display());
    }

    let openapi_content = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read file \"{}\". Error: {}", path.display(), err))?;

    let format = openapi_format_from_path(path)?;

    if verbose {
        println!(
            "   Detected format: {}",
            match format {
                OpenApiInputFormat::Json => "JSON",
                OpenApiInputFormat::Yaml => "YAML",
            }
        );
    }

    Ok((openapi_content, format))
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

    let (base_content, base_format) = match read_openapi_file(&cli.base_spec, cli.verbose) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("❌ Error reading base specification: {}", err);
            std::process::exit(1);
        }
    };

    let (current_content, current_format) = match read_openapi_file(&cli.current_spec, cli.verbose)
    {
        Ok(v) => v,
        Err(err) => {
            eprintln!("❌ Error reading current specification: {}", err);
            std::process::exit(1);
        }
    };

    if cli.verbose {
        println!("✅ Successfully read both specifications\n");
    }

    let report_format = match cli.format {
        OutputFormat::Html => ReportFormat::Html,
        OutputFormat::Yaml => ReportFormat::YamlAgent,
    };

    let output_path = if cli.format == OutputFormat::Yaml
        && cli.output == PathBuf::from("apidrift_report.html")
    {
        PathBuf::from("apidrift_report.yaml")
    } else {
        cli.output.clone()
    };

    let report_kind = match cli.format {
        OutputFormat::Html => "HTML",
        OutputFormat::Yaml => "YAML",
    };

    println!("\n📄 Generating {} report...", report_kind);
    let (report_output, stats) = match diff_openapi(
        &base_content,
        &current_content,
        base_format,
        current_format,
        cli.include_descriptions,
        report_format,
    ) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("❌ Error: {}", err);
            std::process::exit(1);
        }
    };

    if stats.base_schema_count == 0 {
        eprintln!("⚠️  Warning: Base specification has no schemas defined");
    }
    if stats.current_schema_count == 0 {
        eprintln!("⚠️  Warning: Current specification has no schemas defined");
    }

    println!("=== Schema Comparison Stats ===\n");
    println!("  Base schemas:         {}", stats.base_schema_count);
    println!("  Current schemas:      {}", stats.current_schema_count);
    println!("  Schemas with changes: {}", stats.schemas_with_changes);

    println!("\n=== Route Comparison Stats ===\n");
    println!("  Total routes:         {}", stats.total_routes);
    println!("  Routes with changes:  {}", stats.routes_with_changes);

    // Write to file
    if let Err(err) = fs::write(&output_path, report_output) {
        eprintln!("❌ Error: Failed to write report file: {}", err);
        std::process::exit(1);
    }

    let absolute_path =
        match std::env::current_dir().and_then(|cwd| cwd.join(&output_path).canonicalize()) {
            Ok(path) => path,
            Err(_) => output_path.clone(),
        };

    println!("✅ Report generated: {}", absolute_path.display());

    // Validate flag combination
    if cli.chrome && !cli.open {
        println!("\n⚠️  Warning: --chrome flag requires --open flag to take effect");
    }

    if cli.open && cli.format == OutputFormat::Yaml {
        println!("\n⚠️  Warning: --open is supported only for HTML reports");
    } else if cli.open {
        println!();
        open_in_browser(&absolute_path, cli.chrome);
    }

    println!("\n✨ Done!");
}
