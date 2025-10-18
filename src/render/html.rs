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
    description: String,
    change_level: String,
    change_level_class: String,
    details: Vec<String>,
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
        let (description, details) = match diff {
            SchemaDifference::Added => ("Schema Added".to_string(), vec![]),
            SchemaDifference::Removed => ("Schema Removed".to_string(), vec![]),
            SchemaDifference::TypeChanged { old_type, new_type } => (
                format!("{}Type Changed", prefix),
                vec![format!("{} → {}", old_type, new_type)],
            ),
            SchemaDifference::RequiredPropertiesAdded { properties } => (
                format!("{}Required Properties Added", prefix),
                properties.iter().map(|p| format!("• {}", p)).collect(),
            ),
            SchemaDifference::RequiredPropertiesRemoved { properties } => (
                format!("{}Required Properties Removed", prefix),
                properties.iter().map(|p| format!("• {}", p)).collect(),
            ),
            SchemaDifference::PropertyAdded { property_name } => (
                format!("{}Property Added: {}", prefix, property_name),
                vec![],
            ),
            SchemaDifference::PropertyRemoved { property_name } => (
                format!("{}Property Removed: {}", prefix, property_name),
                vec![],
            ),
            SchemaDifference::PropertyModified {
                property_name,
                details,
            } => {
                let nested_prefix = format!("{}.{} - ", prefix, property_name);
                let nested_diff = self.convert_difference(details, &nested_prefix);
                return nested_diff;
            }
            SchemaDifference::DescriptionChanged {
                old_description,
                new_description,
            } => {
                let old = old_description.as_deref().unwrap_or("(none)");
                let new = new_description.as_deref().unwrap_or("(none)");
                (
                    format!("{}Description Changed", prefix),
                    vec![format!("{} → {}", old, new)],
                )
            }
            SchemaDifference::EnumValuesAdded { values } => (
                format!("{}Enum Values Added", prefix),
                values.iter().map(|v| format!("• {}", v)).collect(),
            ),
            SchemaDifference::EnumValuesRemoved { values } => (
                format!("{}Enum Values Removed", prefix),
                values.iter().map(|v| format!("• {}", v)).collect(),
            ),
            SchemaDifference::FormatChanged {
                old_format,
                new_format,
            } => {
                let old = old_format.as_deref().unwrap_or("(none)");
                let new = new_format.as_deref().unwrap_or("(none)");
                (
                    format!("{}Format Changed", prefix),
                    vec![format!("{} → {}", old, new)],
                )
            }
            SchemaDifference::NullableChanged {
                old_nullable,
                new_nullable,
            } => (
                format!("{}Nullable Changed", prefix),
                vec![format!("{} → {}", old_nullable, new_nullable)],
            ),
            SchemaDifference::ArrayItemsChanged { details } => {
                let nested_diff = self.convert_difference(details, &format!("{}Items - ", prefix));
                return nested_diff;
            }
        };

        let (change_level, change_level_class) = match diff.change_level() {
            ChangeLevel::Breaking => ("Breaking".to_string(), "breaking".to_string()),
            ChangeLevel::Warning => ("Warning".to_string(), "warning".to_string()),
            ChangeLevel::Change => ("Change".to_string(), "change".to_string()),
        };

        DifferenceData {
            description,
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

