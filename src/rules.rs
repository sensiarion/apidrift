pub mod route;
pub mod schema;

use crate::ChangeLevel;

/// Anchor point for a change in the schema structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeAnchor {
    /// Change at the schema level itself (added/removed schema)
    Schema,
    /// Change at a specific property path (e.g., "address.street")
    Property(String),
    /// Change in a property's type
    PropertyType(String),
    /// Change in required properties list
    Required,
    /// Change in enum values at a property
    EnumValues(String),
    /// Change in property format
    Format(String),
    /// Change in nullable flag
    Nullable(String),
    /// Change in array items
    ArrayItems(String),
    /// Change in description
    Description(String),
    /// Change at route level
    Route,
    /// Change in route parameter
    Parameter(String),
    /// Change in response status
    ResponseStatus(String),
}

impl ChangeAnchor {
    /// Get the property path if this anchor is property-related
    pub fn property_path(&self) -> Option<&str> {
        match self {
            ChangeAnchor::Property(path)
            | ChangeAnchor::PropertyType(path)
            | ChangeAnchor::EnumValues(path)
            | ChangeAnchor::Format(path)
            | ChangeAnchor::Nullable(path)
            | ChangeAnchor::ArrayItems(path)
            | ChangeAnchor::Description(path) => Some(path.as_str()),
            _ => None,
        }
    }

    /// Check if this is a schema-level anchor
    pub fn is_schema_level(&self) -> bool {
        matches!(self, ChangeAnchor::Schema | ChangeAnchor::Required)
    }

    /// Check if this is a property-level anchor
    pub fn is_property_level(&self) -> bool {
        self.property_path().is_some()
    }
}

/// Base trait for all comparison rules
/// Rules detect specific types of differences between API versions
pub trait Rule: std::fmt::Debug {
    /// The name of the rule (e.g., "SchemaRemoved", "TypeChanged")
    fn name(&self) -> &str;

    /// Human-readable description of what this rule detected
    fn description(&self) -> String;

    /// The severity/change level of this rule violation
    fn change_level(&self) -> ChangeLevel;

    /// Typed context/anchor for this change in the schema structure
    fn context(&self) -> ChangeAnchor;

    /// Optional: the category of the rule (schema, endpoint, parameter, etc.)
    fn category(&self) -> RuleCategory {
        RuleCategory::Schema
    }
}

/// Category of rule to support different API aspects
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleCategory {
    Schema,
    Endpoint,
    Parameter,
    Response,
    RequestBody,
}

/// Wrapper for dynamic rule violations
#[derive(Debug)]
pub struct RuleViolation {
    rule: Box<dyn Rule>,
}

impl RuleViolation {
    pub fn new(rule: Box<dyn Rule>) -> Self {
        Self { rule }
    }

    pub fn rule(&self) -> &dyn Rule {
        self.rule.as_ref()
    }

    pub fn name(&self) -> &str {
        self.rule.name()
    }

    pub fn description(&self) -> String {
        self.rule.description()
    }

    pub fn change_level(&self) -> ChangeLevel {
        self.rule.change_level()
    }

    pub fn context(&self) -> ChangeAnchor {
        self.rule.context()
    }

    pub fn category(&self) -> RuleCategory {
        self.rule.category()
    }
}

/// Full schema information with all properties
#[derive(Debug, Clone, serde::Serialize)]
pub struct FullSchemaInfo {
    pub name: String,
    pub description: Option<String>,
    pub properties: Vec<SchemaProperty>,
    pub schema_level_violations: Vec<ViolationInfo>,
    pub change_level: String,
    pub change_level_class: String,
}

/// Full schema property information
#[derive(Debug, Clone, serde::Serialize)]
pub struct SchemaProperty {
    pub name: String,
    pub property_type: Option<String>,
    pub format: Option<String>,
    pub description: Option<String>,
    pub required: bool,
    pub nullable: bool,
    pub enum_values: Vec<serde_json::Value>,
    /// Violations anchored to this property
    pub violations: Vec<ViolationInfo>,
}

/// Light weight violation info for serialization
#[derive(Debug, Clone, serde::Serialize)]
pub struct ViolationInfo {
    pub rule_name: String,
    pub description: String,
    pub change_level: String,
    pub anchor: String, // Debug string of the anchor
}

/// Result of matching with rule violations
#[derive(Debug)]
pub struct MatchResult {
    pub name: String,
    pub violations: Vec<RuleViolation>,
    pub change_level: ChangeLevel,
}

impl MatchResult {
    pub fn new(name: String, violations: Vec<RuleViolation>) -> Self {
        let change_level = calculate_overall_change_level(&violations);
        Self {
            name,
            violations,
            change_level,
        }
    }
}

/// Calculate the overall change level from a list of violations
pub fn calculate_overall_change_level(violations: &[RuleViolation]) -> ChangeLevel {
    let mut has_breaking = false;
    let mut has_warning = false;

    for violation in violations {
        match violation.change_level() {
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
