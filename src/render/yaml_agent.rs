use crate::render::report_model::DiffReport;
use crate::render::DiffReportRenderer;
use serde::Serialize;
use std::error::Error;

pub struct YamlAgentRenderer;

#[derive(Serialize)]
struct AgentYamlReport<'a> {
    schema_version: u32,
    generator: &'a str,
    report: SlimReport<'a>,
}

#[derive(Serialize)]
struct SlimReport<'a> {
    stats: &'a crate::render::report_model::Stats,
    grouped_changes: Vec<YamlGroupedChange>,
}

#[derive(Serialize)]
struct YamlGroupedChange {
    change_key: String,
    description: String,
    change_level: String,
    change_level_class: String,
    details: Vec<YamlPropertyCard>,
    schema_names: Vec<String>,
    is_route_change: bool,
    is_schema_grouped: bool,
    changes: Vec<YamlChangeItem>,
    schema_name: Option<String>,
    route_names: Vec<String>,
    route_schema_usage: Vec<YamlRouteSchemaUsage>,
}

#[derive(Serialize)]
struct YamlPropertyCard {
    property_type: String,
    content: String,
}

#[derive(Serialize)]
struct YamlChangeItem {
    description: String,
    change_level: String,
    change_level_class: String,
}

#[derive(Serialize)]
struct YamlRouteSchemaUsage {
    route_name: String,
    usage_type: String,
}

impl YamlAgentRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl DiffReportRenderer for YamlAgentRenderer {
    fn render_report(&self, report: &DiffReport) -> Result<String, Box<dyn Error>> {
        let payload = AgentYamlReport {
            schema_version: 1,
            generator: "apidrift",
            report: SlimReport {
                stats: &report.stats,
                grouped_changes: report
                    .grouped_changes
                    .iter()
                    .map(|gc| YamlGroupedChange {
                        change_key: gc.change_key.clone(),
                        description: gc.description.clone(),
                        change_level: gc.change_level.clone(),
                        change_level_class: gc.change_level_class.clone(),
                        details: gc
                            .details
                            .iter()
                            .map(|d| YamlPropertyCard {
                                property_type: d.property_type.clone(),
                                content: d.content.clone(),
                            })
                            .collect(),
                        schema_names: gc.schema_names.clone(),
                        is_route_change: gc.is_route_change,
                        is_schema_grouped: gc.is_schema_grouped,
                        changes: gc
                            .changes
                            .iter()
                            .map(|c| YamlChangeItem {
                                description: c.description.clone(),
                                change_level: c.change_level.clone(),
                                change_level_class: c.change_level_class.clone(),
                            })
                            .collect(),
                        schema_name: gc.schema_name.clone(),
                        route_names: gc.route_names.clone(),
                        route_schema_usage: gc
                            .route_schema_usage
                            .iter()
                            .map(|u| YamlRouteSchemaUsage {
                                route_name: u.route_name.clone(),
                                usage_type: u.usage_type.clone(),
                            })
                            .collect(),
                    })
                    .collect(),
            },
        };

        serde_yaml::to_string(&payload).map_err(|err| Box::new(err) as Box<dyn Error>)
    }

    fn file_extension(&self) -> &'static str {
        "yaml"
    }
}
