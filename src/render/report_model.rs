use crate::matcher::{RouteInfo, SchemaLocation, SchemaReference};
use crate::rules::{MatchResult, RuleViolation};
use crate::ChangeLevel;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Serialize)]
pub struct DiffReport {
    pub schemas: Vec<SchemaData>,
    pub routes: Vec<RouteData>,
    pub stats: Stats,
    pub grouped_changes: Vec<GroupedChange>,
    pub full_schemas: Vec<crate::rules::FullSchemaInfo>,
}

#[derive(Serialize)]
pub struct Stats {
    pub total_changes: usize,
    pub breaking_changes: usize,
    pub warnings: usize,
    pub non_breaking_changes: usize,
}

#[derive(Serialize)]
pub struct SchemaData {
    pub name: String,
    pub change_level: String,
    pub change_level_class: String,
    pub differences: Vec<DifferenceData>,
}

#[derive(Serialize)]
pub struct RouteData {
    pub name: String,
    pub path: String,
    pub method: String,
    pub change_level: String,
    pub change_level_class: String,
    pub differences: Vec<DifferenceData>,
    pub request_schemas: Vec<SchemaLinkData>,
    pub response_schemas: Vec<SchemaLinkData>,
    pub has_request_schema_changes: bool,
    pub has_response_schema_changes: bool,
}

#[derive(Serialize)]
pub struct SchemaLinkData {
    pub schema_name: String,
    pub content_type: String,
    pub location: String,
    pub status_code: Option<String>,
    pub has_changes: bool,
}

#[derive(Serialize, Clone)]
pub struct DifferenceData {
    pub emoji: String,
    pub description: String,
    pub change_level: String,
    pub change_level_class: String,
    pub details: Vec<PropertyCard>,
}

#[derive(Serialize, Clone)]
pub struct PropertyCard {
    pub emoji: String,
    pub property_type: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct GroupedChange {
    pub change_key: String,
    pub emoji: String,
    pub description: String,
    pub change_level: String,
    pub change_level_class: String,
    pub details: Vec<PropertyCard>,
    pub schema_names: Vec<String>,
    pub is_route_change: bool,
    pub is_schema_grouped: bool,
    pub changes: Vec<ChangeItem>,
    pub schema_name: Option<String>,
    pub route_names: Vec<String>,
    pub route_schema_usage: Vec<RouteSchemaUsage>,
}

#[derive(Serialize, Clone)]
pub struct RouteSchemaUsage {
    pub route_name: String,
    pub usage_type: String,
    pub emoji: String,
}

#[derive(Serialize, Clone)]
pub struct ChangeItem {
    pub emoji: String,
    pub description: String,
    pub change_level: String,
    pub change_level_class: String,
}

impl DiffReport {
    pub fn from_match_results(
        schema_results: &[MatchResult],
        route_results: &[MatchResult],
        route_infos: &[RouteInfo],
        full_schema_infos: &[crate::rules::FullSchemaInfo],
    ) -> Self {
        let mut breaking_count = 0;
        let mut warning_count = 0;
        let mut change_count = 0;

        for result in schema_results.iter().chain(route_results.iter()) {
            for violation in &result.violations {
                match violation.change_level() {
                    ChangeLevel::Breaking => breaking_count += 1,
                    ChangeLevel::Warning => warning_count += 1,
                    ChangeLevel::Change => change_count += 1,
                }
            }
        }

        let mut grouped_changes =
            group_repeating_changes_with_route_infos(schema_results, route_infos);
        let route_grouped = group_repeating_changes_with_route_infos(route_results, route_infos);
        grouped_changes.extend(route_grouped);

        let schemas: Vec<SchemaData> = schema_results
            .iter()
            .map(|result| {
                let (change_level, change_level_class) = change_level_strings(&result.change_level);
                let differences = result.violations.iter().map(convert_violation).collect();
                SchemaData {
                    name: result.name.clone(),
                    change_level,
                    change_level_class,
                    differences,
                }
            })
            .collect();

        let schemas_with_changes: HashSet<String> =
            schema_results.iter().map(|r| r.name.clone()).collect();

        let routes: Vec<RouteData> = route_results
            .iter()
            .map(|result| {
                let (change_level, change_level_class) = change_level_strings(&result.change_level);

                let mut differences = Vec::new();
                let mut has_request_schema_changes = false;
                let mut has_response_schema_changes = false;

                for violation in &result.violations {
                    let diff = convert_violation(violation);
                    let description = &diff.description;
                    if description.contains("Request schema")
                        || description.contains("RequestSchemaViolation")
                    {
                        has_request_schema_changes = true;
                    } else if description.contains("Response schema")
                        || description.contains("ResponseSchemaViolation")
                    {
                        has_response_schema_changes = true;
                    } else {
                        differences.push(diff);
                    }
                }

                let parts: Vec<&str> = result.name.split_whitespace().collect();
                let method = parts.first().copied().unwrap_or("").to_lowercase();
                let path = parts.get(1).copied().unwrap_or("");
                let route_info = route_infos
                    .iter()
                    .find(|r| r.method == method && r.path == *path);

                let (request_schemas, response_schemas) = if let Some(info) = route_info {
                    (
                        convert_schema_references(&info.request_schemas, &schemas_with_changes),
                        convert_schema_references(&info.response_schemas, &schemas_with_changes),
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

        Self {
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

    pub fn from_schema_results(results: &[MatchResult]) -> Self {
        let mut breaking_count = 0;
        let mut warning_count = 0;
        let mut change_count = 0;

        for result in results {
            for violation in &result.violations {
                match violation.change_level() {
                    ChangeLevel::Breaking => breaking_count += 1,
                    ChangeLevel::Warning => warning_count += 1,
                    ChangeLevel::Change => change_count += 1,
                }
            }
        }

        let grouped_changes = group_repeating_changes_with_route_infos(results, &[]);
        let schemas: Vec<SchemaData> = results
            .iter()
            .map(|result| {
                let (change_level, change_level_class) = change_level_strings(&result.change_level);
                let differences = result.violations.iter().map(convert_violation).collect();
                SchemaData {
                    name: result.name.clone(),
                    change_level,
                    change_level_class,
                    differences,
                }
            })
            .collect();

        Self {
            stats: Stats {
                total_changes: breaking_count + warning_count + change_count,
                breaking_changes: breaking_count,
                warnings: warning_count,
                non_breaking_changes: change_count,
            },
            schemas,
            routes: vec![],
            grouped_changes,
            full_schemas: vec![],
        }
    }
}

fn change_level_strings(level: &ChangeLevel) -> (String, String) {
    match level {
        ChangeLevel::Breaking => ("Breaking".to_string(), "breaking".to_string()),
        ChangeLevel::Warning => ("Warning".to_string(), "warning".to_string()),
        ChangeLevel::Change => ("Change".to_string(), "change".to_string()),
    }
}

fn convert_schema_references(
    refs: &[SchemaReference],
    schemas_with_changes: &HashSet<String>,
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

fn group_repeating_changes_with_route_infos(
    results: &[MatchResult],
    route_infos: &[RouteInfo],
) -> Vec<GroupedChange> {
    let mut change_map: HashMap<String, (DifferenceData, Vec<String>, bool)> = HashMap::new();
    let mut route_schema_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut route_schema_usage_map: HashMap<String, Vec<RouteSchemaUsage>> = HashMap::new();

    for route_info in route_infos {
        let route_name = format!("{} {}", route_info.method.to_uppercase(), route_info.path);

        for schema_ref in &route_info.request_schemas {
            route_schema_map
                .entry(schema_ref.schema_name.clone())
                .or_default()
                .push(route_name.clone());
            route_schema_usage_map
                .entry(schema_ref.schema_name.clone())
                .or_default()
                .push(RouteSchemaUsage {
                    route_name: route_name.clone(),
                    usage_type: "input".to_string(),
                    emoji: "⬇️".to_string(),
                });
        }

        for schema_ref in &route_info.response_schemas {
            route_schema_map
                .entry(schema_ref.schema_name.clone())
                .or_default()
                .push(route_name.clone());
            route_schema_usage_map
                .entry(schema_ref.schema_name.clone())
                .or_default()
                .push(RouteSchemaUsage {
                    route_name: route_name.clone(),
                    usage_type: "output".to_string(),
                    emoji: "⬆️".to_string(),
                });
        }
    }

    for result in results {
        let is_route = result.name.starts_with("GET ")
            || result.name.starts_with("POST ")
            || result.name.starts_with("PUT ")
            || result.name.starts_with("DELETE ")
            || result.name.starts_with("PATCH ")
            || result.name.starts_with("HEAD ")
            || result.name.starts_with("OPTIONS ");

        for violation in &result.violations {
            let diff_data = convert_violation(violation);
            let is_route_schema_violation = violation.name() == "RequestSchemaViolation"
                || violation.name() == "ResponseSchemaViolation";

            if is_route_schema_violation {
                let description = violation.description();
                if let Some(schema_name) = extract_schema_name_from_route_violation(&description) {
                    route_schema_map
                        .entry(schema_name)
                        .or_default()
                        .push(result.name.clone());
                    continue;
                }
            }

            let key = create_change_key(&diff_data);
            let entry = change_map
                .entry(key)
                .or_insert_with(|| (diff_data.clone(), Vec::new(), is_route));
            entry.1.push(result.name.clone());

            if entry.0.description != diff_data.description {
                entry.0.description =
                    merge_descriptions(&entry.0.description, &diff_data.description);
            }
        }
    }

    let mut multi_occurrence: Vec<GroupedChange> = Vec::new();
    let mut single_occurrence: HashMap<String, Vec<(DifferenceData, bool)>> = HashMap::new();

    for (key, (diff, mut schema_names, is_route)) in change_map {
        schema_names.sort();
        schema_names.dedup();

        match schema_names.len().cmp(&1) {
            std::cmp::Ordering::Greater => {
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
            }
            std::cmp::Ordering::Equal => {
                let schema_name = schema_names[0].clone();
                single_occurrence
                    .entry(schema_name)
                    .or_default()
                    .push((diff, is_route));
            }
            std::cmp::Ordering::Less => {}
        }
    }

    for (schema_name, changes) in single_occurrence {
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
        let change_items: Vec<ChangeItem> = changes
            .iter()
            .map(|(diff, _)| ChangeItem {
                emoji: diff.emoji.clone(),
                description: diff.description.clone(),
                change_level: diff.change_level.clone(),
                change_level_class: diff.change_level_class.clone(),
            })
            .collect();

        let route_names = route_schema_map
            .get(&schema_name)
            .cloned()
            .unwrap_or_default();
        let route_schema_usage = route_schema_usage_map
            .get(&schema_name)
            .cloned()
            .unwrap_or_default();

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
            schema_name: Some(schema_name),
            route_names,
            route_schema_usage,
        });
    }

    multi_occurrence.sort_by(|a, b| match (a.is_schema_grouped, b.is_schema_grouped) {
        (false, true) => std::cmp::Ordering::Less,
        (true, false) => std::cmp::Ordering::Greater,
        _ => {
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
    });

    multi_occurrence
}

fn extract_schema_name_from_route_violation(description: &str) -> Option<String> {
    if !(description.starts_with("Request schema '")
        || description.starts_with("Response schema '"))
    {
        return None;
    }

    let start = description.find('\'')?;
    let end_rel = description[start + 1..].find('\'')?;
    Some(description[start + 1..start + 1 + end_rel].to_string())
}

fn merge_descriptions(desc1: &str, desc2: &str) -> String {
    if desc1.contains("Property Added") && desc2.contains("Required Properties Added") {
        if let Some(prop1) = desc1.strip_prefix("Property Added: ") {
            if desc2.contains(prop1) {
                return format!("Property Added (Required): {prop1}");
            }
        }
    } else if desc1.contains("Property Removed") && desc2.contains("Required Properties Removed") {
        if let Some(prop1) = desc1.strip_prefix("Property Removed: ") {
            if desc2.contains(prop1) {
                return format!("Property Removed (Required): {prop1}");
            }
        }
    }

    if desc1 == desc2 {
        return desc1.to_string();
    }

    if desc1.len() < desc2.len() {
        desc1.to_string()
    } else {
        desc2.to_string()
    }
}

fn create_change_key(diff: &DifferenceData) -> String {
    let mut property_names = Vec::new();

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

    for detail in &diff.details {
        if detail.property_type == "Required" {
            property_names.push(detail.content.clone());
        }
    }

    property_names.sort();
    let properties_key = property_names.join(",");

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
        diff.description.replace(' ', "_").to_lowercase()
    };

    format!("{change_type}:{}:{properties_key}", diff.change_level_class)
}

fn convert_violation(violation: &RuleViolation) -> DifferenceData {
    let rule = violation.rule();
    let rule_name = rule.name();
    let description = rule.description();

    let (emoji, details) = match rule_name {
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
