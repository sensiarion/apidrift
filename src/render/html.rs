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
    full_schemas: Vec<crate::rules::FullSchemaInfo>,
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
    has_request_schema_changes: bool,
    has_response_schema_changes: bool,
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
    // For single-schema grouping
    is_schema_grouped: bool, // true if this groups multiple changes for one schema
    changes: Vec<ChangeItem>, // list of individual changes (for schema-grouped items)
    // New fields for consolidated display
    schema_name: Option<String>, // The main schema name (for schema-grouped items)
    route_names: Vec<String>, // Routes that use this schema
    route_schema_usage: Vec<RouteSchemaUsage>, // Detailed usage info for each route
}

#[derive(Serialize, Clone)]
struct RouteSchemaUsage {
    route_name: String,
    usage_type: String, // "request" or "response"
    emoji: String, // "📤" for request, "📥" for response
}

#[derive(Serialize, Clone)]
struct ChangeItem {
    emoji: String,
    description: String,
    change_level: String,
    change_level_class: String,
}

pub struct HtmlRenderer {
    tera: Tera,
}

impl HtmlRenderer {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // Load templates from the templates directory
        let mut tera = Tera::default();
        
        // Load main template
        let _ = tera.add_raw_template("report.html", include_str!("../../templates/report.html"));
        
        // Load component templates
        let _ = tera.add_raw_template("components/base_styles.html", include_str!("../../templates/components/base_styles.html"));
        let _ = tera.add_raw_template("components/header.html", include_str!("../../templates/components/header.html"));
        let _ = tera.add_raw_template("components/stats.html", include_str!("../../templates/components/stats.html"));
        let _ = tera.add_raw_template("components/help.html", include_str!("../../templates/components/help.html"));
        let _ = tera.add_raw_template("components/grouped_changes.html", include_str!("../../templates/components/grouped_changes.html"));
        let _ = tera.add_raw_template("components/routes.html", include_str!("../../templates/components/routes.html"));
        let _ = tera.add_raw_template("components/schemas.html", include_str!("../../templates/components/schemas.html"));
        let _ = tera.add_raw_template("components/scripts.html", include_str!("../../templates/components/scripts.html"));

        Ok(Self { tera })
    }

    /// Render HTML report with routes and schemas
    pub fn render_with_routes(
        &self,
        schema_results: &[MatchResult],
        route_results: &[MatchResult],
        route_infos: &[RouteInfo],
        full_schema_infos: &[crate::rules::FullSchemaInfo],
    ) -> Result<String, Box<dyn Error>> {
        let data = self.convert_to_template_data_with_routes(
            schema_results,
            route_results,
            route_infos,
            full_schema_infos,
        );
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
        full_schema_infos: &[crate::rules::FullSchemaInfo],
    ) -> TemplateData {
        let mut breaking_count = 0;
        let mut warning_count = 0;
        let mut change_count = 0;

        // Count individual violations instead of MatchResults
        for result in schema_results.iter().chain(route_results.iter()) {
            for violation in &result.violations {
                match violation.change_level() {
                    ChangeLevel::Breaking => breaking_count += 1,
                    ChangeLevel::Warning => warning_count += 1,
                    ChangeLevel::Change => change_count += 1,
                }
            }
        }

        // Group schema and route changes separately and combine
        let mut grouped_changes = self.group_repeating_changes_with_route_infos(schema_results, route_infos);
        let route_grouped = self.group_repeating_changes_with_route_infos(route_results, route_infos);
        grouped_changes.extend(route_grouped);

        // Convert schema results
        let schemas: Vec<SchemaData> = schema_results
            .iter()
            .map(|result| {
                let (change_level, change_level_class) = match result.change_level {
                    ChangeLevel::Breaking => ("Breaking".to_string(), "breaking".to_string()),
                    ChangeLevel::Warning => ("Warning".to_string(), "warning".to_string()),
                    ChangeLevel::Change => ("Change".to_string(), "change".to_string()),
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

                // Filter out schema violations and detect schema changes
                let mut differences = Vec::new();
                let mut has_request_schema_changes = false;
                let mut has_response_schema_changes = false;

                for violation in &result.violations {
                    let diff = self.convert_violation(violation);
                    let description = &diff.description;
                    
                    if description.contains("Request schema") || description.contains("RequestSchemaViolation") {
                        has_request_schema_changes = true;
                    } else if description.contains("Response schema") || description.contains("ResponseSchemaViolation") {
                        has_response_schema_changes = true;
                    } else {
                        // Only include non-schema violations in differences
                        differences.push(diff);
                    }
                }

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
                    has_request_schema_changes,
                    has_response_schema_changes,
                }
            })
            .collect();

        TemplateData {
            stats: Stats {
                total_changes: breaking_count + warning_count + change_count,
                breaking_changes: breaking_count,
                warnings: warning_count,
                non_breaking_changes: change_count,
            },
            schemas,
            routes,
            grouped_changes,
            full_schemas: full_schema_infos.to_vec(),
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

        // Count individual violations instead of MatchResults
        for result in results {
            for violation in &result.violations {
                match violation.change_level() {
                    ChangeLevel::Breaking => breaking_count += 1,
                    ChangeLevel::Warning => warning_count += 1,
                    ChangeLevel::Change => change_count += 1,
                }
            }
        }

        let grouped_changes = self.group_repeating_changes(results);

        let schemas: Vec<SchemaData> = results
            .iter()
            .map(|result| {
                let (change_level, change_level_class) = match result.change_level {
                    ChangeLevel::Breaking => ("Breaking".to_string(), "breaking".to_string()),
                    ChangeLevel::Warning => ("Warning".to_string(), "warning".to_string()),
                    ChangeLevel::Change => ("Change".to_string(), "change".to_string()),
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
                total_changes: breaking_count + warning_count + change_count,
                breaking_changes: breaking_count,
                warnings: warning_count,
                non_breaking_changes: change_count,
            },
            schemas,
            routes: vec![], // No routes in old method
            grouped_changes,
            full_schemas: vec![], // No full schemas in old method
        }
    }

    fn group_repeating_changes(&self, results: &[MatchResult]) -> Vec<GroupedChange> {
        self.group_repeating_changes_with_route_infos(results, &[])
    }

    fn group_repeating_changes_with_route_infos(&self, results: &[MatchResult], route_infos: &[RouteInfo]) -> Vec<GroupedChange> {
        let mut change_map: HashMap<String, (DifferenceData, Vec<String>, bool)> = HashMap::new();
        let mut route_schema_map: HashMap<String, Vec<String>> = HashMap::new(); // schema_name -> routes using it
        let mut route_schema_usage_map: HashMap<String, Vec<RouteSchemaUsage>> = HashMap::new(); // schema_name -> detailed usage info

        // Build comprehensive schema-to-routes mapping from route_infos
        for route_info in route_infos {
            let route_name = format!("{} {}", route_info.method.to_uppercase(), route_info.path);
            
            // Add request schemas
            for schema_ref in &route_info.request_schemas {
                route_schema_map.entry(schema_ref.schema_name.clone()).or_insert_with(Vec::new).push(route_name.clone());
                route_schema_usage_map.entry(schema_ref.schema_name.clone()).or_insert_with(Vec::new).push(RouteSchemaUsage {
                    route_name: route_name.clone(),
                    usage_type: "input".to_string(),
                    emoji: "⬇️".to_string(),
                });
            }
            
            // Add response schemas
            for schema_ref in &route_info.response_schemas {
                route_schema_map.entry(schema_ref.schema_name.clone()).or_insert_with(Vec::new).push(route_name.clone());
                route_schema_usage_map.entry(schema_ref.schema_name.clone()).or_insert_with(Vec::new).push(RouteSchemaUsage {
                    route_name: route_name.clone(),
                    usage_type: "output".to_string(),
                    emoji: "⬆️".to_string(),
                });
            }
        }

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
                
                // Check if this is a route schema violation (RequestSchemaViolation or ResponseSchemaViolation)
                let is_route_schema_violation = violation.name() == "RequestSchemaViolation" || violation.name() == "ResponseSchemaViolation";
                
                if is_route_schema_violation {
                    // Extract schema name from route schema violation description
                    // Format: "Request schema 'SchemaName' (content-type) - original_description"
                    // or "Response schema 'SchemaName' (content-type) for status 200 - original_description"
                    let description = violation.description();
                    if let Some(schema_name) = self.extract_schema_name_from_route_violation(&description) {
                        route_schema_map.entry(schema_name).or_insert_with(Vec::new).push(result.name.clone());
                        
                        // Skip adding this to change_map as it will be handled by the schema change
                        continue;
                    }
                }
                
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

        // Separate multi-occurrence and single-occurrence changes
        let mut multi_occurrence: Vec<GroupedChange> = Vec::new();
        let mut single_occurrence: HashMap<String, Vec<(DifferenceData, bool)>> = HashMap::new();

        for (key, (diff, mut schema_names, is_route)) in change_map {
            schema_names.sort();
            schema_names.dedup();

            if schema_names.len() > 1 {
                // Multiple schemas - group by change type
                // Collect all routes that use any of these schemas
                let mut all_route_names = Vec::new();
                let mut all_route_usage = Vec::new();
                for schema_name in &schema_names {
                    if let Some(routes) = route_schema_map.get(schema_name) {
                        all_route_names.extend(routes.clone());
                    }
                    if let Some(usage) = route_schema_usage_map.get(schema_name) {
                        all_route_usage.extend(usage.clone());
                    }
                }
                all_route_names.sort();
                all_route_names.dedup();
                all_route_usage.sort_by(|a, b| a.route_name.cmp(&b.route_name));
                all_route_usage.dedup_by(|a, b| a.route_name == b.route_name);
                
                multi_occurrence.push(GroupedChange {
                    change_key: key,
                    emoji: diff.emoji,
                    description: diff.description,
                    change_level: diff.change_level.clone(),
                    change_level_class: diff.change_level_class.clone(),
                    details: diff.details,
                    schema_names,
                    is_route_change: is_route,
                    is_schema_grouped: false,
                    changes: vec![],
                    schema_name: None,
                    route_names: all_route_names,
                    route_schema_usage: all_route_usage,
                });
            } else if schema_names.len() == 1 {
                // Single schema - collect all changes for this schema
                let schema_name = schema_names[0].clone();
                single_occurrence
                    .entry(schema_name)
                    .or_insert_with(Vec::new)
                    .push((diff, is_route));
            }
        }

        // Group single-occurrence changes by schema
        for (schema_name, changes) in single_occurrence {
            // Determine overall change level (highest severity)
            let overall_level = changes
                .iter()
                .map(|(diff, _)| &diff.change_level_class)
                .min_by_key(|class| match class.as_str() {
                    "breaking" => 0,
                    "warning" => 1,
                    "change" => 2,
                    _ => 3,
                })
                .cloned()
                .unwrap_or_else(|| "change".to_string());

            let overall_level_str = match overall_level.as_str() {
                "breaking" => "Breaking",
                "warning" => "Warning",
                _ => "Change",
            };

            let is_route = changes.first().map(|(_, ir)| *ir).unwrap_or(false);

            // Create list of individual changes
            let change_items: Vec<ChangeItem> = changes
                .iter()
                .map(|(diff, _)| ChangeItem {
                    emoji: diff.emoji.clone(),
                    description: diff.description.clone(),
                    change_level: diff.change_level.clone(),
                    change_level_class: diff.change_level_class.clone(),
                })
                .collect();

            // Get routes that use this schema (if any)
            let route_names = route_schema_map.get(&schema_name).cloned().unwrap_or_default();
            let route_schema_usage = route_schema_usage_map.get(&schema_name).cloned().unwrap_or_default();

            multi_occurrence.push(GroupedChange {
                change_key: schema_name.clone(),
                emoji: if is_route { "🛣️" } else { "🔧" }.to_string(),
                description: schema_name.clone(),
                change_level: overall_level_str.to_string(),
                change_level_class: overall_level,
                details: vec![],
                schema_names: vec![schema_name.clone()],
                is_route_change: is_route,
                is_schema_grouped: true,
                changes: change_items,
                schema_name: Some(schema_name.clone()),
                route_names,
                route_schema_usage,
            });
        }

        // Sort: multi-occurrence first (by count), then single-occurrence (by change level)
        multi_occurrence.sort_by(|a, b| {
            match (a.is_schema_grouped, b.is_schema_grouped) {
                (false, true) => std::cmp::Ordering::Less, // Multi-occurrence first
                (true, false) => std::cmp::Ordering::Greater, // Single-occurrence later
                _ => {
                    // Within same group, sort by count or level
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
                }
            }
        });

        multi_occurrence
    }

    fn extract_schema_name_from_route_violation(&self, description: &str) -> Option<String> {
        // Extract schema name from route schema violation description
        // Format: "Request schema 'SchemaName' (content-type) - original_description"
        // or "Response schema 'SchemaName' (content-type) for status 200 - original_description"
        
        if description.starts_with("Request schema '") {
            if let Some(start) = description.find("'") {
                if let Some(end) = description[start + 1..].find("'") {
                    return Some(description[start + 1..start + 1 + end].to_string());
                }
            }
        } else if description.starts_with("Response schema '") {
            if let Some(start) = description.find("'") {
                if let Some(end) = description[start + 1..].find("'") {
                    return Some(description[start + 1..start + 1 + end].to_string());
                }
            }
        }
        
        None
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
            "RequestSchemaViolation" => ("📋", vec![]),
            "ResponseSchemaViolation" => ("📋", vec![]),
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
