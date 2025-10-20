use crate::matcher::{RouteInfo, SchemaLocation, SchemaReference};
use crate::render::Renderer;
use crate::rules::{MatchResult, RuleViolation};
use crate::ChangeLevel;
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use tera::{Context, Tera};

#[derive(Serialize)]
struct TemplateData {
    schemas: Vec<SchemaData>,
    routes: Vec<RouteData>,
    stats: Stats,
    grouped_changes: Vec<GroupedChange>,
}

#[derive(Serialize)]
struct Stats {
    total_changes: usize,
    breaking_changes: usize,
    warnings: usize,
    non_breaking_changes: usize,
}

#[derive(Serialize)]
struct SchemaData {
    name: String,
    change_level: String,
    change_level_class: String,
    differences: Vec<DifferenceData>,
}

#[derive(Serialize)]
struct RouteData {
    name: String,
    path: String,
    method: String,
    change_level: String,
    change_level_class: String,
    differences: Vec<DifferenceData>,
    request_schemas: Vec<SchemaLinkData>,
    response_schemas: Vec<SchemaLinkData>,
}

#[derive(Serialize)]
struct SchemaLinkData {
    schema_name: String,
    content_type: String,
    location: String,
    status_code: Option<String>,
    has_changes: bool, // true if this schema has changes and can be linked
}

#[derive(Serialize, Clone)]
struct DifferenceData {
    emoji: String,
    description: String,
    change_level: String,
    change_level_class: String,
    details: Vec<PropertyCard>,
}

#[derive(Serialize, Clone)]
struct PropertyCard {
    emoji: String,
    property_type: String,
    content: String,
}

#[derive(Serialize)]
struct GroupedChange {
    change_key: String,
    emoji: String,
    description: String,
    change_level: String,
    change_level_class: String,
    details: Vec<PropertyCard>,
    schema_names: Vec<String>,
    is_route_change: bool, // true if this is a route change, false if schema
}

pub struct HtmlRenderer {
    tera: Tera,
}

impl HtmlRenderer {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // Load templates from the templates directory
        let mut tera = Tera::default();
        let _ = tera.add_raw_template("report.html", include_str!("../../templates/report.html"));

        Ok(Self { tera })
    }

    /// Render HTML report with routes and schemas
    pub fn render_with_routes(
        &self,
        schema_results: &[MatchResult],
        route_results: &[MatchResult],
        route_infos: &[RouteInfo],
    ) -> Result<String, Box<dyn Error>> {
        let data =
            self.convert_to_template_data_with_routes(schema_results, route_results, route_infos);
        let mut context = Context::new();
        context.insert("data", &data);

        let html = self.tera.render("report.html", &context)?;
        Ok(html)
    }

    fn convert_to_template_data_with_routes(
        &self,
        schema_results: &[MatchResult],
        route_results: &[MatchResult],
        route_infos: &[RouteInfo],
    ) -> TemplateData {
        let mut breaking_count = 0;
        let mut warning_count = 0;
        let mut change_count = 0;

        // Group schema and route changes separately and combine
        let mut grouped_changes = self.group_repeating_changes(schema_results);
        let route_grouped = self.group_repeating_changes(route_results);
        grouped_changes.extend(route_grouped);

        // Convert schema results
        let schemas: Vec<SchemaData> = schema_results
            .iter()
            .map(|result| {
                let (change_level, change_level_class) = match result.change_level {
                    ChangeLevel::Breaking => {
                        breaking_count += 1;
                        ("Breaking".to_string(), "breaking".to_string())
                    }
                    ChangeLevel::Warning => {
                        warning_count += 1;
                        ("Warning".to_string(), "warning".to_string())
                    }
                    ChangeLevel::Change => {
                        change_count += 1;
                        ("Change".to_string(), "change".to_string())
                    }
                };

                let differences = result
                    .violations
                    .iter()
                    .map(|violation| self.convert_violation(violation))
                    .collect();

                SchemaData {
                    name: result.name.clone(),
                    change_level,
                    change_level_class,
                    differences,
                }
            })
            .collect();

        // Convert route results
        let routes: Vec<RouteData> = route_results
            .iter()
            .map(|result| {
                let (change_level, change_level_class) = match result.change_level {
                    ChangeLevel::Breaking => {
                        breaking_count += 1;
                        ("Breaking".to_string(), "breaking".to_string())
                    }
                    ChangeLevel::Warning => {
                        warning_count += 1;
                        ("Warning".to_string(), "warning".to_string())
                    }
                    ChangeLevel::Change => {
                        change_count += 1;
                        ("Change".to_string(), "change".to_string())
                    }
                };

                let differences = result
                    .violations
                    .iter()
                    .map(|violation| self.convert_violation(violation))
                    .collect();

                // Find route info for this route
                let parts: Vec<&str> = result.name.split_whitespace().collect();
                let method = parts.get(0).unwrap_or(&"").to_lowercase();
                let path = parts.get(1).unwrap_or(&"");

                let route_info = route_infos
                    .iter()
                    .find(|r| r.method == method && r.path == *path);

                // Get list of schemas with changes for filtering
                let schemas_with_changes: std::collections::HashSet<String> =
                    schema_results.iter().map(|r| r.name.clone()).collect();

                let (request_schemas, response_schemas) = if let Some(info) = route_info {
                    (
                        self.convert_schema_references(
                            &info.request_schemas,
                            &schemas_with_changes,
                        ),
                        self.convert_schema_references(
                            &info.response_schemas,
                            &schemas_with_changes,
                        ),
                    )
                } else {
                    (vec![], vec![])
                };

                RouteData {
                    name: result.name.clone(),
                    path: path.to_string(),
                    method: method.to_uppercase(),
                    change_level,
                    change_level_class,
                    differences,
                    request_schemas,
                    response_schemas,
                }
            })
            .collect();

        TemplateData {
            stats: Stats {
                total_changes: schema_results.len() + route_results.len(),
                breaking_changes: breaking_count,
                warnings: warning_count,
                non_breaking_changes: change_count,
            },
            schemas,
            routes,
            grouped_changes,
        }
    }

    fn convert_schema_references(
        &self,
        refs: &[SchemaReference],
        schemas_with_changes: &std::collections::HashSet<String>,
    ) -> Vec<SchemaLinkData> {
        refs.iter()
            .map(|r| SchemaLinkData {
                schema_name: r.schema_name.clone(),
                content_type: r.content_type.clone(),
                location: match &r.location {
                    SchemaLocation::RequestBody => "Request Body".to_string(),
                    SchemaLocation::Response(_) => "Response".to_string(),
                },
                status_code: match &r.location {
                    SchemaLocation::Response(code) => Some(code.clone()),
                    _ => None,
                },
                has_changes: schemas_with_changes.contains(&r.schema_name),
            })
            .collect()
    }

    fn convert_to_template_data(&self, results: &[MatchResult]) -> TemplateData {
        let mut breaking_count = 0;
        let mut warning_count = 0;
        let mut change_count = 0;

        let grouped_changes = self.group_repeating_changes(results);

        let schemas: Vec<SchemaData> = results
            .iter()
            .map(|result| {
                let (change_level, change_level_class) = match result.change_level {
                    ChangeLevel::Breaking => {
                        breaking_count += 1;
                        ("Breaking".to_string(), "breaking".to_string())
                    }
                    ChangeLevel::Warning => {
                        warning_count += 1;
                        ("Warning".to_string(), "warning".to_string())
                    }
                    ChangeLevel::Change => {
                        change_count += 1;
                        ("Change".to_string(), "change".to_string())
                    }
                };

                let differences = result
                    .violations
                    .iter()
                    .map(|violation| self.convert_violation(violation))
                    .collect();

                SchemaData {
                    name: result.name.clone(),
                    change_level,
                    change_level_class,
                    differences,
                }
            })
            .collect();

        TemplateData {
            stats: Stats {
                total_changes: results.len(),
                breaking_changes: breaking_count,
                warnings: warning_count,
                non_breaking_changes: change_count,
            },
            schemas,
            routes: vec![], // No routes in old method
            grouped_changes,
        }
    }

    fn group_repeating_changes(&self, results: &[MatchResult]) -> Vec<GroupedChange> {
        let mut change_map: HashMap<String, (DifferenceData, Vec<String>, bool)> = HashMap::new();

        // Collect all changes and their schemas
        for result in results {
            // Check if this is a route (has HTTP method prefix)
            let is_route = result.name.starts_with("GET ")
                || result.name.starts_with("POST ")
                || result.name.starts_with("PUT ")
                || result.name.starts_with("DELETE ")
                || result.name.starts_with("PATCH ")
                || result.name.starts_with("HEAD ")
                || result.name.starts_with("OPTIONS ");

            for violation in &result.violations {
                let diff_data = self.convert_violation(violation);
                let key = self.create_change_key(&diff_data);

                let entry = change_map
                    .entry(key)
                    .or_insert_with(|| (diff_data.clone(), Vec::new(), is_route));
                entry.1.push(result.name.clone());

                // If we already have this change, merge the descriptions
                if entry.0.description != diff_data.description {
                    entry.0.description =
                        self.merge_descriptions(&entry.0.description, &diff_data.description);
                }
            }
        }

        // Only group changes that appear in multiple schemas/routes
        let mut grouped: Vec<GroupedChange> = change_map
            .into_iter()
            .filter(|(_, (_, schemas, _))| schemas.len() > 1)
            .map(|(key, (diff, mut schema_names, is_route))| {
                // Remove duplicates from schema names
                schema_names.sort();
                schema_names.dedup();

                GroupedChange {
                    change_key: key,
                    emoji: diff.emoji,
                    description: diff.description,
                    change_level: diff.change_level,
                    change_level_class: diff.change_level_class,
                    details: diff.details,
                    schema_names,
                    is_route_change: is_route,
                }
            })
            .collect();

        // Sort by number of schemas (descending) and then by change level
        grouped.sort_by(|a, b| {
            let count_cmp = b.schema_names.len().cmp(&a.schema_names.len());
            if count_cmp != std::cmp::Ordering::Equal {
                count_cmp
            } else {
                let level_order = |class: &str| match class {
                    "breaking" => 0,
                    "warning" => 1,
                    "change" => 2,
                    _ => 3,
                };
                level_order(&a.change_level_class).cmp(&level_order(&b.change_level_class))
            }
        });

        grouped
    }

    fn merge_descriptions(&self, desc1: &str, desc2: &str) -> String {
        // If both are about the same property, create a combined description
        if desc1.contains("Property Added") && desc2.contains("Required Properties Added") {
            if let Some(prop1) = desc1.strip_prefix("Property Added: ") {
                if desc2.contains(prop1) {
                    return format!("Property Added (Required): {}", prop1);
                }
            }
        } else if desc1.contains("Property Removed")
            && desc2.contains("Required Properties Removed")
        {
            if let Some(prop1) = desc1.strip_prefix("Property Removed: ") {
                if desc2.contains(prop1) {
                    return format!("Property Removed (Required): {}", prop1);
                }
            }
        }

        // If descriptions are the same, return one of them
        if desc1 == desc2 {
            return desc1.to_string();
        }

        // Otherwise, return the more specific one (shorter description usually means more specific)
        if desc1.len() < desc2.len() {
            desc1.to_string()
        } else {
            desc2.to_string()
        }
    }

    fn create_change_key(&self, diff: &DifferenceData) -> String {
        // Extract property names from the description and details
        let mut property_names = Vec::new();

        // Extract from description (e.g., "Property Added: places" -> "places")
        if diff.description.starts_with("Property Added: ") {
            if let Some(prop) = diff.description.strip_prefix("Property Added: ") {
                property_names.push(prop.to_string());
            }
        } else if diff.description.starts_with("Property Removed: ") {
            if let Some(prop) = diff.description.strip_prefix("Property Removed: ") {
                property_names.push(prop.to_string());
            }
        } else if diff.description.starts_with("Property Modified: ") {
            if let Some(prop) = diff.description.strip_prefix("Property Modified: ") {
                property_names.push(prop.to_string());
            }
        }

        // Extract from details (for required properties)
        for detail in &diff.details {
            if detail.property_type == "Required" {
                property_names.push(detail.content.clone());
            }
        }

        // Sort property names to ensure consistent grouping
        property_names.sort();
        let properties_key = property_names.join(",");

        // Create a base change type from the description
        let change_type = if diff.description.contains("Property Added")
            || diff.description.contains("Required Properties Added")
        {
            "property_added".to_string()
        } else if diff.description.contains("Property Removed")
            || diff.description.contains("Required Properties Removed")
        {
            "property_removed".to_string()
        } else if diff.description.contains("Property Modified") {
            "property_modified".to_string()
        } else {
            // For other changes, use the full description
            diff.description.replace(" ", "_").to_lowercase()
        };

        format!(
            "{}:{}:{}",
            change_type, diff.change_level_class, properties_key
        )
    }

    fn convert_violation(&self, violation: &RuleViolation) -> DifferenceData {
        let rule = violation.rule();
        let rule_name = rule.name();
        let description = rule.description();

        // Map rule names to emojis and extract details
        let (emoji, details) = match rule_name {
            // Schema rules
            "SchemaAdded" => ("➕", vec![]),
            "SchemaRemoved" => ("➖", vec![]),
            "TypeChanged" => ("📝", vec![]),
            "RequiredPropertyAdded" => (
                "⚠️",
                vec![PropertyCard {
                    emoji: "🔧".to_string(),
                    property_type: "Required".to_string(),
                    content: description.clone(),
                }],
            ),
            "RequiredPropertyRemoved" => (
                "⚠️",
                vec![PropertyCard {
                    emoji: "🔧".to_string(),
                    property_type: "Optional".to_string(),
                    content: description.clone(),
                }],
            ),
            "PropertyAdded" => ("🔧", vec![]),
            "PropertyRemoved" => ("🔧", vec![]),
            "DescriptionChanged" => ("📄", vec![]),
            "EnumValuesAdded" => ("➕", vec![]),
            "EnumValuesRemoved" => ("➖", vec![]),
            "FormatChanged" => ("🏷️", vec![]),
            "NullableChanged" => ("❓", vec![]),
            "ArrayItemsChanged" => ("📦", vec![]),
            // Route rules
            "RouteAdded" => ("➕", vec![]),
            "RouteRemoved" => ("➖", vec![]),
            "RouteDescriptionChanged" => ("📄", vec![]),
            "RouteSummaryChanged" => ("📝", vec![]),
            "RequiredParameterAdded" => ("⚠️", vec![]),
            "ParameterRemoved" => ("⚠️", vec![]),
            "ResponseStatusAdded" => ("➕", vec![]),
            "ResponseStatusRemoved" => ("➖", vec![]),
            _ => ("❔", vec![]),
        };

        let (change_level, change_level_class) = match rule.change_level() {
            ChangeLevel::Breaking => ("Breaking".to_string(), "breaking".to_string()),
            ChangeLevel::Warning => ("Warning".to_string(), "warning".to_string()),
            ChangeLevel::Change => ("Change".to_string(), "change".to_string()),
        };

        DifferenceData {
            emoji: emoji.to_string(),
            description,
            change_level,
            change_level_class,
            details,
        }
    }
}

impl Renderer for HtmlRenderer {
    fn render(&self, results: &[MatchResult]) -> Result<String, Box<dyn Error>> {
        let data = self.convert_to_template_data(results);
        let mut context = Context::new();
        context.insert("data", &data);

        let html = self.tera.render("report.html", &context)?;
        Ok(html)
    }

    fn file_extension(&self) -> &str {
        "html"
    }
}
