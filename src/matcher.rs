use oas3::spec::{ObjectOrReference, ObjectSchema, Spec};
use std::collections::BTreeMap;

use crate::ChangeLevel;

/// Represents a difference found in a schema
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaDifference {
    /// Schema was added (only in new version)
    Added,
    /// Schema was removed (only in old version)
    Removed,
    /// Schema type changed
    TypeChanged {
        old_type: String,
        new_type: String,
    },
    /// Required properties changed
    RequiredPropertiesAdded {
        properties: Vec<String>,
    },
    RequiredPropertiesRemoved {
        properties: Vec<String>,
    },
    /// Properties changed
    PropertyAdded {
        property_name: String,
    },
    PropertyRemoved {
        property_name: String,
    },
    PropertyModified {
        property_name: String,
        details: Box<SchemaDifference>,
    },
    /// Description changed
    DescriptionChanged {
        old_description: Option<String>,
        new_description: Option<String>,
    },
    /// Enum values changed
    EnumValuesAdded {
        values: Vec<serde_json::Value>,
    },
    EnumValuesRemoved {
        values: Vec<serde_json::Value>,
    },
    /// Format changed
    FormatChanged {
        old_format: Option<String>,
        new_format: Option<String>,
    },
    /// Nullable changed
    NullableChanged {
        old_nullable: bool,
        new_nullable: bool,
    },
    /// Array items changed
    ArrayItemsChanged {
        details: Box<SchemaDifference>,
    },
}

/// Result of matching a schema between two versions
#[derive(Debug, Clone)]
pub struct SchemaMatchResult {
    pub name: String,
    pub differences: Vec<SchemaDifference>,
    pub change_level: ChangeLevel,
}

impl SchemaDifference {
    /// Determine the change level for this difference
    pub fn change_level(&self) -> ChangeLevel {
        match self {
            // Breaking changes
            SchemaDifference::Removed => ChangeLevel::Breaking,
            SchemaDifference::TypeChanged { .. } => ChangeLevel::Breaking,
            SchemaDifference::RequiredPropertiesAdded { .. } => ChangeLevel::Breaking,
            SchemaDifference::PropertyRemoved { .. } => ChangeLevel::Breaking,
            SchemaDifference::EnumValuesRemoved { .. } => ChangeLevel::Breaking,
            SchemaDifference::NullableChanged {
                old_nullable: true,
                new_nullable: false,
            } => ChangeLevel::Breaking,
            SchemaDifference::NullableChanged {
                old_nullable: false,
                new_nullable: false,
            } => ChangeLevel::Change,
            SchemaDifference::NullableChanged {
                old_nullable: true,
                new_nullable: true,
            } => ChangeLevel::Change,

            // Warnings
            SchemaDifference::FormatChanged { .. } => ChangeLevel::Warning,
            SchemaDifference::NullableChanged {
                old_nullable: false,
                new_nullable: true,
            } => ChangeLevel::Warning,

            // Non-breaking changes
            SchemaDifference::Added => ChangeLevel::Change,
            SchemaDifference::RequiredPropertiesRemoved { .. } => ChangeLevel::Change,
            SchemaDifference::PropertyAdded { .. } => ChangeLevel::Change,
            SchemaDifference::DescriptionChanged { .. } => ChangeLevel::Change,
            SchemaDifference::EnumValuesAdded { .. } => ChangeLevel::Change,

            // Recursive cases
            SchemaDifference::PropertyModified { details, .. } => details.change_level(),
            SchemaDifference::ArrayItemsChanged { details } => details.change_level(),
        }
    }
}

/// Schema matcher for comparing OpenAPI schemas between versions
pub struct SchemaMatcher<'a> {
    base_schemas: &'a BTreeMap<String, ObjectOrReference<ObjectSchema>>,
    current_schemas: &'a BTreeMap<String, ObjectOrReference<ObjectSchema>>,
    base_spec: &'a Spec,
    current_spec: &'a Spec,
}

impl<'a> SchemaMatcher<'a> {
    pub fn new(
        base_schemas: &'a BTreeMap<String, ObjectOrReference<ObjectSchema>>,
        current_schemas: &'a BTreeMap<String, ObjectOrReference<ObjectSchema>>,
        base_spec: &'a Spec,
        current_spec: &'a Spec,
    ) -> Self {
        Self {
            base_schemas,
            current_schemas,
            base_spec,
            current_spec,
        }
    }

    /// Match schemas between base and current versions
    pub fn match_schemas(&self) -> Vec<SchemaMatchResult> {
        let mut results = Vec::new();

        // Find all schema names from both versions
        let mut all_schema_names: std::collections::HashSet<String> =
            self.base_schemas.keys().cloned().collect();
        all_schema_names.extend(self.current_schemas.keys().cloned());

        for schema_name in all_schema_names {
            let base_schema = self.base_schemas.get(&schema_name);
            let current_schema = self.current_schemas.get(&schema_name);

            let differences = self.compare_schemas(base_schema, current_schema);

            if !differences.is_empty() {
                // Calculate overall change level (use the highest severity)
                let change_level = self.calculate_overall_change_level(&differences);

                results.push(SchemaMatchResult {
                    name: schema_name,
                    differences,
                    change_level,
                });
            }
        }

        results
    }

    /// Compare two schemas and find differences
    fn compare_schemas(
        &self,
        base: Option<&ObjectOrReference<ObjectSchema>>,
        current: Option<&ObjectOrReference<ObjectSchema>>,
    ) -> Vec<SchemaDifference> {
        match (base, current) {
            (None, None) => vec![],
            (None, Some(_)) => vec![SchemaDifference::Added],
            (Some(_), None) => vec![SchemaDifference::Removed],
            (Some(base_ref), Some(current_ref)) => {
                self.compare_schema_details(base_ref, current_ref)
            }
        }
    }

    /// Resolve a reference to an actual schema
    fn resolve_schema_ref<'b>(
        &self,
        schema_ref: &'b ObjectOrReference<ObjectSchema>,
        spec: &'b Spec,
    ) -> Option<&'b ObjectSchema> {
        match schema_ref {
            ObjectOrReference::Object(obj) => Some(obj),
            ObjectOrReference::Ref { ref_path, .. } => {
                // Parse the reference path (e.g., "#/components/schemas/User")
                if let Some(schema_name) = ref_path.strip_prefix("#/components/schemas/") {
                    // Look up the schema in the spec's components
                    spec.components
                        .as_ref()
                        .and_then(|components| components.schemas.get(schema_name))
                        .and_then(|schema| match schema {
                            ObjectOrReference::Object(obj) => Some(obj),
                            // Handle nested references (though this is rare)
                            ObjectOrReference::Ref { .. } => None,
                        })
                } else {
                    None
                }
            }
        }
    }

    /// Compare detailed schema properties with depth limit to prevent stack overflow
    fn compare_schema_details(&self, base: &ObjectOrReference<ObjectSchema>, current: &ObjectOrReference<ObjectSchema>) -> Vec<SchemaDifference> {
        self.compare_schema_details_with_depth(base, current, 0)
    }

    /// Compare detailed schema properties with recursion depth tracking
    fn compare_schema_details_with_depth(
        &self,
        base: &ObjectOrReference<ObjectSchema>,
        current: &ObjectOrReference<ObjectSchema>,
        depth: usize,
    ) -> Vec<SchemaDifference> {
        const MAX_DEPTH: usize = 10; // Prevent infinite recursion
        let mut differences = Vec::new();

        // Stop recursion if we're too deep
        if depth >= MAX_DEPTH {
            return differences;
        }

        // Resolve references to actual schemas
        let base_schema = match self.resolve_schema_ref(base, self.base_spec) {
            Some(schema) => schema,
            None => return differences, // Skip if we can't resolve the reference
        };

        let current_schema = match self.resolve_schema_ref(current, self.current_spec) {
            Some(schema) => schema,
            None => return differences, // Skip if we can't resolve the reference
        };

        // Compare schema type
        if base_schema.schema_type != current_schema.schema_type {
            differences.push(SchemaDifference::TypeChanged {
                old_type: format!("{:?}", base_schema.schema_type),
                new_type: format!("{:?}", current_schema.schema_type),
            });
        }

        // Compare required properties
        let base_required: std::collections::HashSet<_> =
            base_schema.required.iter().cloned().collect();
        let current_required: std::collections::HashSet<_> =
            current_schema.required.iter().cloned().collect();

        let added_required: Vec<String> = current_required
            .difference(&base_required)
            .cloned()
            .collect();
        let removed_required: Vec<String> = base_required
            .difference(&current_required)
            .cloned()
            .collect();

        if !added_required.is_empty() {
            differences.push(SchemaDifference::RequiredPropertiesAdded {
                properties: added_required,
            });
        }
        if !removed_required.is_empty() {
            differences.push(SchemaDifference::RequiredPropertiesRemoved {
                properties: removed_required,
            });
        }

        // Compare properties
        let base_props = &base_schema.properties;
        let current_props = &current_schema.properties;

        let mut all_prop_names: std::collections::HashSet<String> =
            base_props.keys().cloned().collect();
        all_prop_names.extend(current_props.keys().cloned());

        for prop_name in all_prop_names {
            let base_prop = base_props.get(&prop_name);
            let current_prop = current_props.get(&prop_name);

            match (base_prop, current_prop) {
                (None, Some(_)) => {
                    differences.push(SchemaDifference::PropertyAdded {
                        property_name: prop_name,
                    });
                }
                (Some(_), None) => {
                    differences.push(SchemaDifference::PropertyRemoved {
                        property_name: prop_name.clone(),
                    });
                }
                (Some(base_p), Some(current_p)) => {
                    let prop_diffs = self.compare_schema_details_with_depth(base_p, current_p, depth + 1);
                    if !prop_diffs.is_empty() {
                        for diff in prop_diffs {
                            differences.push(SchemaDifference::PropertyModified {
                                property_name: prop_name.clone(),
                                details: Box::new(diff),
                            });
                        }
                    }
                }
                (None, None) => {}
            }
        }

        // Compare description
        if base_schema.description != current_schema.description {
            differences.push(SchemaDifference::DescriptionChanged {
                old_description: base_schema.description.clone(),
                new_description: current_schema.description.clone(),
            });
        }

        // Compare enum values
        if !base_schema.enum_values.is_empty() || !current_schema.enum_values.is_empty() {
            let base_values: std::collections::HashSet<_> =
                base_schema.enum_values.iter().collect();
            let current_values: std::collections::HashSet<_> =
                current_schema.enum_values.iter().collect();

            let added_values: Vec<_> = current_values
                .difference(&base_values)
                .map(|v| (*v).clone())
                .collect();
            let removed_values: Vec<_> = base_values
                .difference(&current_values)
                .map(|v| (*v).clone())
                .collect();

            if !added_values.is_empty() {
                differences.push(SchemaDifference::EnumValuesAdded {
                    values: added_values,
                });
            }
            if !removed_values.is_empty() {
                differences.push(SchemaDifference::EnumValuesRemoved {
                    values: removed_values,
                });
            }
        }

        // Compare format
        if base_schema.format != current_schema.format {
            differences.push(SchemaDifference::FormatChanged {
                old_format: base_schema.format.clone(),
                new_format: current_schema.format.clone(),
            });
        }

        // Compare nullable
        let base_nullable = base_schema.is_nullable().unwrap_or(false);
        let current_nullable = current_schema.is_nullable().unwrap_or(false);
        if base_nullable != current_nullable {
            differences.push(SchemaDifference::NullableChanged {
                old_nullable: base_nullable,
                new_nullable: current_nullable,
            });
        }

        // Compare array items (for now, skip Schema enum handling)
        // TODO: Implement proper array items comparison

        differences
    }

    /// Calculate the overall change level from a list of differences
    fn calculate_overall_change_level(&self, differences: &Vec<SchemaDifference>) -> ChangeLevel {
        let mut has_breaking = false;
        let mut has_warning = false;

        for diff in differences {
            match diff.change_level() {
                ChangeLevel::Breaking => has_breaking = true,
                ChangeLevel::Warning => has_warning = true,
                ChangeLevel::Change => {}
            }
        }

        if has_breaking {
            ChangeLevel::Breaking
        } else if has_warning {
            ChangeLevel::Warning
        } else {
            ChangeLevel::Change
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_level_detection() {
        // Breaking changes
        assert!(matches!(
            SchemaDifference::Removed.change_level(),
            ChangeLevel::Breaking
        ));
        assert!(matches!(
            SchemaDifference::TypeChanged {
                old_type: String::from("String"),
                new_type: String::from("Number")
            }
            .change_level(),
            ChangeLevel::Breaking
        ));

        // Warnings
        assert!(matches!(
            SchemaDifference::FormatChanged {
                old_format: Some("email".to_string()),
                new_format: Some("uri".to_string())
            }
            .change_level(),
            ChangeLevel::Warning
        ));

        // Non-breaking changes
        assert!(matches!(
            SchemaDifference::Added.change_level(),
            ChangeLevel::Change
        ));
    }
}
