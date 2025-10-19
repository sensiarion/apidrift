use crate::rules::Rule;
use crate::ChangeLevel;
use oas3::spec::ObjectSchema;
use std::collections::HashSet;

/// Trait for schema-specific rules with internal detection logic
pub trait SchemaRule: Rule {
    /// Detect if this rule applies and return instances if detected
    /// Returns empty vector if rule doesn't apply
    fn detect(
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self>
    where
        Self: Sized;
}

/// Schema was added (only in new version)
#[derive(Debug, Clone)]
pub struct SchemaAddedRule {
    pub schema_name: String,
}

impl Rule for SchemaAddedRule {
    fn name(&self) -> &str {
        "SchemaAdded"
    }

    fn description(&self) -> String {
        format!("Schema '{}' was added", self.schema_name)
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Change
    }

    fn context(&self) -> String {
        format!("schema: {}", self.schema_name)
    }
}

impl SchemaRule for SchemaAddedRule {
    fn detect(
        schema_name: &str,
        _property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (None, Some(_)) => vec![SchemaAddedRule {
                schema_name: schema_name.to_string(),
            }],
            _ => vec![],
        }
    }
}

/// Schema was removed (only in old version)
#[derive(Debug, Clone)]
pub struct SchemaRemovedRule {
    pub schema_name: String,
}

impl Rule for SchemaRemovedRule {
    fn name(&self) -> &str {
        "SchemaRemoved"
    }

    fn description(&self) -> String {
        format!("Schema '{}' was removed", self.schema_name)
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Breaking
    }

    fn context(&self) -> String {
        format!("schema: {}", self.schema_name)
    }
}

impl SchemaRule for SchemaRemovedRule {
    fn detect(
        schema_name: &str,
        _property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(_), None) => vec![SchemaRemovedRule {
                schema_name: schema_name.to_string(),
            }],
            _ => vec![],
        }
    }
}

/// Schema type changed
#[derive(Debug, Clone)]
pub struct TypeChangedRule {
    pub schema_name: String,
    pub property_path: String,
    pub old_type: String,
    pub new_type: String,
}

impl Rule for TypeChangedRule {
    fn name(&self) -> &str {
        "TypeChanged"
    }

    fn description(&self) -> String {
        format!(
            "Type changed from '{}' to '{}'",
            self.old_type, self.new_type
        )
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Breaking
    }

    fn context(&self) -> String {
        if self.property_path.is_empty() {
            format!("schema: {}", self.schema_name)
        } else {
            format!(
                "schema: {}, property: {}",
                self.schema_name, self.property_path
            )
        }
    }
}

impl SchemaRule for TypeChangedRule {
    fn detect(
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_schema), Some(current_schema)) => {
                if base_schema.schema_type != current_schema.schema_type {
                    vec![TypeChangedRule {
                        schema_name: schema_name.to_string(),
                        property_path: property_path.to_string(),
                        old_type: format!("{:?}", base_schema.schema_type),
                        new_type: format!("{:?}", current_schema.schema_type),
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

/// Base rule for property changes
#[derive(Debug, Clone)]
pub struct PropertyAddedRule {
    pub schema_name: String,
    pub property_path: String,
    pub property_name: String,
}

impl Rule for PropertyAddedRule {
    fn name(&self) -> &str {
        "PropertyAdded"
    }

    fn description(&self) -> String {
        format!("Property '{}' was added", self.property_name)
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Change
    }

    fn context(&self) -> String {
        if self.property_path.is_empty() {
            format!(
                "schema: {}, property: {}",
                self.schema_name, self.property_name
            )
        } else {
            format!(
                "schema: {}, property: {}.{}",
                self.schema_name, self.property_path, self.property_name
            )
        }
    }
}

impl SchemaRule for PropertyAddedRule {
    fn detect(
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_schema), Some(current_schema)) => {
                let base_props: HashSet<_> = base_schema.properties.keys().collect();
                let current_props: HashSet<_> = current_schema.properties.keys().collect();

                current_props
                    .difference(&base_props)
                    .map(|prop_name| PropertyAddedRule {
                        schema_name: schema_name.to_string(),
                        property_path: property_path.to_string(),
                        property_name: (*prop_name).clone(),
                    })
                    .collect()
            }
            _ => vec![],
        }
    }
}

/// Property was removed (handles both required and optional properties)
#[derive(Debug, Clone)]
pub struct PropertyRemovedRule {
    pub schema_name: String,
    pub property_path: String,
    pub property_name: String,
    pub was_required: bool,
    pub totally_removed: bool, // true if removed entirely, false if just made optional
}

impl Rule for PropertyRemovedRule {
    fn name(&self) -> &str {
        if self.was_required {
            "RequiredPropertyRemoved"
        } else {
            "PropertyRemoved"
        }
    }

    fn description(&self) -> String {
        if self.was_required {
            format!("Required property '{}' was removed", self.property_name)
        } else {
            format!("Property '{}' was removed", self.property_name)
        }
    }

    fn change_level(&self) -> ChangeLevel {
        if self.totally_removed {
            // Property was completely removed - breaking change
            ChangeLevel::Breaking
        } else {
            // Property was just made optional (from required to non-required) - non-breaking
            ChangeLevel::Change
        }
    }

    fn context(&self) -> String {
        if self.property_path.is_empty() {
            if self.was_required {
                format!(
                    "schema: {}, required: {}",
                    self.schema_name, self.property_name
                )
            } else {
                format!(
                    "schema: {}, property: {}",
                    self.schema_name, self.property_name
                )
            }
        } else {
            if self.was_required {
                format!(
                    "schema: {}, property: {}, required: {}",
                    self.schema_name, self.property_path, self.property_name
                )
            } else {
                format!(
                    "schema: {}, property: {}.{}",
                    self.schema_name, self.property_path, self.property_name
                )
            }
        }
    }
}

impl SchemaRule for PropertyRemovedRule {
    fn detect(
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_schema), Some(current_schema)) => {
                let base_props: HashSet<_> = base_schema.properties.keys().collect();
                let current_props: HashSet<_> = current_schema.properties.keys().collect();
                let base_required: HashSet<_> = base_schema.required.iter().collect();

                // Only detect properties that are completely removed from the properties map
                // Properties that are just removed from required (but still exist) are handled separately in matcher.rs
                base_props
                    .difference(&current_props)
                    .map(|prop_name| {
                        // Verify property is completely removed from schema
                        let is_totally_removed =
                            !current_schema.properties.contains_key(*prop_name);

                        PropertyRemovedRule {
                            schema_name: schema_name.to_string(),
                            property_path: property_path.to_string(),
                            property_name: (*prop_name).clone(),
                            was_required: base_required.contains(prop_name),
                            totally_removed: is_totally_removed,
                        }
                    })
                    .collect()
            }
            _ => vec![],
        }
    }
}

/// Required property was added (breaking change - clients must provide it)
#[derive(Debug, Clone)]
pub struct RequiredPropertyAddedRule {
    pub schema_name: String,
    pub property_path: String,
    pub property_name: String,
}

impl Rule for RequiredPropertyAddedRule {
    fn name(&self) -> &str {
        "RequiredPropertyAdded"
    }

    fn description(&self) -> String {
        format!("Required property '{}' was added", self.property_name)
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Breaking
    }

    fn context(&self) -> String {
        if self.property_path.is_empty() {
            format!(
                "schema: {}, required: {}",
                self.schema_name, self.property_name
            )
        } else {
            format!(
                "schema: {}, property: {}, required: {}",
                self.schema_name, self.property_path, self.property_name
            )
        }
    }
}

impl SchemaRule for RequiredPropertyAddedRule {
    fn detect(
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_schema), Some(current_schema)) => {
                let base_required: HashSet<_> = base_schema.required.iter().collect();
                let current_required: HashSet<_> = current_schema.required.iter().collect();

                current_required
                    .difference(&base_required)
                    .map(|prop_name| RequiredPropertyAddedRule {
                        schema_name: schema_name.to_string(),
                        property_path: property_path.to_string(),
                        property_name: (*prop_name).clone(),
                    })
                    .collect()
            }
            _ => vec![],
        }
    }
}

/// Description changed
#[derive(Debug, Clone)]
pub struct DescriptionChangedRule {
    pub schema_name: String,
    pub property_path: String,
    pub old_description: Option<String>,
    pub new_description: Option<String>,
}

impl Rule for DescriptionChangedRule {
    fn name(&self) -> &str {
        "DescriptionChanged"
    }

    fn description(&self) -> String {
        format!(
            "Description changed from '{}' to '{}'",
            self.old_description.as_deref().unwrap_or("(none)"),
            self.new_description.as_deref().unwrap_or("(none)")
        )
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Change
    }

    fn context(&self) -> String {
        if self.property_path.is_empty() {
            format!("schema: {}", self.schema_name)
        } else {
            format!(
                "schema: {}, property: {}",
                self.schema_name, self.property_path
            )
        }
    }
}

impl SchemaRule for DescriptionChangedRule {
    fn detect(
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_schema), Some(current_schema)) => {
                if base_schema.description != current_schema.description {
                    vec![DescriptionChangedRule {
                        schema_name: schema_name.to_string(),
                        property_path: property_path.to_string(),
                        old_description: base_schema.description.clone(),
                        new_description: current_schema.description.clone(),
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

/// Enum values added
#[derive(Debug, Clone)]
pub struct EnumValuesAddedRule {
    pub schema_name: String,
    pub property_path: String,
    pub values: Vec<serde_json::Value>,
}

impl Rule for EnumValuesAddedRule {
    fn name(&self) -> &str {
        "EnumValuesAdded"
    }

    fn description(&self) -> String {
        let values_str = self
            .values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("Enum values added: [{}]", values_str)
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Change
    }

    fn context(&self) -> String {
        if self.property_path.is_empty() {
            format!("schema: {}", self.schema_name)
        } else {
            format!(
                "schema: {}, property: {}",
                self.schema_name, self.property_path
            )
        }
    }
}

impl SchemaRule for EnumValuesAddedRule {
    fn detect(
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_schema), Some(current_schema)) => {
                let base_values: HashSet<_> = base_schema.enum_values.iter().collect();
                let current_values: HashSet<_> = current_schema.enum_values.iter().collect();

                let added_values: Vec<_> = current_values
                    .difference(&base_values)
                    .map(|v| (*v).clone())
                    .collect();

                if !added_values.is_empty() {
                    vec![EnumValuesAddedRule {
                        schema_name: schema_name.to_string(),
                        property_path: property_path.to_string(),
                        values: added_values,
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

/// Enum values removed
#[derive(Debug, Clone)]
pub struct EnumValuesRemovedRule {
    pub schema_name: String,
    pub property_path: String,
    pub values: Vec<serde_json::Value>,
}

impl Rule for EnumValuesRemovedRule {
    fn name(&self) -> &str {
        "EnumValuesRemoved"
    }

    fn description(&self) -> String {
        let values_str = self
            .values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("Enum values removed: [{}]", values_str)
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Breaking
    }

    fn context(&self) -> String {
        if self.property_path.is_empty() {
            format!("schema: {}", self.schema_name)
        } else {
            format!(
                "schema: {}, property: {}",
                self.schema_name, self.property_path
            )
        }
    }
}

impl SchemaRule for EnumValuesRemovedRule {
    fn detect(
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_schema), Some(current_schema)) => {
                let base_values: HashSet<_> = base_schema.enum_values.iter().collect();
                let current_values: HashSet<_> = current_schema.enum_values.iter().collect();

                let removed_values: Vec<_> = base_values
                    .difference(&current_values)
                    .map(|v| (*v).clone())
                    .collect();

                if !removed_values.is_empty() {
                    vec![EnumValuesRemovedRule {
                        schema_name: schema_name.to_string(),
                        property_path: property_path.to_string(),
                        values: removed_values,
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

/// Format changed
#[derive(Debug, Clone)]
pub struct FormatChangedRule {
    pub schema_name: String,
    pub property_path: String,
    pub old_format: Option<String>,
    pub new_format: Option<String>,
}

impl Rule for FormatChangedRule {
    fn name(&self) -> &str {
        "FormatChanged"
    }

    fn description(&self) -> String {
        format!(
            "Format changed from '{}' to '{}'",
            self.old_format.as_deref().unwrap_or("(none)"),
            self.new_format.as_deref().unwrap_or("(none)")
        )
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Warning
    }

    fn context(&self) -> String {
        if self.property_path.is_empty() {
            format!("schema: {}", self.schema_name)
        } else {
            format!(
                "schema: {}, property: {}",
                self.schema_name, self.property_path
            )
        }
    }
}

impl SchemaRule for FormatChangedRule {
    fn detect(
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_schema), Some(current_schema)) => {
                if base_schema.format != current_schema.format {
                    vec![FormatChangedRule {
                        schema_name: schema_name.to_string(),
                        property_path: property_path.to_string(),
                        old_format: base_schema.format.clone(),
                        new_format: current_schema.format.clone(),
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

/// Nullable changed
#[derive(Debug, Clone)]
pub struct NullableChangedRule {
    pub schema_name: String,
    pub property_path: String,
    pub old_nullable: bool,
    pub new_nullable: bool,
}

impl Rule for NullableChangedRule {
    fn name(&self) -> &str {
        "NullableChanged"
    }

    fn description(&self) -> String {
        format!(
            "Nullable changed from {} to {}",
            self.old_nullable, self.new_nullable
        )
    }

    fn change_level(&self) -> ChangeLevel {
        match (self.old_nullable, self.new_nullable) {
            (true, false) => ChangeLevel::Breaking, // Was nullable, now required
            (false, true) => ChangeLevel::Warning,  // Was required, now nullable
            _ => ChangeLevel::Change,               // Both true or both false (shouldn't happen)
        }
    }

    fn context(&self) -> String {
        if self.property_path.is_empty() {
            format!("schema: {}", self.schema_name)
        } else {
            format!(
                "schema: {}, property: {}",
                self.schema_name, self.property_path
            )
        }
    }
}

impl SchemaRule for NullableChangedRule {
    fn detect(
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_schema), Some(current_schema)) => {
                let base_nullable = base_schema.is_nullable().unwrap_or(false);
                let current_nullable = current_schema.is_nullable().unwrap_or(false);

                if base_nullable != current_nullable {
                    vec![NullableChangedRule {
                        schema_name: schema_name.to_string(),
                        property_path: property_path.to_string(),
                        old_nullable: base_nullable,
                        new_nullable: current_nullable,
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

/// Array items schema changed
#[derive(Debug, Clone)]
pub struct ArrayItemsChangedRule {
    pub schema_name: String,
    pub property_path: String,
    pub change_description: String,
}

impl Rule for ArrayItemsChangedRule {
    fn name(&self) -> &str {
        "ArrayItemsChanged"
    }

    fn description(&self) -> String {
        format!("Array items changed: {}", self.change_description)
    }

    fn change_level(&self) -> ChangeLevel {
        // This depends on the specific change, but generally conservative
        ChangeLevel::Warning
    }

    fn context(&self) -> String {
        if self.property_path.is_empty() {
            format!("schema: {}", self.schema_name)
        } else {
            format!(
                "schema: {}, property: {}",
                self.schema_name, self.property_path
            )
        }
    }
}

impl SchemaRule for ArrayItemsChangedRule {
    fn detect(
        _schema_name: &str,
        _property_path: &str,
        _base: Option<&ObjectSchema>,
        _current: Option<&ObjectSchema>,
    ) -> Vec<Self> {
        // Array items detection is complex and requires looking at the items field
        // which is of type Schema enum in the oas3 library
        // For now, return empty - this will be handled by the matcher directly
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oas3::spec::{SchemaType, SchemaTypeSet};
    use std::collections::BTreeMap;

    fn create_test_schema(schema_type: Option<SchemaTypeSet>) -> ObjectSchema {
        ObjectSchema {
            schema_type: schema_type,
            properties: BTreeMap::new(),
            required: vec![],
            description: None,
            enum_values: vec![],
            format: None,
            ..Default::default()
        }
    }

    fn create_nullable_schema(nullable: bool) -> ObjectSchema {
        let schema = if nullable {
            create_test_schema(Some(SchemaTypeSet::Single(SchemaType::Null)))
        } else {
            create_test_schema(Some(SchemaTypeSet::Single(SchemaType::Object)))
        };
        schema
    }

    #[test]
    fn test_schema_added_rule_detection() {
        // Test detection when schema is added (base is None, current is Some)
        let current = create_test_schema(None);
        let detected = SchemaAddedRule::detect("User", "", None, Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "SchemaAdded");
        assert_eq!(detected[0].change_level(), ChangeLevel::Change);
        assert!(detected[0].description().contains("User"));
    }

    #[test]
    fn test_schema_added_rule_no_detection() {
        // Should not detect when both schemas exist
        let base = create_test_schema(None);
        let current = create_test_schema(None);
        let detected = SchemaAddedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 0);
    }

    #[test]
    fn test_schema_removed_rule_detection() {
        // Test detection when schema is removed (base is Some, current is None)
        let base = create_test_schema(None);
        let detected = SchemaRemovedRule::detect("User", "", Some(&base), None);

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "SchemaRemoved");
        assert_eq!(detected[0].change_level(), ChangeLevel::Breaking);
        assert!(detected[0].description().contains("User"));
    }

    #[test]
    fn test_schema_removed_rule_no_detection() {
        // Should not detect when both schemas exist
        let base = create_test_schema(None);
        let current = create_test_schema(None);
        let detected = SchemaRemovedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 0);
    }

    #[test]
    fn test_type_changed_rule_detection() {
        let base = create_test_schema(Some(SchemaTypeSet::Single(SchemaType::String)));
        let current = create_test_schema(Some(SchemaTypeSet::Single(SchemaType::Number)));
        let detected = TypeChangedRule::detect("User", "email", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "TypeChanged");
        assert_eq!(detected[0].change_level(), ChangeLevel::Breaking);
        assert!(detected[0].description().contains("String"));
        assert!(detected[0].description().contains("Number"));
    }

    #[test]
    fn test_type_changed_rule_no_detection() {
        let base = create_test_schema(Some(SchemaTypeSet::Single(SchemaType::String)));
        let current = create_test_schema(Some(SchemaTypeSet::Single(SchemaType::String)));
        let detected = TypeChangedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 0);
    }

    #[test]
    fn test_property_added_rule_detection() {
        let base = create_test_schema(None);
        let mut current = create_test_schema(None);

        // Add a property to current that doesn't exist in base
        current.properties.insert(
            "email".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::String,
            )))),
        );

        let detected = PropertyAddedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "PropertyAdded");
        assert_eq!(detected[0].property_name, "email");
        assert_eq!(detected[0].change_level(), ChangeLevel::Change);
    }

    #[test]
    fn test_property_removed_rule_detection_optional() {
        let mut base = create_test_schema(None);
        let current = create_test_schema(None);

        // Add a property to base that doesn't exist in current (and it's not required)
        base.properties.insert(
            "email".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::String,
            )))),
        );

        let detected = PropertyRemovedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "PropertyRemoved");
        assert_eq!(detected[0].property_name, "email");
        assert_eq!(detected[0].was_required, false);
        assert_eq!(detected[0].totally_removed, true); // Property is completely removed
        assert_eq!(detected[0].change_level(), ChangeLevel::Breaking);
    }

    #[test]
    fn test_property_removed_rule_detection_required() {
        let mut base = create_test_schema(None);
        let current = create_test_schema(None);

        // Add a required property to base that doesn't exist in current
        base.properties.insert(
            "email".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::String,
            )))),
        );
        base.required.push("email".to_string());

        let detected = PropertyRemovedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "RequiredPropertyRemoved");
        assert_eq!(detected[0].property_name, "email");
        assert_eq!(detected[0].was_required, true);
        assert_eq!(detected[0].totally_removed, true); // Property is completely removed
        assert_eq!(detected[0].change_level(), ChangeLevel::Breaking);
    }

    #[test]
    fn test_property_made_optional() {
        // Test when a property goes from required to optional (not removed)
        let rule = PropertyRemovedRule {
            schema_name: "User".to_string(),
            property_path: "".to_string(),
            property_name: "email".to_string(),
            was_required: true,
            totally_removed: false, // Property still exists, just made optional
        };

        assert_eq!(rule.name(), "RequiredPropertyRemoved");
        assert_eq!(rule.change_level(), ChangeLevel::Change); // Should be Change, not Breaking
        assert_eq!(rule.description(), "Required property 'email' was removed");
    }

    #[test]
    fn test_property_still_exists_but_not_required() {
        // Test that a property that still exists in properties but is removed from required
        // is NOT detected by PropertyRemovedRule::detect (handled separately in matcher.rs)
        let mut base = create_test_schema(None);
        let mut current = create_test_schema(None);

        // Add property to both base and current
        base.properties.insert(
            "email".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::String,
            )))),
        );
        current.properties.insert(
            "email".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::String,
            )))),
        );

        // Make it required in base but not in current
        base.required.push("email".to_string());

        // PropertyRemovedRule::detect should NOT detect this because the property still exists
        let detected = PropertyRemovedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(
            detected.len(),
            0,
            "Property still exists, should not be detected as removed"
        );
    }

    #[test]
    fn test_multiple_properties_removed() {
        // Test multiple properties being removed at once
        let mut base = create_test_schema(None);
        let current = create_test_schema(None);

        // Add multiple properties to base
        base.properties.insert(
            "email".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::String,
            )))),
        );
        base.properties.insert(
            "phone".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::String,
            )))),
        );
        base.properties.insert(
            "address".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::String,
            )))),
        );

        // Make email and phone required
        base.required.push("email".to_string());
        base.required.push("phone".to_string());

        let detected = PropertyRemovedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 3, "Should detect all 3 removed properties");

        // Check that all are marked as totally_removed
        for rule in &detected {
            assert_eq!(
                rule.totally_removed, true,
                "All removed properties should be marked as totally_removed"
            );
            assert_eq!(
                rule.change_level(),
                ChangeLevel::Breaking,
                "Removing properties should be breaking"
            );
        }

        // Check specific properties
        let email_rule = detected
            .iter()
            .find(|r| r.property_name == "email")
            .unwrap();
        assert_eq!(email_rule.was_required, true);

        let phone_rule = detected
            .iter()
            .find(|r| r.property_name == "phone")
            .unwrap();
        assert_eq!(phone_rule.was_required, true);

        let address_rule = detected
            .iter()
            .find(|r| r.property_name == "address")
            .unwrap();
        assert_eq!(address_rule.was_required, false);
    }

    #[test]
    fn test_totally_removed_true_breaking_change() {
        // Test that totally_removed = true results in Breaking change level
        let rule = PropertyRemovedRule {
            schema_name: "User".to_string(),
            property_path: "".to_string(),
            property_name: "email".to_string(),
            was_required: false,
            totally_removed: true, // Property completely removed
        };

        assert_eq!(rule.change_level(), ChangeLevel::Breaking);
        assert_eq!(rule.name(), "PropertyRemoved");
    }

    #[test]
    fn test_totally_removed_false_change_level() {
        // Test that totally_removed = false (property made optional) results in Change level
        let rule = PropertyRemovedRule {
            schema_name: "User".to_string(),
            property_path: "".to_string(),
            property_name: "email".to_string(),
            was_required: true,
            totally_removed: false, // Property still exists, just optional now
        };

        assert_eq!(rule.change_level(), ChangeLevel::Change);
        assert_eq!(rule.name(), "RequiredPropertyRemoved");
    }

    #[test]
    fn test_property_removed_with_nested_path() {
        // Test property removal with nested path
        let mut base = create_test_schema(None);
        let current = create_test_schema(None);

        base.properties.insert(
            "nested_field".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::String,
            )))),
        );

        let detected = PropertyRemovedRule::detect("User", "address", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].property_path, "address");
        assert_eq!(detected[0].property_name, "nested_field");
        assert_eq!(detected[0].totally_removed, true);
    }

    #[test]
    fn test_required_property_added_rule_detection() {
        let base = create_test_schema(None);
        let mut current = create_test_schema(None);

        // Add a required property to current
        current.required.push("email".to_string());

        let detected = RequiredPropertyAddedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "RequiredPropertyAdded");
        assert_eq!(detected[0].property_name, "email");
        assert_eq!(detected[0].change_level(), ChangeLevel::Breaking);
    }

    #[test]
    fn test_description_changed_rule_detection() {
        let mut base = create_test_schema(None);
        let mut current = create_test_schema(None);

        base.description = Some("Old description".to_string());
        current.description = Some("New description".to_string());

        let detected = DescriptionChangedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "DescriptionChanged");
        assert!(detected[0].description().contains("Old description"));
        assert!(detected[0].description().contains("New description"));
        assert_eq!(detected[0].change_level(), ChangeLevel::Change);
    }

    #[test]
    fn test_enum_values_added_rule_detection() {
        let mut base = create_test_schema(None);
        let mut current = create_test_schema(None);

        base.enum_values = vec![serde_json::Value::String("active".to_string())];
        current.enum_values = vec![
            serde_json::Value::String("active".to_string()),
            serde_json::Value::String("inactive".to_string()),
        ];

        let detected = EnumValuesAddedRule::detect("Status", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "EnumValuesAdded");
        assert!(detected[0].description().contains("inactive"));
        assert_eq!(detected[0].change_level(), ChangeLevel::Change);
    }

    #[test]
    fn test_enum_values_removed_rule_detection() {
        let mut base = create_test_schema(None);
        let mut current = create_test_schema(None);

        base.enum_values = vec![
            serde_json::Value::String("active".to_string()),
            serde_json::Value::String("pending".to_string()),
        ];
        current.enum_values = vec![serde_json::Value::String("active".to_string())];

        let detected = EnumValuesRemovedRule::detect("Status", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "EnumValuesRemoved");
        assert!(detected[0].description().contains("pending"));
        assert_eq!(detected[0].change_level(), ChangeLevel::Breaking);
    }

    #[test]
    fn test_format_changed_rule_detection() {
        let mut base = create_test_schema(None);
        let mut current = create_test_schema(None);

        base.format = Some("email".to_string());
        current.format = Some("uri".to_string());

        let detected = FormatChangedRule::detect("User", "email", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "FormatChanged");
        assert!(detected[0].description().contains("email"));
        assert!(detected[0].description().contains("uri"));
        assert_eq!(detected[0].change_level(), ChangeLevel::Warning);
    }

    #[test]
    fn test_nullable_changed_rule_detection_breaking() {
        // Create schemas where nullable changes from true to false (breaking change)
        let base = create_nullable_schema(true);
        let current = create_nullable_schema(false);

        let detected = NullableChangedRule::detect("User", "email", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name(), "NullableChanged");
        assert_eq!(detected[0].old_nullable, true);
        assert_eq!(detected[0].new_nullable, false);
        assert_eq!(detected[0].change_level(), ChangeLevel::Breaking);
    }

    #[test]
    fn test_nullable_changed_rule_detection_warning() {
        // Create schemas where nullable changes from false to true (warning)
        let base = create_nullable_schema(false);
        let current = create_nullable_schema(true);

        let detected = NullableChangedRule::detect("User", "email", Some(&base), Some(&current));

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].old_nullable, false);
        assert_eq!(detected[0].new_nullable, true);
        assert_eq!(detected[0].change_level(), ChangeLevel::Warning);
    }

    #[test]
    fn test_multiple_properties_added() {
        let base = create_test_schema(None);
        let mut current = create_test_schema(None);

        // Add multiple properties
        current.properties.insert(
            "email".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::String,
            )))),
        );
        current.properties.insert(
            "age".to_string(),
            oas3::spec::ObjectOrReference::Object(create_test_schema(Some(SchemaTypeSet::Single(
                SchemaType::Number,
            )))),
        );

        let detected = PropertyAddedRule::detect("User", "", Some(&base), Some(&current));

        assert_eq!(detected.len(), 2);
        let prop_names: Vec<_> = detected.iter().map(|r| r.property_name.as_str()).collect();
        assert!(prop_names.contains(&"email"));
        assert!(prop_names.contains(&"age"));
    }
}
