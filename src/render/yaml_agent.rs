use crate::render::report_model::DiffReport;
use crate::render::DiffReportRenderer;
use serde::Serialize;
use std::error::Error;

pub struct YamlAgentRenderer;

#[derive(Serialize)]
struct CompactReport {
    stats: CompactStats,
    changes: Vec<CompactChange>,
}

#[derive(Serialize)]
struct CompactStats {
    total: usize,
    breaking: usize,
    warning: usize,
    #[serde(rename = "non_breaking")]
    non_breaking: usize,
}

#[derive(Serialize)]
struct CompactChange {
    desc: String,
    level: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    schemas: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    routes: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    items: Vec<CompactChangeItem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    details: Vec<CompactDetail>,
}

#[derive(Serialize)]
struct CompactChangeItem {
    desc: String,
    level: String,
}

#[derive(Serialize)]
struct CompactDetail {
    kind: String,
    value: String,
}

impl YamlAgentRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl DiffReportRenderer for YamlAgentRenderer {
    fn render_report(&self, report: &DiffReport) -> Result<String, Box<dyn Error>> {
        let payload = build_compact_report(report);
        serde_yaml::to_string(&payload).map_err(|err| Box::new(err) as Box<dyn Error>)
    }

    fn file_extension(&self) -> &'static str {
        "yaml"
    }
}

fn build_compact_report(report: &DiffReport) -> CompactReport {
    let stats = CompactStats {
        total: report.stats.total_changes,
        breaking: report.stats.breaking_changes,
        warning: report.stats.warnings,
        non_breaking: report.stats.non_breaking_changes,
    };

    let changes: Vec<CompactChange> = report
        .grouped_changes
        .iter()
        .map(|gc| {
            let level = gc.change_level_class.clone();

            let items: Vec<CompactChangeItem> = gc
                .changes
                .iter()
                .map(|c| CompactChangeItem {
                    desc: c.description.clone(),
                    level: c.change_level_class.clone(),
                })
                .collect();

            let details: Vec<CompactDetail> = gc
                .details
                .iter()
                .map(|d| CompactDetail {
                    kind: d.property_type.clone(),
                    value: d.content.clone(),
                })
                .collect();

            let mut routes: Vec<String> = gc
                .route_schema_usage
                .iter()
                .map(|u| u.route_name.clone())
                .collect();
            if gc.is_route_change {
                routes.extend(gc.schema_names.clone());
            }
            routes.sort();
            routes.dedup();

            let schemas = if gc.is_route_change {
                vec![]
            } else {
                gc.schema_names.clone()
            };

            CompactChange {
                desc: gc.description.clone(),
                level,
                schemas,
                routes,
                items,
                details,
            }
        })
        .collect();

    CompactReport { stats, changes }
}
