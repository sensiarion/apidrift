//! Compare OpenAPI specs from strings and render reports.

use crate::matcher;
use crate::parse_error;
use crate::render::html::HtmlRenderer;
use crate::render::report_model::DiffReport;
use crate::render::yaml_agent::YamlAgentRenderer;
use crate::render::DiffReportRenderer;
use oas3::OpenApiV3Spec;
use serde_path_to_error::deserialize;
use std::path::Path;

/// Input encoding for [`diff_openapi_to_html`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenApiInputFormat {
    Json,
    Yaml,
}

/// Console-friendly counts after a successful diff (schemas / routes).
#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub base_schema_count: usize,
    pub current_schema_count: usize,
    pub schemas_with_changes: usize,
    pub total_routes: usize,
    pub routes_with_changes: usize,
}

/// Output format for diff reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Html,
    YamlAgent,
}

fn parse_openapi_str(
    content: &str,
    format: OpenApiInputFormat,
    label: &str,
) -> Result<OpenApiV3Spec, String> {
    let path = Path::new(label);
    match format {
        OpenApiInputFormat::Json => {
            let mut deserializer = serde_json::Deserializer::from_str(content);
            let parsed: Result<OpenApiV3Spec, _> = deserialize(&mut deserializer);
            parsed.map_err(|err| {
                parse_error::format_openapi_parse_error(
                    path,
                    "json",
                    &err.to_string(),
                    content,
                    Some(&err.path().to_string()),
                )
            })
        }
        OpenApiInputFormat::Yaml => oas3::from_yaml(content).map_err(|err| {
            parse_error::format_openapi_parse_error(path, "yaml", &err.to_string(), content, None)
        }),
    }
}

/// Compare two OpenAPI documents and return the chosen report format plus stats.
pub fn diff_openapi(
    base_content: &str,
    current_content: &str,
    base_format: OpenApiInputFormat,
    current_format: OpenApiInputFormat,
    include_descriptions: bool,
    report_format: ReportFormat,
) -> Result<(String, DiffStats), String> {
    let base = parse_openapi_str(base_content, base_format, "base")?;
    let current = parse_openapi_str(current_content, current_format, "current")?;

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

    let schema_matcher = matcher::SchemaMatcher::new_with_options(
        base_schemas,
        current_schemas,
        &base,
        &current,
        include_descriptions,
    );
    let schema_results = schema_matcher.match_schemas();
    let full_schema_infos = schema_matcher.build_full_schema_infos(&schema_results);

    let route_matcher = matcher::RouteMatcher::new(&base, &current);
    let route_results = route_matcher.match_routes_with_schema_violations(&schema_results);
    let route_infos = route_matcher.get_all_routes_with_schemas();

    let stats = DiffStats {
        base_schema_count: base_schemas.len(),
        current_schema_count: current_schemas.len(),
        schemas_with_changes: schema_results.len(),
        total_routes: route_infos.len(),
        routes_with_changes: route_results.len(),
    };

    let report = DiffReport::from_match_results(
        &schema_results,
        &route_results,
        &route_infos,
        &full_schema_infos,
    );

    let output = match report_format {
        ReportFormat::Html => {
            let renderer = HtmlRenderer::new().map_err(|e| e.to_string())?;
            renderer.render_report(&report).map_err(|e| e.to_string())?
        }
        ReportFormat::YamlAgent => {
            let renderer = YamlAgentRenderer::new();
            renderer.render_report(&report).map_err(|e| e.to_string())?
        }
    };

    Ok((output, stats))
}

/// Compare two OpenAPI documents and return an HTML report plus stats.
pub fn diff_openapi_to_html(
    base_content: &str,
    current_content: &str,
    base_format: OpenApiInputFormat,
    current_format: OpenApiInputFormat,
    include_descriptions: bool,
) -> Result<(String, DiffStats), String> {
    diff_openapi(
        base_content,
        current_content,
        base_format,
        current_format,
        include_descriptions,
        ReportFormat::Html,
    )
}
