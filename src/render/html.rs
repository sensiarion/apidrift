use crate::matcher::{SchemaMatchResult, SchemaDifference};
use crate::render::Renderer;
use crate::ChangeLevel;
use serde::Serialize;
use std::error::Error;
use tera::{Context, Tera};

#[derive(Serialize)]
struct TemplateData {
    schemas: Vec<SchemaData>,
    stats: Stats,
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
struct DifferenceData {
    emoji: String,
    description: String,
    change_level: String,
    change_level_class: String,
    details: Vec<PropertyCard>,
}

#[derive(Serialize)]
struct PropertyCard {
    emoji: String,
    property_type: String,
    content: String,
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

    fn convert_to_template_data(&self, results: &[SchemaMatchResult]) -> TemplateData {
        let mut breaking_count = 0;
        let mut warning_count = 0;
        let mut change_count = 0;

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

                let differences = self.convert_differences(&result.differences);

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
        }
    }

    fn convert_differences(&self, diffs: &[SchemaDifference]) -> Vec<DifferenceData> {
        diffs
            .iter()
            .map(|diff| self.convert_difference(diff, ""))
            .collect()
    }

    fn convert_difference(&self, diff: &SchemaDifference, prefix: &str) -> DifferenceData {
        let (emoji, description, details) = match diff {
            SchemaDifference::Added => ("➕", "Schema Added", vec![]),
            SchemaDifference::Removed => ("➖", "Schema Removed", vec![]),
            SchemaDifference::TypeChanged { old_type, new_type } => (
                "🔄",
                "Type Changed",
                vec![PropertyCard {
                    emoji: "📝".to_string(),
                    property_type: "Type".to_string(),
                    content: format!("{} → {}", old_type, new_type),
                }],
            ),
            SchemaDifference::RequiredPropertiesAdded { properties } => (
                "⚠️",
                "Required Properties Added",
                properties.iter().map(|p| PropertyCard {
                    emoji: "🔧".to_string(),
                    property_type: "Property".to_string(),
                    content: p.clone(),
                }).collect(),
            ),
            SchemaDifference::RequiredPropertiesRemoved { properties } => (
                "⚠️",
                "Required Properties Removed",
                properties.iter().map(|p| PropertyCard {
                    emoji: "🔧".to_string(),
                    property_type: "Property".to_string(),
                    content: p.clone(),
                }).collect(),
            ),
            SchemaDifference::PropertyAdded { property_name } => (
                "➕",
                "Property Added",
                vec![PropertyCard {
                    emoji: "🔧".to_string(),
                    property_type: "Property".to_string(),
                    content: property_name.clone(),
                }],
            ),
            SchemaDifference::PropertyRemoved { property_name } => (
                "➖",
                "Property Removed",
                vec![PropertyCard {
                    emoji: "🔧".to_string(),
                    property_type: "Property".to_string(),
                    content: property_name.clone(),
                }],
            ),
            SchemaDifference::PropertyModified {
                property_name,
                details,
            } => {
                let nested_prefix = format!("{}.{} - ", prefix, property_name);
                let mut nested_diff = self.convert_difference(details, &nested_prefix);
                nested_diff.description = format!("Property Modified: {}", property_name);
                return nested_diff;
            }
            SchemaDifference::DescriptionChanged {
                old_description,
                new_description,
            } => {
                let old = old_description.as_deref().unwrap_or("(none)");
                let new = new_description.as_deref().unwrap_or("(none)");
                (
                    "📝",
                    "Description Changed",
                    vec![PropertyCard {
                        emoji: "📄".to_string(),
                        property_type: "Description".to_string(),
                        content: format!("{} → {}", old, new),
                    }],
                )
            }
            SchemaDifference::EnumValuesAdded { values } => (
                "➕",
                "Enum Values Added",
                values.iter().map(|v| PropertyCard {
                    emoji: "📋".to_string(),
                    property_type: "Enum".to_string(),
                    content: v.to_string(),
                }).collect(),
            ),
            SchemaDifference::EnumValuesRemoved { values } => (
                "➖",
                "Enum Values Removed",
                values.iter().map(|v| PropertyCard {
                    emoji: "📋".to_string(),
                    property_type: "Enum".to_string(),
                    content: v.to_string(),
                }).collect(),
            ),
            SchemaDifference::FormatChanged {
                old_format,
                new_format,
            } => {
                let old = old_format.as_deref().unwrap_or("(none)");
                let new = new_format.as_deref().unwrap_or("(none)");
                (
                    "🔄",
                    "Format Changed",
                    vec![PropertyCard {
                        emoji: "🏷️".to_string(),
                        property_type: "Format".to_string(),
                        content: format!("{} → {}", old, new),
                    }],
                )
            }
            SchemaDifference::NullableChanged {
                old_nullable,
                new_nullable,
            } => (
                "🔄",
                "Nullable Changed",
                vec![PropertyCard {
                    emoji: "❓".to_string(),
                    property_type: "Nullable".to_string(),
                    content: format!("{} → {}", old_nullable, new_nullable),
                }],
            ),
            SchemaDifference::ArrayItemsChanged { details } => {
                let mut nested_diff = self.convert_difference(details, &format!("{}Items - ", prefix));
                nested_diff.description = "Array Items Changed".to_string();
                return nested_diff;
            }
        };

        let (change_level, change_level_class) = match diff.change_level() {
            ChangeLevel::Breaking => ("Breaking".to_string(), "breaking".to_string()),
            ChangeLevel::Warning => ("Warning".to_string(), "warning".to_string()),
            ChangeLevel::Change => ("Change".to_string(), "change".to_string()),
        };

        DifferenceData {
            emoji: emoji.to_string(),
            description: description.to_string(),
            change_level,
            change_level_class,
            details,
        }
    }
}

impl Renderer for HtmlRenderer {
    fn render(&self, results: &[SchemaMatchResult]) -> Result<String, Box<dyn Error>> {
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

