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
}

pub struct HtmlRenderer {
    tera: Tera,
}

impl HtmlRenderer {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // Load templates from the templates directory
        let tera = Tera::new("templates/**/*.html")?;
        
        Ok(Self { tera })
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

                let differences = result.violations
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
            grouped_changes,
        }
    }

    fn group_repeating_changes(&self, results: &[MatchResult]) -> Vec<GroupedChange> {
        let mut change_map: HashMap<String, (DifferenceData, Vec<String>)> = HashMap::new();

        // Collect all changes and their schemas
        for result in results {
            for violation in &result.violations {
                let diff_data = self.convert_violation(violation);
                let key = self.create_change_key(&diff_data);
                
                let entry = change_map.entry(key).or_insert_with(|| (diff_data.clone(), Vec::new()));
                entry.1.push(result.name.clone());
                
                // If we already have this change, merge the descriptions
                if entry.0.description != diff_data.description {
                    entry.0.description = self.merge_descriptions(&entry.0.description, &diff_data.description);
                }
            }
        }

        // Only group changes that appear in multiple schemas
        let mut grouped: Vec<GroupedChange> = change_map
            .into_iter()
            .filter(|(_, (_, schemas))| schemas.len() > 1)
            .map(|(key, (diff, mut schema_names))| {
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
        } else if desc1.contains("Property Removed") && desc2.contains("Required Properties Removed") {
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
        let change_type = if diff.description.contains("Property Added") || diff.description.contains("Required Properties Added") {
            "property_added".to_string()
        } else if diff.description.contains("Property Removed") || diff.description.contains("Required Properties Removed") {
            "property_removed".to_string()
        } else if diff.description.contains("Property Modified") {
            "property_modified".to_string()
        } else {
            // For other changes, use the full description
            diff.description.replace(" ", "_").to_lowercase()
        };
        
        format!("{}:{}:{}", change_type, diff.change_level_class, properties_key)
    }


    fn convert_violation(&self, violation: &RuleViolation) -> DifferenceData {
        let rule = violation.rule();
        let rule_name = rule.name();
        let description = rule.description();
        
        // Map rule names to emojis and extract details
        let (emoji, details) = match rule_name {
            "SchemaAdded" => ("➕", vec![]),
            "SchemaRemoved" => ("➖", vec![]),
            "TypeChanged" => ("📝", vec![]),
            "RequiredPropertyAdded" => ("⚠️", vec![PropertyCard {
                emoji: "🔧".to_string(),
                property_type: "Required".to_string(),
                content: description.clone(),
            }]),
            "RequiredPropertyRemoved" => ("⚠️", vec![PropertyCard {
                emoji: "🔧".to_string(),
                property_type: "Optional".to_string(),
                content: description.clone(),
            }]),
            "PropertyAdded" => ("🔧", vec![]),
            "PropertyRemoved" => ("🔧", vec![]),
            "DescriptionChanged" => ("📄", vec![]),
            "EnumValuesAdded" => ("➕", vec![]),
            "EnumValuesRemoved" => ("➖", vec![]),
            "FormatChanged" => ("🏷️", vec![]),
            "NullableChanged" => ("❓", vec![]),
            "ArrayItemsChanged" => ("📦", vec![]),
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

